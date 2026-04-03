use bevy::{
    color::Color,
    ecs::component::Component,
    math::{Rot2, Vec2, Vec3, bounding::Aabb2d},
    render::render_resource::ShaderType,
};
use bytemuck::{NoUninit, Pod, Zeroable};

use crate::{buffers::BufferIndex, change::Changes, occluders::Occluder2dShape};

/// Component with data extracted to the Render World from Occluders.
#[derive(Component, Clone)]
#[require(PolyOccluderIndex, RoundOccluderIndex)]
pub struct ExtractedOccluder {
    pub color: Color,
    pub opacity: f32,
    pub z_sorting: bool,
    pub pos: Vec2,
    pub rot: f32,
    pub shape: Occluder2dShape,
    pub aabb: Aabb2d,
    pub z: f32,
    pub changes: Changes,
    pub index: u32,
}

// impl PartialEq for ExtractedOccluder {
//     fn eq(&self, other: &Self) -> bool {
//         self.pos == other.pos && self.rot == other.rot && self.shape == other.shape
//     }
// }

impl ExtractedOccluder {
    /// Get the occluder's vertices. This will be an empty Vec if the occluder has no vertices.
    pub fn vertices(&self) -> Vec<Vec2> {
        self.shape.vertices(self.pos, Rot2::radians(self.rot))
    }
    /// Get an iterator of the occluder's vertices. This will panic if the occluder has no vertices.
    pub fn vertices_iter<'a>(&'a self) -> Box<dyn 'a + DoubleEndedIterator<Item = Vec2>> {
        self.shape
            .vertices_iter(self.pos, Rot2::radians(self.rot))
            .unwrap()
    }
}

#[derive(Component, Clone, Copy, Default)]
pub struct RoundOccluderIndex(pub Option<BufferIndex>);

#[derive(Component, Clone, Copy, Default)]
pub struct PolyOccluderIndex {
    pub occluder: Option<BufferIndex>,
    pub vertices: Option<BufferIndex>,
}

/// Data that is transferred to the GPU to be read inside shaders.
#[repr(C)]
#[derive(ShaderType, Clone, Copy, Default, NoUninit)]
pub struct UniformPolyOccluder {
    pub vertex_start: u32,
    pub n_vertices: u32,
    pub z: f32,
    pub color: Vec3,
    pub _pad0: f32,
    pub opacity: f32,
    pub z_sorting: u32,
    pub index: u32,
    pub _pad1: [u32; 2],
}

/// Data that is transferred to the GPU to be read inside shaders.
#[repr(C)]
#[derive(ShaderType, Clone, Copy, Default, NoUninit)]
pub struct UniformRoundOccluder {
    pub pos: Vec2,
    pub rot: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub z: f32,
    pub color: Vec3,
    pub _pad0: f32,
    pub opacity: f32,
    pub z_sorting: u32,
    pub _pad1: [u32; 3],
}

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Zeroable, Pod, Default)]
pub(crate) struct UniformVertex {
    pub angle: f32,
    pub pos: Vec2,
}
