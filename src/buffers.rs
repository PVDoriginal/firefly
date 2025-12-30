//! This module contains structs and functions that create and manage render-world entities and GPU buffers.

use std::{collections::VecDeque, f32::consts::PI};

use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        render_resource::{
            BindingResource, BufferUsages, BufferVec, RawBufferVec, ShaderType,
            encase::private::WriteInto,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use bytemuck::{NoUninit, Pod, Zeroable};

use crate::{
    occluders::{
        ExtractedOccluder, Occluder2dShape, RoundOccluderIndex, UniformOccluder,
        UniformRoundOccluder, UniformVertex,
    },
    visibility::NotVisible,
};

/// Plugin that adds systems and observers for managing GPU buffers. This is added automatically through [`FireflyPlugin`](crate::prelude::FireflyPlugin)
pub struct BuffersPlugin;

impl Plugin for BuffersPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(RenderStartup, spawn_observers);
        render_app.add_systems(
            Render,
            prepare_occluders
                .in_set(RenderSystems::Prepare)
                .before(crate::prepare::prepare_data),
        );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<BufferManager<UniformRoundOccluder>>();
    }
}

fn spawn_observers(mut commands: Commands) {
    commands.spawn(Observer::new(on_occluder_removed));
    commands.spawn(Observer::new(on_occluder_not_visible));
}

// handles buffer when the occluder gets despawned or the component is removed
fn on_occluder_removed(
    trigger: On<Remove, ExtractedOccluder>,
    mut occluders: Query<(&ExtractedOccluder, &mut RoundOccluderIndex), With<ExtractedOccluder>>,
    mut manager: ResMut<BufferManager<UniformRoundOccluder>>,
) {
    if let Ok((occluder, mut index)) = occluders.get_mut(trigger.entity) {
        if !matches!(occluder.shape, Occluder2dShape::RoundRectangle { .. }) {
            return;
        }

        if let Some(old_index) = index.0 {
            manager.free_index(old_index);
            index.0 = None;
        }
    }
}

// handles buffer when occluder is not visible anymore
fn on_occluder_not_visible(
    trigger: On<Add, NotVisible>,
    mut occluders: Query<(Entity, &ExtractedOccluder, &mut RoundOccluderIndex), With<NotVisible>>,
    mut manager: ResMut<BufferManager<UniformRoundOccluder>>,
    mut commands: Commands,
) {
    if let Ok((id, occluder, mut index)) = occluders.get_mut(trigger.entity) {
        if !matches!(occluder.shape, Occluder2dShape::RoundRectangle { .. }) {
            return;
        }

        if let Some(old_index) = index.0 {
            manager.free_index(old_index);
            index.0 = None;
        }

        commands.entity(id).remove::<ExtractedOccluder>();
        commands.entity(id).remove::<NotVisible>();
    }
}

// adds occluders to buffers for use in prepare system
fn prepare_occluders(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut occluders: Query<(&ExtractedOccluder, &mut RoundOccluderIndex)>,
    mut manager: ResMut<BufferManager<UniformRoundOccluder>>,
) {
    for (occluder, mut index) in &mut occluders {
        if !occluder.changed_form && !index.0.is_none() {
            continue;
        }

        if let Occluder2dShape::RoundRectangle {
            width,
            height,
            radius,
        } = occluder.shape
        {
            let value = UniformRoundOccluder {
                pos: occluder.pos,
                rot: occluder.rot,
                width,
                height,
                radius,
                _padding: default(),
            };

            let new_index = manager.set_value(&value, index.0);
            index.0 = Some(new_index);
        }
    }
    manager.flush(&render_device, &render_queue);
}

/// This resource is a wrapper around [`RawBufferVec`] that reserves and distributes VRAM slots to
/// a set of entities that are intended to be transferred to the GPU.  
#[derive(Resource)]
pub struct BufferManager<T: ShaderType + WriteInto + Default + NoUninit> {
    buffer: RawBufferVec<T>,
    next_index: usize,
    free_indices: VecDeque<usize>,
    write_min: usize,
    write_max: usize,
    current_generation: u32,
}

impl<T: ShaderType + WriteInto + Default + NoUninit> FromWorld for BufferManager<T> {
    fn from_world(world: &mut bevy::prelude::World) -> BufferManager<T> {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();

        Self::new(device, queue)
    }
}

impl<T: ShaderType + WriteInto + Default + NoUninit> BufferManager<T> {
    fn new_index(&mut self) -> usize {
        self.free_indices.pop_back().unwrap_or_else(|| {
            self.next_index += 1;
            self.next_index - 1
        })
    }

    fn new(device: &RenderDevice, queue: &RenderQueue) -> Self {
        let mut res = Self {
            buffer: RawBufferVec::<T>::new(BufferUsages::STORAGE),
            next_index: default(),
            free_indices: default(),
            write_min: usize::MAX,
            write_max: usize::MIN,
            current_generation: 0,
        };

        // empty value is added so the buffer can be written to VRAM from the start
        res.buffer.push(default());
        res.buffer.write_buffer(device, queue);

        res
    }

    pub fn binding(&self) -> BindingResource<'_> {
        self.buffer.binding().unwrap()
    }

    /// Called by an entity to pass it's current index and value to the buffer.
    /// It returns back it's (possibly changed) index.  
    ///
    /// It is an entity's responsibility to store the received index and use it in subsequent calls.
    ///
    /// If an entity didn't have any changes, it shouldn't call this.
    pub fn set_value(&mut self, value: &T, index: Option<BufferIndex>) -> BufferIndex {
        let index = match index {
            None => self.new_index(),
            Some(BufferIndex { index, generation }) => {
                if index < self.next_index && generation == self.current_generation {
                    index
                } else {
                    self.new_index()
                }
            }
        };

        if index + 1 >= self.buffer.len() {
            self.buffer.push(*value);
        } else {
            self.buffer.set(index as u32 + 1, *value);
        }

        self.write_min = self.write_min.min(index + 1);
        self.write_max = self.write_max.max(index + 1);

        BufferIndex {
            index,
            generation: self.current_generation,
        }
    }

    /// Flush the changes at the end of a render frame. This writes all changes to the GPU.
    pub fn flush(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.write_min != usize::MAX {
            if self.write_max >= self.buffer.capacity() {
                self.buffer.reserve(
                    ((self.write_max + 1) as f32 / 1024.0).ceil() as usize * 1024,
                    device,
                );
                self.buffer.write_buffer(device, queue);
            } else {
                self.buffer
                    .write_buffer_range(queue, self.write_min as usize..self.write_max as usize + 1)
                    .expect("couldn't write to buffer");
            }
        }

        // info!(
        //     "Finished writing! Buffer length: {}, Element size: {}, Buffer size: {}, Buffer capacity: {}, Unoccupied: {}",
        //     self.buffer.len(),
        //     T::min_size().get(),
        //     self.buffer.buffer().unwrap().size(),
        //     self.buffer.capacity(),
        //     self.free_indices.len(),
        // );

        // Refragmentation. Because of wasted space the buffer will empty itself and pass all-new data next frame. This can be optimized
        if self.free_indices.len() > self.buffer.capacity() as usize / 2 {
            let old_generation = self.current_generation;
            *self = Self::new(device, queue);
            self.current_generation = old_generation + 1;
        }

        self.write_min = usize::MAX;
        self.write_max = usize::MIN;
    }

    /// An entity that has gone out of view, been despawned, or is no longer intended to be rendered,
    /// has to call this method to free it's Buffer slot.
    ///
    /// The index / slot will be automatically redistributed to another entity when needed.
    pub fn free_index(&mut self, index: BufferIndex) {
        if index.generation != self.current_generation {
            return;
        };

        if index.index >= self.buffer.len() {
            return;
        }

        self.free_indices.push_front(index.index);
    }
}

/// The amount of bins that each [`Bins`] will have.
pub const N_BINS: usize = 256;

/// The amount of occluder per bin.
pub const N_OCCLUDERS: usize = 64;

/// A component that each light has, containing sets of bins of occluders for
/// faster shader iteration.
#[derive(Component)]
pub struct BinBuffer {
    buffer: RawBufferVec<[[OccluderPointer; N_OCCLUDERS]; N_BINS]>,
    counts: [(usize, usize); N_BINS],
}

impl Default for BinBuffer {
    fn default() -> Self {
        Self {
            buffer: RawBufferVec::<[[OccluderPointer; N_OCCLUDERS]; N_BINS]>::new(
                BufferUsages::STORAGE,
            ),
            counts: [(0, 0); N_BINS],
        }
    }
}

impl BinBuffer {
    const PI2: f32 = PI * 2.0;
    const N_BINS: f32 = N_BINS as f32;

    fn push_empty(&mut self) {
        self.buffer.push([[default(); N_OCCLUDERS]; N_BINS]);
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.push_empty();
    }

    pub fn add_occluder(&mut self, occluder: OccluderPointer, min_angle: f32, max_angle: f32) {
        let min_bin = ((min_angle / Self::PI2) * Self::N_BINS).floor() as usize;
        let max_bin = ((max_angle / Self::PI2) * Self::N_BINS).ceil() as usize;

        for bin in min_bin..max_bin {
            if self.counts[bin].0 >= self.buffer.len() {
                self.push_empty();
            }

            let values = self.buffer.values_mut();
            values[self.counts[bin].0][bin][self.counts[bin].1] = occluder;

            if self.counts[bin].1 + 1 == N_OCCLUDERS {
                self.counts[bin] = (self.counts[bin].0 + 1, 0);
            } else {
                self.counts[bin].1 += 1;
            }
        }
    }
}

/// Compact struct pointing to an occluder.
#[repr(C)]
#[derive(Default, Pod, Zeroable, Clone, Copy)]
pub struct OccluderPointer {
    pub index: u32,
    pub min_v: u32,
    pub max_v: u32,
    pub distance: f32,
}

// Index - ( 00  |      ..     )
//          type   actual index

// type: 0 - end of buffer
//       1 - round occluder
//       2 - polygonal occluder

// pub struct VertexBuffer {
//     vertices: BufferVec<Vec2>,
//     next_index: usize,
//     min_index: usize,
//     empty_slots: u32,
// }

// impl FromWorld for VertexBuffer {
//     fn from_world(world: &mut World) -> Self {
//         let device = world.resource::<RenderDevice>();
//         let queue = world.resource::<RenderQueue>();

//         Self::new(device, queue)
//     }
// }

// impl VertexBuffer {
//     fn new(device: &RenderDevice, queue: &RenderQueue) -> Self {
//         let mut res = Self {
//             vertices: BufferVec::<UniformVertex>::new(BufferUsages::STORAGE),
//             next_index: 0,
//             min_index: usize::MAX,
//             empty_slots: 0,
//         };

//         // empty value is added so the buffer can be written to VRAM from the start
//         res.vertices.push(default());
//         res.vertices.write_buffer(device, queue);

//         res
//     }

//     pub fn add_vertices(&mut self, occluder: &ExtractedOccluder) -> usize {
//         for vertex in occluder.vertices_iter() {}
//     }
// }

#[derive(Clone, Copy)]
pub struct BufferIndex {
    pub index: usize,
    pub generation: u32,
}
