use std::f32::consts::{FRAC_2_PI, FRAC_PI_2, PI};

use bevy::{
    log::warn,
    math::{FloatOrd, Isometry2d, Vec2, bounding::Aabb2d, vec2},
    platform::collections::HashSet,
};

pub(crate) fn line_decomposition(vertices: &Vec<Vec2>) -> Option<Vec<Vec<Vec2>>> {
    if vertices.len() < 2 {
        return None;
    }

    let mut res = vec![];

    let mut curr = vec![vertices[0], vertices[1]];
    let mut old_orientation = None;

    let mut angle = 0.0;

    for line in vertices.windows(3) {
        let new_orientation = orientation(line[0], line[1], line[2]);

        let next_angle = angle + (line[0] - line[1]).angle_to(line[2] - line[1]);
        println!("angle: {}", (line[0] - line[1]).angle_to(line[2] - line[1]));

        if (old_orientation.is_none()
            || matches!(new_orientation, Orientation::Touch)
            || Some(new_orientation) == old_orientation)
            && next_angle < FRAC_PI_2
        {
            curr.push(line[2]);
            angle = next_angle;
        } else {
            for i in (1..curr.len() - 1).rev() {
                curr.push(curr[i]);
            }

            res.push(curr);
            curr = vec![line[0], line[1], line[2]];

            angle = next_angle - angle;
        }

        old_orientation = Some(new_orientation);
    }

    if curr.len() > 1 {
        for i in (1..curr.len() - 1).rev() {
            curr.push(curr[i]);
        }

        res.push(curr);
    }

    // for line in vertices.windows(3) {
    //     res.push(vec![line[0], line[1], line[2], line[1]]);
    // }

    Some(res)
}

pub(crate) fn complementary_decomposition(vertices: Vec<Vec2>) -> Option<Vec<Vec<Vec2>>> {
    let mut aabb = Aabb2d::from_point_cloud(Isometry2d::default(), &vertices);
    // aabb.min -= Vec2::splat(f32::EPSILON);
    // aabb.max += Vec2::splat(f32::EPSILON);

    aabb.min -= Vec2::splat(0.5);
    aabb.max += Vec2::splat(0.5);

    let (min_y_index, min_y_point) = vertices
        .iter()
        .enumerate()
        .min_by_key(|point| (FloatOrd(point.1.y), FloatOrd(-point.1.x)))
        .unwrap();

    let mut new_vertices = vec![];

    let mut index = min_y_index;

    loop {
        new_vertices.push(vertices[index]);

        if index == 0 {
            index = vertices.len() - 1;
        } else {
            index -= 1;
        }

        if index == min_y_index {
            break;
        }
    }

    new_vertices.push(*min_y_point);
    new_vertices.push(vec2(aabb.max.x, aabb.min.y));
    new_vertices.push(aabb.min);
    new_vertices.push(vec2(aabb.min.x, aabb.max.y));
    new_vertices.push(aabb.max);
    new_vertices.push(vec2(aabb.max.x, aabb.min.y));

    warn!("{new_vertices:?}");
    convex_decomposition(new_vertices)
}

pub(crate) fn convex_decomposition(vertices: Vec<Vec2>) -> Option<Vec<Vec<Vec2>>> {
    if is_convex(&vertices) {
        return Some(vec![vertices]);
    }

    let mut triangles = triangulate(vertices)?;

    let mut i = 0;
    while i < triangles.len() {
        let mut merged_any = false;

        let mut j = i + 1;
        while j < triangles.len() {
            if let Some(merged) = try_merge(&triangles[i], &triangles[j]) {
                triangles[i] = merged;
                triangles.remove(j);
                merged_any = true;
                break;
            } else {
                j += 1;
            }
        }

        if !merged_any {
            i += 1;
        }
    }

    Some(triangles)
}

fn try_merge(a: &Vec<Vec2>, b: &Vec<Vec2>) -> Option<Vec<Vec2>> {
    for i in 0..a.len() {
        let (a1, a2) = (a[i], a[(i + 1) % a.len()]);
        for j in 0..b.len() {
            let (b1, b2) = (b[j], b[(j + 1) % b.len()]);

            if a1 == b2 && a2 == b1 {
                let mut merged = vec![];

                let mut k = (i + 1) % a.len();
                while k != i {
                    merged.push(a[k]);
                    k = (k + 1) % a.len();
                }

                let mut k = (j + 1) % b.len();
                while k != j {
                    merged.push(b[k]);
                    k = (k + 1) % b.len();
                }

                if is_convex(&merged) {
                    return Some(merged);
                }
            }
        }
    }

    None
}

fn is_convex(vertices: &Vec<Vec2>) -> bool {
    for i in 0..vertices.len() {
        if matches!(
            orientation(
                vertices[i],
                vertices[(i + 1) % vertices.len()],
                vertices[(i + 2) % vertices.len()]
            ),
            Orientation::Left
        ) {
            return false;
        }
    }
    return true;
}

// #[derive(PartialEq, PartialOrd)]
// struct FloatOrd(pub f32);

// // impl PartialOrd for FloatOrd {
// //     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
// //         self.0.partial_cmp(&other.0)
// //     }
// // }

// impl Eq for FloatOrd {}

// impl Ord for FloatOrd {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         self.0.total_cmp(&other.0)
//     }
// }

// impl Hash for FloatOrd {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         if self.0.is_nan() {
//             state.write(&f32::to_ne_bytes(f32::NAN));
//         } else if self.0 == 0.0 {
//             state.write(&f32::to_ne_bytes(0.0f32));
//         } else {
//             state.write(&f32::to_ne_bytes(self.0));
//         }
//     }
// }

fn triangulate(vertices: Vec<Vec2>) -> Option<Vec<Vec<Vec2>>> {
    let mut original_edges = HashSet::new();
    for i in 0..vertices.len() {
        let p1 = vertices[i];
        let p2 = vertices[(i + 1) % vertices.len()];
        original_edges.insert((
            FloatOrd(p1.x),
            FloatOrd(p1.y),
            FloatOrd(p2.x),
            FloatOrd(p2.y),
        ));
    }

    let mut res = vec![];
    let mut indices: Vec<_> = (0..vertices.len()).collect();

    while indices.len() > 3 {
        let n = indices.len();
        let mut found_ear = false;

        'a: for i in 0..n {
            let prev = indices[(i + n - 1) % n];
            let curr = indices[i];
            let next = indices[(i + 1) % n];

            if matches!(
                orientation(vertices[prev], vertices[curr], vertices[next]),
                Orientation::Left
            ) {
                continue;
            }

            for j in &indices {
                if [prev, curr, next].contains(&j) {
                    continue;
                }
                if [vertices[prev], vertices[curr], vertices[next]].contains(&vertices[*j]) {
                    continue;
                }

                if point_in_triangle(
                    vertices[*j],
                    (vertices[prev], vertices[curr], vertices[next]),
                ) {
                    continue 'a;
                }
            }

            found_ear = true;
            res.push(vec![vertices[prev], vertices[curr], vertices[next]]);
            indices.remove(i);
            break;
        }

        if !found_ear {
            warn!("Couldn't triangulate occluder!");
            return None;
        }
    }

    res.push(vec![
        vertices[indices[0]],
        vertices[indices[1]],
        vertices[indices[2]],
    ]);

    Some(res)
}

fn point_in_triangle(p: Vec2, (a, b, c): (Vec2, Vec2, Vec2)) -> bool {
    let ab = orientation(p, a, b);
    let bc = orientation(p, b, c);
    let ca = orientation(p, c, a);

    let a = matches!(ab, Orientation::Right)
        && matches!(bc, Orientation::Right)
        && matches!(ca, Orientation::Right);

    let b = matches!(ab, Orientation::Left)
        && matches!(bc, Orientation::Left)
        && matches!(ca, Orientation::Left);

    a || b
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Orientation {
    Touch,
    Left,
    Right,
}

fn orientation(a: Vec2, b: Vec2, p: Vec2) -> Orientation {
    let res = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
    if res < 0. {
        return Orientation::Right;
    }
    if res > 0. {
        return Orientation::Left;
    }
    Orientation::Touch
}
