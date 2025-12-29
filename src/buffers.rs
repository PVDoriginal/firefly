//! This module contains structs and functions that create and manage render-world entities and GPU buffers.

use std::collections::VecDeque;

use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        render_resource::{
            BindingResource, BufferUsages, RawBufferVec, ShaderType, encase::private::WriteInto,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use bytemuck::NoUninit;

use crate::{
    occluders::{ExtractedOccluder, Occluder2dShape, OccluderIndex, UniformRoundOccluder},
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
    mut occluders: Query<(&ExtractedOccluder, &mut OccluderIndex), With<ExtractedOccluder>>,
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
    mut occluders: Query<(Entity, &ExtractedOccluder, &mut OccluderIndex), With<NotVisible>>,
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
    mut occluders: Query<(&ExtractedOccluder, &mut OccluderIndex)>,
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
    pub fn set_value(&mut self, value: &T, index: Option<usize>) -> usize {
        let index = match index {
            None => self.new_index(),
            Some(i) => {
                if i < self.next_index {
                    i
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

        index
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
            *self = Self::new(device, queue);
        }

        self.write_min = usize::MAX;
        self.write_max = usize::MIN;
    }

    /// An entity that has gone out of view, been despawned, or is no longer intended to be rendered,
    /// has to call this method to free it's Buffer slot.
    ///
    /// The index / slot will be automatically redistributed to another entity when needed.
    pub fn free_index(&mut self, index: usize) {
        if index >= self.buffer.len() {
            return;
        }
        self.free_indices.push_front(index);
    }
}
