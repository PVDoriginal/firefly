use bevy::{
    color::palettes::css::BLACK,
    prelude::*,
    render::{
        render_resource::{BufferUsages, BufferVec, GpuArrayBuffer, ShaderType},
        renderer::RenderDevice,
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
#[require(SyncToRenderWorld, Transform, ViewVisibility)]
pub struct Occluder2d {
    shape: Occluder2dShape,
    rect: Rect,

    /// Color of the occluder. **Alpha is ignored**.
    pub color: Color,

    /// Opacity of the occluder.
    ///
    /// An occluder of opacity 0 won't block any light.
    /// An occluder of opacity 1 will completely both light (and cast a fully black shadow).
    ///
    /// Anything in-between will cast a colored shadow depending on how opaque it is.
    pub opacity: f32,

    /// List of entities that this occluder will not cast shadows over.
    ///
    /// Note that these can be have a significant impact on performance. [`crate::prelude::FireflyConfig::z_sorting`] should be used instead if possible.  
    pub ignored_sprites: Vec<Entity>,

    /// If true, this occluder won't cast shadows over sprites with a higher z value.
    ///
    /// This does nothing if z_sorting is set to false in the [config](crate::prelude::FireflyConfig::z_sorting).
    pub z_sorting: bool,

    /// Offset to the position of the occluder.
    ///
    /// **Default**: [Vec3::ZERO].
    pub offset: Vec3,
}

impl Occluder2d {
    /// Get the occluder's **internal shape**.
    pub fn shape(&self) -> &Occluder2dShape {
        &self.shape
    }

    fn from_shape(shape: Occluder2dShape) -> Self {
        let rect = match &shape {
            Occluder2dShape::RoundRectangle {
                width,
                height,
                radius,
            } => Rect {
                min: Vec2::splat(-(width.max(*height)) / 2. - radius),
                max: Vec2::splat(width.max(*height) / 2. + radius),
            },
            Occluder2dShape::Polyline { vertices, .. } => vertices_rect(vertices),
            Occluder2dShape::Polygon { vertices, .. } => vertices_rect(vertices),
        };

        Self {
            shape,
            rect,
            opacity: 1.,
            color: bevy::prelude::Color::Srgba(BLACK),
            z_sorting: true,
            ..default()
        }
    }

    /// Bounding rect of the occluder.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Construct a new occluder with the specified [color](Occluder2d::color).
    pub fn with_color(&self, color: Color) -> Self {
        let mut res = self.clone();
        res.color = color;
        res
    }

    /// Construct a new occluder with the specified [opacity](Occluder2d::opacity).
    pub fn with_opacity(&self, opacity: f32) -> Self {
        let mut res = self.clone();
        res.opacity = opacity;
        res
    }

    /// Construct a new occluder with the specified [ignored sprites](Occluder2d::ignored_sprites).
    pub fn with_ignored_sprites(&self, sprites: Vec<Entity>) -> Self {
        let mut res = self.clone();
        res.ignored_sprites = sprites;
        res
    }

    /// Construct a new occluder with the specified [z-sorting](Occluder2d::z_sorting).
    pub fn with_z_sorting(&self, z_sorting: bool) -> Self {
        let mut res = self.clone();
        res.z_sorting = z_sorting;
        res
    }

    /// Construct a new occluder with the specified [offset](Occluder2d::offset).
    pub fn with_offset(&self, offset: Vec3) -> Self {
        let mut res = self.clone();
        res.offset = offset;
        res
    }

    /// Construct a polygonal occluder from the given points.
    ///
    /// The points can form a convex or concave polygon. However,
    /// having self-intersections can cause unexpected behavior.
    ///
    /// The points should be relative to the entity's translation.
    ///
    /// # Failure
    /// This returns None if the provided list doesn't contain at least 2 vertices.
    pub fn polygon(vertices: Vec<Vec2>) -> Option<Self> {
        normalize_vertices(vertices).and_then(|mut vertices| {
            vertices.push(vertices[0]);
            Some(Self::from_shape(Occluder2dShape::Polygon { vertices }))
        })
    }

    /// Construct a polyline occluder from the given points.
    ///
    /// Having self-intersections can cause unexpected behavior.
    ///
    /// The points should be relative to the entity's translation.
    ///
    /// # Failure
    /// This returns None if the provided list doesn't contain at least 2 vertices.
    pub fn polyline(mut vertices: Vec<Vec2>) -> Option<Self> {
        let mut vertices_clone = vertices.clone();
        vertices_clone.reverse();
        vertices.extend_from_slice(&vertices_clone[1..vertices_clone.len()]);

        normalize_vertices(vertices)
            .and_then(|vertices| Some(Self::from_shape(Occluder2dShape::Polyline { vertices })))
    }

    /// Construct a rectangle occluder from width and height.
    pub fn rectangle(width: f32, height: f32) -> Self {
        Self::round_rectangle(width, height, 0.)
    }

    /// Construct a round rectangle occluder from width, height and radius.
    ///
    /// The resulted occluder is esentially a rectangle with a radius-sized padding around it.
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

    /// Construct a capsule occluder. This is vertical by default. For a horizontal capsule check [`Occluder2d::horizontal_capsule()`].
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
    pub z_sorting: bool,
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
    pub fn vertices_iter<'a>(&'a self) -> Box<dyn 'a + DoubleEndedIterator<Item = Vec2>> {
        self.shape
            .vertices_iter(self.pos, Rot2::radians(self.rot))
            .unwrap()
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

// rotates vertices to be clockwise
fn normalize_vertices(vertices: Vec<Vec2>) -> Option<Vec<Vec2>> {
    if vertices.len() < 2 {
        warn!("Not enough vertices to form shape");
        return None;
    }

    if vertices.len() < 3 {
        return Some(vertices.to_vec());
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
        return Some(vertices.to_vec());
    }

    if orientations.contains(&Orientation::Left) {
        return Some(vertices.iter().rev().map(|x| *x).collect());
    }

    Some(vertices.to_vec())
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

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformOccluder {
    pub n_sequences: u32,
    pub n_vertices: u32,
    pub z: f32,
    pub color: Vec3,
    pub opacity: f32,
    pub z_sorting: u32,
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformRoundOccluder {
    pub pos: Vec2,
    pub rot: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
}

#[derive(Resource)]
pub(crate) struct OccluderBuffers {
    pub round_occluders: BufferVec<UniformRoundOccluder>,
}

impl Default for OccluderBuffers {
    fn default() -> Self {
        Self {
            round_occluders: BufferVec::<UniformRoundOccluder>::new(BufferUsages::STORAGE),
        }
    }
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
    },
    Polyline {
        vertices: Vec<Vec2>,
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
    pub(crate) fn vertices_iter<'a>(
        &'a self,
        pos: Vec2,
        rot: Rot2,
    ) -> Option<Box<dyn 'a + DoubleEndedIterator<Item = Vec2>>> {
        match self {
            Self::Polygon { vertices, .. } => Some(translate_vertices_iter(
                Box::new(vertices.iter().map(|v| *v)),
                pos,
                rot,
            )),
            Self::Polyline { vertices, .. } => Some(translate_vertices_iter(
                Box::new(vertices.iter().map(|v| *v)),
                pos,
                rot,
            )),
            Self::RoundRectangle { .. } => None,
        }
    }
}

pub(crate) fn translate_vertices(vertices: Vec<Vec2>, pos: Vec2, rot: Rot2) -> Vec<Vec2> {
    vertices.iter().map(|v| rot * *v + pos).collect()
}

pub(crate) fn translate_vertices_iter<'a>(
    vertices: Box<dyn 'a + DoubleEndedIterator<Item = Vec2>>,
    pos: Vec2,
    rot: Rot2,
) -> Box<dyn 'a + DoubleEndedIterator<Item = Vec2>> {
    Box::new(vertices.map(move |v| rot * v + pos))
}
