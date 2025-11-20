use bevy::{
    color::palettes::css::BLACK,
    prelude::*,
    render::{
        render_resource::{GpuArrayBuffer, ShaderType},
        sync_world::SyncToRenderWorld,
    },
};
use core::f32;

/// An occluder that blocks light.
///
/// Can be semi-transparent, have a color, any polygonal shape
/// and a few other select shapes (capsule, circle, round_rectangle).
///
/// Can be moved around or rotated by their transform.
///
/// Only z-axis rotations are allowed, any other type of rotation can cause unexpected behavior and bugs.
#[derive(Component, Clone, Default, Reflect)]
#[require(SyncToRenderWorld)]
pub struct Occluder2d {
    shape: Occluder2dShape,
    /// Color of the occluder, alpha is ignored.
    pub color: Color,
    /// Opacity of the occluder.
    ///
    /// An occluder of opacity 0 won't block any light.
    /// An occluder of opacity 1 will completely both light (and cast a fully black shadow).
    ///
    /// Anything in-between will cast a colored shadow depending on how opaque it is.
    pub opacity: f32,
    /// List of entities that this occluder will not cast shadows over.
    pub ignored_sprites: Vec<Entity>,
}

impl Occluder2d {
    pub fn shape(&self) -> &Occluder2dShape {
        &self.shape
    }

    fn from_shape(shape: Occluder2dShape) -> Self {
        Self {
            shape,
            opacity: 1.,
            color: bevy::prelude::Color::Srgba(BLACK),
            ..default()
        }
    }

    /// Bounding rect of the occluder.
    pub fn rect(&self) -> Rect {
        match &self.shape {
            Occluder2dShape::RoundRectangle {
                width,
                height,
                radius,
            } => Rect {
                min: Vec2::splat(-width.min(-*height) / 2. - radius),
                max: Vec2::splat(width.max(*height) / 2. + radius),
            },
            Occluder2dShape::Polyline { vertices, .. } => vertices_rect(vertices),
            Occluder2dShape::Polygon { vertices, .. } => vertices_rect(vertices),
        }
    }

    /// Construct a new occluder with the specified [color].
    ///
    /// [color]: Occluder2d::color
    pub fn with_color(&self, color: Color) -> Self {
        let mut res = self.clone();
        res.color = color;
        res
    }

    /// Construct a new occluder with the specified opacity.
    ///
    /// An occluder of opacity 0 won't block any light.
    /// An occluder of opacity 1 will completely both light (and cast a fully black shadow).
    ///
    /// Anything in-between will cast a colored shadow depending on how opaque it is.
    pub fn with_opacity(&self, opacity: f32) -> Self {
        let mut res = self.clone();
        res.opacity = opacity;
        res
    }

    /// Construct a polygonal occluder from the given points.
    ///
    /// The points can form a convex or concave polygon. However,
    /// having self-intersections can cause unexpected behavior.
    ///
    /// The points should be relative to the entity's translation.
    pub fn polygon(vertices: Vec<Vec2>) -> Option<Self> {
        normalize_vertices(vertices).and_then(|(vertices, concave)| {
            Some(Self::from_shape(Occluder2dShape::Polygon {
                vertices,
                concave,
            }))
        })
    }

    /// Construct a polyline occluder from the given points.
    ///
    /// Having self-intersections can cause unexpected behavior.
    ///
    /// The points should be relative to the entity's translation.
    pub fn polyline(vertices: Vec<Vec2>) -> Option<Self> {
        Some(Self::from_shape(Occluder2dShape::Polyline {
            vertices,
            concave: true,
        }))
    }

    /// Construct a rectangle occluder from width and height.
    pub fn rectangle(width: f32, height: f32) -> Self {
        Self::round_rectangle(width, height, 0.)
    }

    /// Construct a round rectangle occluder from width, height and radius.
    ///
    /// The resulted occluder is esentially a rectangle a radius-sized padding around it.
    ///  
    /// For instance, a circle is a round rectangle with no height or width, and a capsule
    /// is a round rectangle with only height or only width (and radius).
    pub fn round_rectangle(width: f32, height: f32, radius: f32) -> Self {
        Self::from_shape(Occluder2dShape::RoundRectangle {
            width,
            height,
            radius,
        })
    }

    /// Construct a circle occluder.
    pub fn circle(radius: f32) -> Self {
        Self::round_rectangle(0., 0., radius)
    }

    /// Construct a vertical capsule occluder.
    pub fn vertical_capsule(length: f32, radius: f32) -> Self {
        Self::round_rectangle(0., length, radius)
    }

    /// Construct a horizontal_capsule occluder.
    pub fn horizontal_capsule(length: f32, radius: f32) -> Self {
        Self::round_rectangle(length, 0., radius)
    }

    /// Construct a capsule occluder. This is vertical by default. For a horizontal capsule check [`Occluder2d::horizontal_capsule()`]
    pub fn capsule(length: f32, radius: f32) -> Self {
        Self::vertical_capsule(length, radius)
    }
}

#[derive(Component, Clone, Debug)]
pub(crate) struct ExtractedOccluder {
    pub pos: Vec2,
    pub rot: f32,
    pub shape: Occluder2dShape,
    pub rect: Rect,
    pub z: f32,
    pub color: Color,
    pub opacity: f32,
    pub ignored_sprites: Vec<Entity>,
}

impl PartialEq for ExtractedOccluder {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos && self.rot == other.rot && self.shape == other.shape
    }
}

impl ExtractedOccluder {
    pub fn vertices(&self) -> Vec<Vec2> {
        self.shape.vertices(self.pos, Rot2::radians(self.rot))
    }
}

fn vertices_rect(vertices: &Vec<Vec2>) -> Rect {
    let r = vertices
        .iter()
        .max_by(|a, b| a.length_squared().total_cmp(&b.length_squared()))
        .unwrap()
        .length();

    Rect {
        min: vec2(-r, -r),
        max: vec2(r, r),
    }
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformOccluder {
    pub n_sequences: u32,
    pub n_vertices: u32,
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

#[derive(Reflect, Clone, Debug, PartialEq)]
pub enum Occluder2dShape {
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

impl Default for Occluder2dShape {
    fn default() -> Self {
        Self::RoundRectangle {
            width: 10.,
            height: 10.,
            radius: 0.,
        }
    }
}

impl Occluder2dShape {
    pub fn is_round(&self) -> bool {
        matches!(self, Occluder2dShape::RoundRectangle { .. })
    }

    pub fn is_concave(&self) -> bool {
        match self {
            Self::Polygon { concave, .. } => *concave,
            Self::Polyline { concave, .. } => *concave,
            _ => false,
        }
    }
    pub fn is_line(&self) -> bool {
        matches!(self, Occluder2dShape::Polyline { .. })
    }

    pub(crate) fn vertices(&self, pos: Vec2, rot: Rot2) -> Vec<Vec2> {
        match &self {
            Self::Polygon { vertices, .. } => {
                let mut vertices = vertices.clone();
                vertices.push(vertices[0]);
                translate_vertices(vertices, pos, rot)
            }
            Self::Polyline { vertices, .. } => translate_vertices(vertices.to_vec(), pos, rot),
            Self::RoundRectangle { .. } => default(),
        }
    }
}

pub(crate) fn translate_vertices(vertices: Vec<Vec2>, pos: Vec2, rot: Rot2) -> Vec<Vec2> {
    vertices
        .iter()
        .map(|v| vec2(v.x * rot.cos - v.y * rot.sin, v.x * rot.sin + v.y * rot.cos) + pos)
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

pub(crate) fn point_inside_poly(p: Vec2, mut poly: Vec<Vec2>, rect: Rect) -> bool {
    if !rect.contains(p) {
        return false;
    }

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

#[derive(Resource, Default)]
pub(crate) struct OccluderSet(
    pub  Vec<(
        GpuArrayBuffer<UniformOccluder>,
        GpuArrayBuffer<u32>,
        GpuArrayBuffer<UniformVertex>,
        GpuArrayBuffer<UniformRoundOccluder>,
        GpuArrayBuffer<f32>,
    )>,
);
