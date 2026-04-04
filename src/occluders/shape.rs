use std::ops::Range;

use bevy::{
    camera::visibility::{ViewVisibility, VisibilityClass},
    ecs::{component::Component, entity::Entity},
    math::{Rot2, Vec2},
    reflect::Reflect,
    render::sync_world::SyncToRenderWorld,
    transform::components::Transform,
    utils::default,
};

use crate::{
    change::Changes,
    visibility::{OccluderAabb, VisibilityTimer},
};

/// The internal shape of an [`Occluder`](crate::prelude::Occluder2d). This is intended to be generated automatically through
/// the occluder's constructor methods and not created manually.   

#[derive(Clone, Reflect, Debug)]
pub enum Occluder2dInternalShape {
    Polygon {
        vertices: Vec<Vec2>,
    },
    Polyline {
        vertices: Vec<Vec2>,
    },
    Round {
        width: f32,
        height: f32,
        radius: f32,
    },
}

impl Default for Occluder2dInternalShape {
    fn default() -> Self {
        Self::Round {
            width: 10.,
            height: 10.,
            radius: 0.,
        }
    }
}

/// Component indicating the shape of an occluder, typically inserted automatically when the [`Occluder2d`](crate::prelude::Occluder2d)
/// component is added.
///
/// If the shape is a concave polygon, multiple entities will be spawned and linked to this via the [`ConvexShapeOf`](super::ConvexShapeOf) relationship, each
/// having a different convex shape.
#[derive(Component, Clone, PartialEq, Reflect, Debug)]
#[require(
    SyncToRenderWorld,
    Transform,
    VisibilityClass,
    ViewVisibility,
    VisibilityTimer,
    OccluderAabb,
    Changes
)]
#[component(immutable)]
pub enum Occluder2dShape {
    Convex {
        vertices: Vec<Vec2>,
    },
    Round {
        width: f32,
        height: f32,
        radius: f32,
    },
}

impl Occluder2dShape {
    pub(crate) fn n_vertices(&self) -> u32 {
        match &self {
            Self::Convex { vertices, .. } => vertices.len() as u32,
            _ => 0,
        }
    }

    pub(crate) fn vertices(&self, pos: Vec2, rot: Rot2) -> Vec<Vec2> {
        match &self {
            Self::Convex { vertices, .. } => translate_vertices(vertices.to_vec(), pos, rot),
            _ => default(),
        }
    }
    pub(crate) fn vertices_iter<'a>(
        &'a self,
        pos: Vec2,
        rot: Rot2,
    ) -> Option<Box<dyn 'a + DoubleEndedIterator<Item = Vec2>>> {
        match self {
            Self::Convex { vertices, .. } => Some(translate_vertices_iter(
                Box::new(vertices.iter().map(|v| *v)),
                pos,
                rot,
            )),
            _ => None,
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

#[derive(Component, Reflect, Debug)]
#[relationship_target(relationship = ConvexShapeOf, linked_spawn)]
pub struct ConvexShapes(Vec<Entity>);

#[derive(Component, Reflect, Debug)]
#[relationship(relationship_target = ConvexShapes)]
pub struct ConvexShapeOf(pub Entity);

#[derive(Component, Debug)]
pub struct ComplementaryShape;
