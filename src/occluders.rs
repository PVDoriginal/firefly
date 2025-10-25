use bevy::{
    prelude::*,
    render::{
        sync_world::SyncToRenderWorld,
        view::{VisibilityClass, visibility},
    },
};

#[derive(Component, Reflect)]
#[require(SyncToRenderWorld, VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<PointLight>)]
pub struct Occluder {
    shape: OccluderShape,
}

#[derive(Reflect, Clone)]
pub enum OccluderShape {
    Rectangle { width: f32, height: f32 },
    Polygon { vertices: Vec<Vec2>, concave: bool },
}

impl Occluder {
    pub(crate) fn vertices(&self) -> Vec<Vec2> {
        match &self.shape {
            OccluderShape::Rectangle { width, height } => {
                let corner = vec2(width / 2., height / 2.);
                vec![
                    vec2(corner.x, corner.y),
                    vec2(corner.x, -corner.y),
                    vec2(-corner.x, -corner.y),
                    vec2(-corner.x, corner.y),
                ]
            }
            OccluderShape::Polygon { vertices, .. } => vertices.clone(),
        }
    }

    pub fn polygon(vertices: Vec<Vec2>, allow_concave: bool) -> Option<Self> {
        normalize_vertices(&vertices, allow_concave).and_then(|(vertices, concave)| {
            Some(Self {
                shape: OccluderShape::Polygon { vertices, concave },
            })
        })
    }

    pub fn rectangle(width: f32, height: f32) -> Self {
        Self {
            shape: OccluderShape::Rectangle { width, height },
        }
    }

    pub fn shape(&self) -> OccluderShape {
        self.shape.clone()
    }
}

// rotates vertices to be clockwise
fn normalize_vertices(vertices: &Vec<Vec2>, allow_concave: bool) -> Option<(Vec<Vec2>, bool)> {
    if vertices.len() < 1 {
        warn!("Not enough vertices to form shape");
        return None;
    }

    if vertices.len() < 3 {
        return Some((vertices.to_vec(), false));
    }

    let orientations: Vec<_> = vertices
        .windows(3)
        .map(|line| orientation(line[0], line[1], line[2]))
        .collect();

    if orientations.contains(&Orientation::Left) && orientations.contains(&Orientation::Right) {
        if allow_concave {
            return Some((vertices.to_vec(), true));
        }

        warn!(
            "Shape is not convex. Set 'allow_concave' to true if you wish to allow this. However, this might have a considerable impact on performance"
        );
        return None;
    }

    if orientations.contains(&Orientation::Left) {
        return Some((vertices.iter().rev().map(|x| *x).collect(), false));
    }

    Some((vertices.to_vec(), false))
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
