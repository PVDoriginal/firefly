use bevy::{log::warn, math::Vec2};

pub(crate) fn convex_decomposition(vertices: Vec<Vec2>) -> Option<Vec<Vec<Vec2>>> {
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

fn triangulate(vertices: Vec<Vec2>) -> Option<Vec<Vec<Vec2>>> {
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

    let a = matches!(ab, Orientation::Right | Orientation::Touch)
        && matches!(bc, Orientation::Right | Orientation::Touch)
        && matches!(ca, Orientation::Right | Orientation::Touch);

    let b = matches!(ab, Orientation::Left | Orientation::Touch)
        && matches!(bc, Orientation::Left | Orientation::Touch)
        && matches!(ca, Orientation::Left | Orientation::Touch);

    a || b
}

#[derive(PartialEq, Eq)]
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
