use core::f32;

use bevy::{
    color::palettes::css::{BLACK, WHITE},
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
    pub color: Color,
    pub opacity: f32,
    pub ignored_sprites: Vec<Entity>,
}

impl Occluder {
    pub fn shape(&self) -> &OccluderShape {
        &self.shape
    }

    fn from_shape(shape: OccluderShape) -> Self {
        Self {
            shape,
            opacity: 1.,
            color: bevy::prelude::Color::Srgba(WHITE),
            ..default()
        }
    }

    pub fn with_color(&self, color: Color) -> Self {
        let mut res = self.clone();
        res.color = color;
        res
    }

    pub fn with_opacity(&self, opacity: f32) -> Self {
        let mut res = self.clone();
        res.opacity = opacity;
        res
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
        Self::round_rectangle(width, height, 0.)
    }

    pub fn round_rectangle(width: f32, height: f32, radius: f32) -> Self {
        Self::from_shape(OccluderShape::RoundRectangle {
            width,
            height,
            radius,
        })
    }

    pub fn circle(radius: f32) -> Self {
        Self::round_rectangle(0., 0., radius)
    }

    pub fn vertical_capsule(length: f32, radius: f32) -> Self {
        Self::round_rectangle(0., length, radius)
    }

    pub fn horizontal_capsule(length: f32, radius: f32) -> Self {
        Self::round_rectangle(length, 0., radius)
    }

    pub fn capsule(length: f32, radius: f32) -> Self {
        Self::vertical_capsule(length, radius)
    }
}

#[derive(Component, Debug)]
pub(crate) struct ExtractedOccluder {
    pub pos: Vec2,
    pub rot: f32,
    pub shape: OccluderShape,
    pub z: f32,
    pub color: Color,
    pub opacity: f32,
    pub ignored_sprites: Vec<Entity>,
}

impl ExtractedOccluder {
    pub fn vertices(&self) -> Vec<Vec2> {
        self.shape.vertices(self.pos, Rot2::radians(self.rot))
    }
    pub fn rect(&self) -> Rect {
        match self.shape {
            OccluderShape::RoundRectangle {
                width,
                height,
                radius,
            } => Rect {
                min: Vec2::splat(-width.max(height) - radius * 2.) + self.pos,
                max: Vec2::splat(width.max(height) + radius * 2.) + self.pos,
            },
            _ => vertices_rect(self.vertices()),
        }
    }
}

fn vertices_rect(vertices: Vec<Vec2>) -> Rect {
    let mut rect = Rect {
        min: Vec2::splat(f32::MAX),
        max: Vec2::splat(f32::MIN),
    };

    for vertex in vertices {
        rect.min.x = rect.min.x.min(vertex.x);
        rect.max.x = rect.max.x.max(vertex.x);
        rect.min.y = rect.min.y.min(vertex.y);
        rect.max.y = rect.max.y.max(vertex.y);
    }
    rect
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformOccluder {
    pub n_vertices: u32,
    pub seam: f32,
    pub concave: u32,
    pub line: u32,
    pub round: u32,
    pub n_sprites: u32,
    pub z: f32,
    pub color: Vec3,
    pub opacity: f32,
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformRoundOccluder {
    pub pos: Vec2,
    pub rot: f32,
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
        GpuArrayBuffer<f32>,
    )>,
);

#[derive(Reflect, Clone, Debug)]
pub enum OccluderShape {
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
        Self::RoundRectangle {
            width: 10.,
            height: 10.,
            radius: 0.,
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

    pub(crate) fn vertices(&self, pos: Vec2, rot: Rot2) -> Vec<Vec2> {
        match &self {
            Self::Polygon { vertices, .. } => rotate_vertices(vertices.to_vec(), pos, rot),
            Self::Polyline { vertices, .. } => rotate_vertices(vertices.to_vec(), pos, rot),
            Self::RoundRectangle { .. } => default(),
        }
    }
}

pub(crate) fn rotate_vertices(vertices: Vec<Vec2>, pos: Vec2, rot: Rot2) -> Vec<Vec2> {
    vertices
        .iter()
        .map(|v| {
            let dir = *v - pos;
            let new_dir = vec2(
                dir.x * rot.cos - dir.y * rot.sin,
                dir.x * rot.sin + dir.y * rot.cos,
            );
            new_dir + pos
        })
        .collect()
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
