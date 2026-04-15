//! Module containing structs and functions relevant to Occluders.

use bevy::{
    camera::visibility::{VisibilityClass, add_visibility_class},
    color::palettes::css::BLACK,
    math::bounding::{Aabb2d, BoundingVolume},
    prelude::*,
    render::sync_world::SyncToRenderWorld,
};
use core::f32;

use crate::{
    change::Changes,
    occluders::convex_decomposition::{convex_decomposition, line_decomposition},
};
use crate::{
    occluders::convex_decomposition::complementary_decomposition,
    visibility::{OccluderAabb, VisibilityTimer},
};

pub mod render;
pub use render::*;

pub mod shape;
pub use shape::*;

pub mod convex_decomposition;

/// An occluder that blocks light.
///
/// Can be semi-transparent, have a color, any polygonal shape
/// and a few other select shapes (capsule, circle, round_rectangle).
///
/// Can be moved around or rotated by their transform.
///
/// Only z-axis rotations are allowed, any other type of rotation can cause unexpected behavior and bugs.
#[derive(Component, Clone, Reflect, Default, Debug)]
#[require(
    SyncToRenderWorld,
    Transform,
    VisibilityClass,
    ViewVisibility,
    VisibilityTimer,
    OccluderAabb,
    Changes
)]
#[component(on_add = add_visibility_class::<Occluder2d>)]
pub struct Occluder2d {
    pub(crate) internal_shape: Occluder2dInternalShape,

    /// Color of the occluder. **Alpha is ignored**.
    pub color: Color,

    /// Opacity of the occluder.
    ///
    /// An occluder of opacity 0 won't block any light.
    /// An occluder of opacity 1 will completely block light (and cast a fully black shadow).
    ///
    /// Anything in-between will cast a colored shadow depending on how opaque it is.
    pub opacity: f32,

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
    /// Get the occluder's internal shape.
    pub fn shape(&self) -> &Occluder2dInternalShape {
        &self.internal_shape
    }

    fn from_shape(internal_shape: Occluder2dInternalShape) -> Self {
        Self {
            internal_shape,
            opacity: 1.,
            color: bevy::prelude::Color::Srgba(BLACK),
            z_sorting: true,
            offset: default(),
        }
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
    pub fn polygon(vertices: impl Into<Vec<Vec2>>) -> Option<Self> {
        let vertices = vertices.into();

        if vertices.len() < 2 {
            return None;
        }

        Some(Self::from_shape(Occluder2dInternalShape::Polygon {
            vertices,
        }))
    }

    /// Construct a polyline occluder from the given points.
    ///
    /// Having self-intersections can cause unexpected behavior.
    ///
    /// The points should be relative to the entity's translation.
    ///
    /// # Failure
    /// This returns None if the provided list doesn't contain at least 2 vertices.
    pub fn polyline(vertices: impl Into<Vec<Vec2>>) -> Option<Self> {
        let vertices = vertices.into();

        if vertices.len() < 2 {
            return None;
        }

        Some(Self::from_shape(Occluder2dInternalShape::Polyline {
            vertices,
        }))
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
        Self::from_shape(Occluder2dInternalShape::Round {
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

#[derive(Component, Clone, Copy, PartialEq, Reflect, Debug)]
#[component(immutable)]
pub struct Occluder2dStyle {
    pub color: Color,
    pub opacity: f32,
    pub z_sorting: bool,
    pub offset: Vec3,
}

pub(crate) fn point_inside_poly(p: Vec2, mut poly: Vec<Vec2>, aabb: Aabb2d) -> bool {
    if !aabb.contains(&Aabb2d { min: p, max: p }) {
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

#[derive(Component)]
pub struct OccluderIndex(pub u32);

/// Plugin that adds general main-world behavior relating to occluders. This is mainly responsible for
/// change and visibility detection. It is added automatically by the [`FireflyPlugin`](crate::prelude::FireflyPlugin).   
pub struct OccluderPlugin;

impl Plugin for OccluderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_new_occluders, propagate_transform));
        app.add_observer(despawn_unlinked_convex_occluders);
    }
}

fn handle_new_occluders(
    occluders: Query<
        (Entity, &Occluder2d, &GlobalTransform, Option<&ConvexShapes>),
        Changed<Occluder2d>,
    >,
    mut commands: Commands,
) {
    for (entity, occluder, transform, convex_shapes) in occluders {
        let index = entity.index().index();

        if let Some(convex_shapes) = convex_shapes {
            for entity in convex_shapes.collection() {
                commands.entity(*entity).despawn();
            }
        }

        let mut entity = commands.entity(entity);
        let style = Occluder2dStyle {
            color: occluder.color,
            opacity: occluder.opacity,
            z_sorting: occluder.z_sorting,
            offset: occluder.offset,
        };

        entity.insert(style);

        let shape = occluder.shape();

        match shape {
            &Occluder2dInternalShape::Round {
                width,
                height,
                radius,
            } => {
                entity.insert((
                    Occluder2dShape::Round {
                        width,
                        height,
                        radius,
                    },
                    OccluderIndex(index),
                ));
            }
            Occluder2dInternalShape::Polygon { vertices } => {
                let parent = entity.id();

                let decomp = convex_decomposition(vertices.clone());

                let mut n_convex = 0;

                if let Some(decomp) = decomp {
                    n_convex = decomp.len();
                    for convex in decomp {
                        commands.spawn((
                            Occluder2dShape::Convex { vertices: convex },
                            style,
                            ConvexShapeOf(parent),
                            OccluderIndex(index),
                            transform.compute_transform(),
                        ));
                    }
                }

                if n_convex > 1 {
                    let decomp = complementary_decomposition(vertices.clone());
                    if let Some(decomp) = decomp {
                        for convex in decomp {
                            commands.spawn((
                                Occluder2dShape::Convex { vertices: convex },
                                style,
                                ConvexShapeOf(parent),
                                OccluderIndex(index),
                                ComplementaryShape,
                                transform.compute_transform(),
                            ));
                        }
                    }
                }
            }
            Occluder2dInternalShape::Polyline { vertices } => {
                let parent = entity.id();

                info!("target: {vertices:?}");
                let decomp = line_decomposition(vertices);
                info!("decomp: {decomp:?}");

                if let Some(decomp) = decomp {
                    for convex in decomp {
                        commands.spawn((
                            Occluder2dShape::Convex { vertices: convex },
                            style,
                            ConvexShapeOf(parent),
                            OccluderIndex(index),
                            transform.compute_transform(),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}

fn propagate_transform(
    occluders: Query<
        (&GlobalTransform, &ConvexShapes),
        (With<Occluder2d>, Changed<GlobalTransform>),
    >,
    mut convex_shapes: Query<&mut Transform, With<ConvexShapeOf>>,
) {
    for (transform, shapes) in occluders {
        for shape in shapes.collection() {
            let Ok(mut shape) = convex_shapes.get_mut(*shape) else {
                continue;
            };

            shape.translation = transform.translation();
            shape.rotation = transform.rotation();
        }
    }
}

fn despawn_unlinked_convex_occluders(trigger: On<Remove, ConvexShapeOf>, mut commands: Commands) {
    commands.entity(trigger.entity).try_despawn();
}
