use bevy::{
    prelude::*,
    render::{
        render_resource::{GpuArrayBuffer, ShaderType},
        sync_world::SyncToRenderWorld,
    },
};

#[derive(Component, Clone, Default, Reflect)]
#[require(SyncToRenderWorld)]
pub struct Occluder {
    shape: OccluderShape,
}

impl Occluder {
    pub fn shape(&self) -> &OccluderShape {
        &self.shape
    }

    pub fn from_shape(shape: OccluderShape) -> Self {
        Self { shape }
    }

    pub fn polygon(vertices: Vec<Vec2>) -> Option<Self> {
        normalize_vertices(vertices).and_then(|(vertices, concave)| {
            Some(Self::from_shape(OccluderShape::Polygon {
                vertices,
                concave,
            }))
        })
    }

    pub fn polyline(vertices: Vec<Vec2>) -> Option<Self> {
        Some(Self::from_shape(OccluderShape::Polyline {
            vertices,
            concave: true,
        }))
    }

    pub fn rectangle(width: f32, height: f32) -> Self {
        Self::from_shape(OccluderShape::Rectangle { width, height })
    }

    pub fn round_rectangle(width: f32, height: f32, radius: f32) -> Self {
        Self::from_shape(OccluderShape::RoundRectangle {
            width,
            height,
            radius,
        })
    }
}

#[derive(Component, Debug)]
pub(crate) struct ExtractedOccluder {
    pub pos: Vec2,
    pub shape: OccluderShape,
    pub sprite_id: f32,
    pub z: f32,
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
    pub line: u32,
    pub round: u32,
    pub sprite_id: f32,
    pub z: f32,
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformRoundOccluder {
    pub pos: Vec2,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformVertex {
    pub angle: f32,
    pub pos: Vec2,
}

#[derive(Resource, Default)]
pub(crate) struct OccluderSet(
    pub  Vec<(
        GpuArrayBuffer<UniformOccluder>,
        GpuArrayBuffer<UniformVertex>,
        GpuArrayBuffer<UniformRoundOccluder>,
    )>,
);

#[derive(Reflect, Clone, Debug)]
pub enum OccluderShape {
    Rectangle {
        width: f32,
        height: f32,
    },
    Polygon {
        vertices: Vec<Vec2>,
        concave: bool,
    },
    Polyline {
        vertices: Vec<Vec2>,
        concave: bool,
    },
    RoundRectangle {
        width: f32,
        height: f32,
        radius: f32,
    },
}

impl Default for OccluderShape {
    fn default() -> Self {
        Self::Rectangle {
            width: 10.,
            height: 10.,
        }
    }
}

impl OccluderShape {
    pub fn is_round(&self) -> bool {
        matches!(self, OccluderShape::RoundRectangle { .. })
    }

    pub fn is_concave(&self) -> bool {
        match self {
            Self::Polygon { concave, .. } => *concave,
            Self::Polyline { concave, .. } => *concave,
            _ => false,
        }
    }
    pub fn is_line(&self) -> bool {
        matches!(self, OccluderShape::Polyline { .. })
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
            Self::RoundRectangle { .. } => default(),
        }
    }
}

// rotates vertices to be clockwise
fn normalize_vertices(vertices: Vec<Vec2>) -> Option<(Vec<Vec2>, bool)> {
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

    if orientations.contains(&Orientation::Left) && orientations.contains(&Orientation::Right) {
        return Some((vertices.to_vec(), true));
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

pub(crate) fn point_inside_poly(p: Vec2, mut poly: Vec<Vec2>) -> bool {
    // TODO: Bounding Box check

    poly.push(poly[0]);

    let mut inside = false;

    for line in poly.windows(2) {
        if p.y > line[0].y.min(line[1].y)
            && p.y <= line[0].y.max(line[1].y)
            && p.x <= line[0].x.max(line[1].x)
        {
            let x_intersection =
                (p.y - line[0].y) * (line[1].x - line[0].x) / (line[1].y - line[0].y) + line[0].x;

            if line[0].x == line[1].x || p.x <= x_intersection {
                inside = !inside;
            }
        }
    }
    inside
}
