use bevy::{
    prelude::*,
    render::{
        render_resource::{GpuArrayBuffer, ShaderType},
        sync_world::SyncToRenderWorld,
    },
};

#[derive(Component, Reflect)]
#[require(SyncToRenderWorld)]
pub struct Occluder {
    pub shape: OccluderShape,
}

#[derive(Component)]
pub(crate) struct ExtractedOccluder {
    pub pos: Vec2,
    pub shape: OccluderShape,
}

impl ExtractedOccluder {
    pub fn vertices(&self) -> Vec<Vec2> {
        self.shape.vertices(self.pos)
    }
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformOccluder {
    pub n_vertices: u32,
    pub seam: f32,
    pub concave: u32,
    pub closed: u32,
}

#[derive(ShaderType, Clone)]
pub(crate) struct UniformVertex {
    pub angle: f32,
    pub pos: Vec2,
}

#[derive(Resource, Default)]
pub(crate) struct OccluderSet(
    pub  Vec<(
        GpuArrayBuffer<UniformOccluder>,
        GpuArrayBuffer<UniformVertex>,
    )>,
);

#[derive(Reflect, Clone)]
pub struct OccluderShape(OccluderShapeInternal);

#[derive(Reflect, Clone)]
pub enum OccluderShapeInternal {
    Rectangle { width: f32, height: f32 },
    Polygon { vertices: Vec<Vec2>, concave: bool },
    Polyline { vertices: Vec<Vec2>, concave: bool },
}

impl OccluderShapeInternal {
    pub fn concave(&self) -> bool {
        match self {
            Self::Polygon { concave, .. } => *concave,
            Self::Polyline { concave, .. } => *concave,
            _ => false,
        }
    }
    pub fn closed(&self) -> bool {
        !matches!(self, OccluderShapeInternal::Polyline { .. })
    }

    pub(crate) fn vertices(&self, pos: Vec2) -> Vec<Vec2> {
        match &self {
            Self::Rectangle { width, height } => {
                let corner = vec2(width / 2., height / 2.);
                vec![
                    vec2(corner.x, corner.y) + pos,
                    vec2(corner.x, -corner.y) + pos,
                    vec2(-corner.x, -corner.y) + pos,
                    vec2(-corner.x, corner.y) + pos,
                ]
            }
            Self::Polygon { vertices, .. } => vertices.clone(),
            Self::Polyline { vertices, .. } => vertices.clone(),
        }
    }
}

impl OccluderShape {
    pub(crate) fn vertices(&self, pos: Vec2) -> Vec<Vec2> {
        self.0.vertices(pos)
    }

    pub fn is_concave(&self) -> bool {
        self.0.concave()
    }
    pub fn is_closed(&self) -> bool {
        self.0.closed()
    }

    pub fn polygon(vertices: Vec<Vec2>, allow_concave: bool) -> Option<Self> {
        normalize_vertices(vertices, allow_concave, true).and_then(|(vertices, concave)| {
            Some(Self(OccluderShapeInternal::Polygon { vertices, concave }))
        })
    }

    pub fn polyline(vertices: Vec<Vec2>, is_concave: bool) -> Option<Self> {
        if is_concave {
            return Some(Self(OccluderShapeInternal::Polyline {
                vertices,
                concave: true,
            }));
        }

        normalize_vertices(vertices, false, false).and_then(|(vertices, concave)| {
            Some(Self(OccluderShapeInternal::Polyline { vertices, concave }))
        })
    }

    pub fn rectangle(width: f32, height: f32) -> Self {
        Self(OccluderShapeInternal::Rectangle { width, height })
    }

    pub fn internal(&self) -> OccluderShapeInternal {
        self.0.clone()
    }
}

// rotates vertices to be clockwise
fn normalize_vertices(
    mut vertices: Vec<Vec2>,
    allow_concave: bool,
    closed: bool,
) -> Option<(Vec<Vec2>, bool)> {
    if vertices.len() < 1 {
        warn!("Not enough vertices to form shape");
        return None;
    }

    if vertices.len() < 3 {
        return Some((vertices.to_vec(), false));
    }

    let mut orientations: Vec<_> = vertices
        .windows(3)
        .map(|line| orientation(line[0], line[1], line[2]))
        .collect();

    orientations.push(orientation(
        vertices[vertices.len() - 2],
        vertices[vertices.len() - 1],
        vertices[0],
    ));
    orientations.push(orientation(
        vertices[vertices.len() - 1],
        vertices[0],
        vertices[1],
    ));

    if !closed {
        for i in (1..vertices.len() - 1).rev() {
            vertices.push(vertices[i]);
        }
    }

    if orientations.contains(&Orientation::Left) && orientations.contains(&Orientation::Right) {
        if allow_concave {
            return Some((vertices.to_vec(), true));
        }

        match closed {
            true => warn!(
                "Shape is not convex. Set 'allow_concave' to true if you wish to allow this. However, this might have a considerable impact on performance"
            ),
            false => warn!(
                "Line is not convex. Set 'is_convex' to true if you wish to allow this, at the cost of performance"
            ),
        }
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
