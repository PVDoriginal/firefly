//! This module contains structs and functions that create and manage render-world entities and GPU buffers.
//!
//! Lights and Occluders are stored in global buffers through their own [`BufferManager`]s.
//!
//! Round and Polygonal Occluders are stores in separate buffers due to having significantly different structures.   
//!
//! Vertices for Polygonal Occluders are stored in a global [`VertexBuffer`].

use core::f32;
use std::{collections::VecDeque, f32::consts::PI};

use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        render_resource::{
            BindingResource, BufferUsages, RawBufferVec, ShaderType, StorageBuffer,
            encase::private::WriteInto,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use bytemuck::{NoUninit, Pod, Zeroable};

use crate::{
    lights::{ExtractedPointLight, Falloff, LightIndex, UniformPointLight},
    occluders::{
        ExtractedOccluder, Occluder2dShape, PolyOccluderIndex, RoundOccluderIndex, UniformOccluder,
        UniformRoundOccluder,
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
            (prepare_occluders, prepare_lights)
                .in_set(RenderSystems::Prepare)
                .before(crate::prepare::prepare_data),
        );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<BufferManager<UniformRoundOccluder>>();
        render_app.init_resource::<BufferManager<UniformOccluder>>();
        render_app.init_resource::<BufferManager<UniformPointLight>>();
        render_app.init_resource::<VertexBuffer>();
    }
}

fn spawn_observers(mut commands: Commands) {
    commands.spawn(Observer::new(on_occluder_removed));
    commands.spawn(Observer::new(on_light_removed));

    commands.spawn(Observer::new(on_entity_not_visible));
}

// handles buffer when the light gets despawned or the component is removed
fn on_light_removed(
    trigger: On<Remove, ExtractedPointLight>,
    mut lights: Query<&mut LightIndex>,
    mut light_manager: ResMut<BufferManager<UniformPointLight>>,
) {
    if let Ok(mut index) = lights.get_mut(trigger.entity) {
        if let Some(old_index) = index.0 {
            light_manager.free_index(old_index);
            index.0 = None;
        }
    }
}

// handles buffer when the occluder gets despawned or the component is removed
fn on_occluder_removed(
    trigger: On<Remove, ExtractedOccluder>,
    mut occluders: Query<
        (
            &ExtractedOccluder,
            &mut RoundOccluderIndex,
            &mut PolyOccluderIndex,
        ),
        With<ExtractedOccluder>,
    >,
    mut round_manager: ResMut<BufferManager<UniformRoundOccluder>>,
    mut poly_manager: ResMut<BufferManager<UniformOccluder>>,
    mut vertex_buffer: ResMut<VertexBuffer>,
) {
    if let Ok((occluder, mut round_index, mut poly_index)) = occluders.get_mut(trigger.entity) {
        if matches!(occluder.shape, Occluder2dShape::RoundRectangle { .. }) {
            if let Some(old_index) = round_index.0 {
                round_manager.free_index(old_index);
                round_index.0 = None;
            }
        } else {
            if let Some(old_index) = poly_index.occluder {
                poly_manager.free_index(old_index);
                poly_index.occluder = None;
            }
            if let Some(old_index) = poly_index.vertices {
                vertex_buffer.free_indices(occluder.shape.n_vertices(), old_index.generation);
                poly_index.vertices = None;
            }
        }
    }
}

// handles buffer when entity is not visible anymore
fn on_entity_not_visible(
    trigger: On<Add, NotVisible>,
    mut occluders: Query<
        (
            Entity,
            &ExtractedOccluder,
            &mut RoundOccluderIndex,
            &mut PolyOccluderIndex,
        ),
        With<NotVisible>,
    >,
    mut lights: Query<(Entity, &mut LightIndex)>,
    mut round_manager: ResMut<BufferManager<UniformRoundOccluder>>,
    mut poly_manager: ResMut<BufferManager<UniformOccluder>>,
    mut vertex_buffer: ResMut<VertexBuffer>,
    mut light_manager: ResMut<BufferManager<UniformPointLight>>,
    mut commands: Commands,
) {
    if let Ok((id, occluder, mut round_index, mut poly_index)) = occluders.get_mut(trigger.entity) {
        if matches!(occluder.shape, Occluder2dShape::RoundRectangle { .. }) {
            if let Some(old_index) = round_index.0 {
                round_manager.free_index(old_index);
                round_index.0 = None;
            }
        } else {
            if let Some(old_index) = poly_index.occluder {
                poly_manager.free_index(old_index);
                poly_index.occluder = None;
            }
            if let Some(old_index) = poly_index.vertices {
                vertex_buffer.free_indices(occluder.shape.n_vertices(), old_index.generation);
                poly_index.vertices = None;
            }
        }

        commands.entity(id).remove::<ExtractedOccluder>();
        commands.entity(id).remove::<NotVisible>();
    } else if let Ok((id, mut index)) = lights.get_mut(trigger.entity) {
        if let Some(old_index) = index.0 {
            light_manager.free_index(old_index);
            index.0 = None;
        }

        commands.entity(id).remove::<ExtractedPointLight>();
        commands.entity(id).remove::<NotVisible>();
    }
}

// adds lights to buffer for use in prepare system
fn prepare_lights(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut lights: Query<(&ExtractedPointLight, &mut LightIndex)>,
    mut light_manager: ResMut<BufferManager<UniformPointLight>>,
) {
    for (light, mut index) in &mut lights {
        let changed = light.changes.0;

        let light = UniformPointLight {
            pos: light.pos,
            intensity: light.intensity,
            range: light.range,
            color: light.color.to_linear().to_vec4(),
            z: light.z,
            inner_range: light.inner_range.min(light.range),
            falloff: match light.falloff {
                Falloff::InverseSquare => 0,
                Falloff::Linear => 1,
            },
            falloff_intensity: light.falloff_intensity,
            angle: light.angle / 180. * PI,
            dir: light.dir,
            height: light.height,
        };

        let new_index = light_manager.set_value(&light, index.0, changed);
        index.0 = Some(new_index);
    }

    light_manager.flush(&render_device, &render_queue);
}

// adds occluders to buffers for use in prepare system
fn prepare_occluders(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut occluders: Query<(
        &ExtractedOccluder,
        &mut RoundOccluderIndex,
        &mut PolyOccluderIndex,
    )>,
    mut round_manager: ResMut<BufferManager<UniformRoundOccluder>>,
    mut poly_manager: ResMut<BufferManager<UniformOccluder>>,
    mut vertex_buffer: ResMut<VertexBuffer>,
) {
    for (occluder, mut round_index, mut poly_index) in &mut occluders {
        let changed = occluder.changes.0;

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
                // padding: default(),
                z: occluder.z,
                color: occluder.color.to_linear().to_vec3(),
                _pad0: 0.0,
                opacity: occluder.opacity,
                z_sorting: match occluder.z_sorting {
                    true => 1,
                    false => 0,
                },
                _pad1: [0, 0, 0],
            };

            // assert_eq!(std::mem::size_of::<UniformRoundOccluder>(), 64);
            // assert_eq!(std::mem::align_of::<UniformRoundOccluder>(), 16);

            let new_index = round_manager.set_value(&value, round_index.0, changed);
            round_index.0 = Some(new_index);
        } else {
            let vertex_index = vertex_buffer.write_vertices(
                occluder,
                poly_index.vertices,
                &render_device,
                &render_queue,
                changed,
            );
            poly_index.vertices = Some(vertex_index);

            let value = UniformOccluder {
                vertex_start: vertex_index.index as u32,
                n_vertices: occluder.shape.n_vertices(),
                z: occluder.z,
                color: occluder.color.to_linear().to_vec3(),
                _pad0: 0.0,
                opacity: occluder.opacity,
                z_sorting: match occluder.z_sorting {
                    true => 1,
                    false => 0,
                },
                _pad1: [0, 0, 0],
            };

            let new_index = poly_manager.set_value(&value, poly_index.occluder, changed);
            poly_index.occluder = Some(new_index);
        }
    }

    round_manager.flush(&render_device, &render_queue);
    poly_manager.flush(&render_device, &render_queue);
    vertex_buffer.pass(&render_device, &render_queue);
}

/// This resource is a wrapper around [`RawBufferVec`] that reserves and distributes VRAM slots to
/// a set of entities that are intended to be transferred to the GPU. It is currently used for Occluders and Lights.
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
            next_index: 2,
            free_indices: default(),
            write_min: usize::MAX,
            write_max: usize::MIN,
            current_generation: 0,
        };

        res.buffer.set_label("global buffer".into());

        // empty value is added so the buffer can be written to VRAM from the start
        res.buffer.push(default());
        res.buffer.push(default());
        res.buffer.write_buffer(device, queue);

        res
    }

    /// Get the binding of this buffer. It is guaranteed to exist.
    pub fn binding(&self) -> BindingResource<'_> {
        self.buffer.binding().unwrap()
    }

    /// Called by an entity to pass it's current index and value to the buffer.
    /// It returns back it's (possibly changed) index.  
    ///
    /// It is an entity's responsibility to store the received index and use it in subsequent calls.
    ///
    /// If an entity didn't have any changes, it shouldn't call this.
    pub fn set_value(
        &mut self,
        value: &T,
        index: Option<BufferIndex>,
        changed: bool,
    ) -> BufferIndex {
        if !changed
            && let Some(index) = index
            && index.generation == self.current_generation
        {
            return index;
        }

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

        if index >= self.buffer.len() {
            self.buffer.push(*value);
        } else {
            self.buffer.set(index as u32, *value);
        }

        self.write_min = self.write_min.min(index);
        self.write_max = self.write_max.max(index);

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
        if self.free_indices.len() > 500
            && self.free_indices.len() > self.buffer.capacity() as usize / 2
        {
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
pub const N_BINS: usize = 128;

/// The amount of occluder per bin.
pub const N_OCCLUDERS: usize = 32;

/// A component that each light has, containing sets of bins of occluders for faster iteration.
/// This is the most important acceleration structure used by Firefly. It is used in a custom
/// type of angular sweep with BVH-inspired elements.
#[derive(Component)]
pub struct BinBuffer {
    buffer: RawBufferVec<[Bin; N_BINS]>,
    // the number of bins for each bin interval,
    // used in case not all occluders fit in a single set of bins
    counts: [u32; N_BINS],
    bin_counts: StorageBuffer<BinCounts>,
}

impl Default for BinBuffer {
    fn default() -> Self {
        Self {
            buffer: RawBufferVec::<[Bin; N_BINS]>::new(BufferUsages::STORAGE),
            counts: [0; N_BINS],
            bin_counts: StorageBuffer::<BinCounts>::default(),
        }
    }
}

/// A bin containing occluders.
#[repr(C)]
#[derive(Zeroable, Pod, Clone, Copy, ShaderType)]
pub struct Bin {
    pub occluders: [OccluderPointer; N_OCCLUDERS],
    pub n_occluders: u32,
}

impl Default for Bin {
    fn default() -> Self {
        Self {
            occluders: [default(); N_OCCLUDERS],
            n_occluders: 0,
        }
    }
}

#[derive(Clone, ShaderType)]
pub struct BinCounts {
    pub counts: [u32; N_BINS],
}

impl Default for BinCounts {
    fn default() -> Self {
        Self {
            counts: [0; N_BINS],
        }
    }
}

impl BinBuffer {
    const PI2: f32 = PI * 2.0;
    const N_BINS: f32 = N_BINS as f32;

    /// Get the binding of the bins. It is guaranteed to exist.
    pub fn bin_binding(&self) -> BindingResource<'_> {
        self.buffer.binding().unwrap()
    }

    /// Get the binding of the number of each bin. It is guaranteed to exist.
    pub fn bin_count_binding(&self) -> BindingResource<'_> {
        self.bin_counts.binding().unwrap()
    }

    /// Write this buffer's data to the GPU. This function also sorts the
    /// occluders by distance enabling early-stopping in GPU checks.
    pub fn write(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        let values = self.buffer.values_mut();
        for bin in 0..N_BINS {
            for index in 0..values.len() {
                if values[index][bin].n_occluders == 0 {
                    break;
                }

                values[index][bin]
                    .occluders
                    .sort_unstable_by(|a, b| a.distance.total_cmp(&b.distance));
            }
        }

        self.buffer.write_buffer(device, queue);

        self.bin_counts.set(BinCounts {
            counts: self.counts,
        });
        self.bin_counts.write_buffer(device, queue);
    }

    fn push_empty(&mut self) {
        self.buffer.push([default(); N_BINS]);
    }

    /// Clear the buffer and add one empty set of bins.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.counts = [0; N_BINS];
        self.push_empty();
    }

    /// Add an occluder to this buffer.
    pub fn add_occluder(&mut self, occluder: OccluderPointer, min_angle: f32, max_angle: f32) {
        let min_bin = (((min_angle + PI) / Self::PI2) * Self::N_BINS).floor() as i32;
        let max_bin = (((max_angle + PI) / Self::PI2) * Self::N_BINS).floor() as i32;

        let mut bin_index = min_bin;

        loop {
            let values = self.buffer.values_mut();

            let index = if bin_index < 0 {
                bin_index + N_BINS as i32
            } else if bin_index >= N_BINS as i32 {
                bin_index - N_BINS as i32
            } else {
                bin_index
            } as usize;

            while self.counts[index] >= values.len() as u32 {
                values.push([default(); N_BINS]);
            }

            let bin = &mut values[self.counts[index] as usize][index];

            bin.occluders[bin.n_occluders as usize] = occluder;
            bin.n_occluders += 1;

            // if bin.n_occluders > 1 {
            //     info!(
            //         "adding occluder of {min_angle} - {max_angle} to bin {bin_index} of {}. bin.n_occcluders: {}",
            //         self.counts[bin_index], bin.n_occluders
            //     );
            // }

            if bin.n_occluders == N_OCCLUDERS as u32 {
                self.counts[index] += 1;
            }

            if bin_index == max_bin {
                break;
            }

            bin_index += 1;
        }
    }
}

/// Compact struct pointing to a round occluder, or a chain of vertices from a polygonal occluder.  
#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, ShaderType)]
pub struct OccluderPointer {
    /// The index's first bit is the type of occluder: 0 for round, 1 for polygonal.
    ///
    /// In the case of polygonal occluders, there is additional data positioned on the consecutive bits:
    ///
    /// - A `term` variable that takes 2 bits, describing the terminator format of this chain. This is 1
    /// if the chain ends looping over the atan2 seam, 2 if it starts like that, 3 if the chain starts or ends at the
    /// limit of the occlusion edge but could've looped over, and 0 otherwise.
    ///
    /// - A `rev` variable that takes 1 bit and specifies if the chain is made of vertices in the same order as they're
    /// stored in (clockwise) or not. This is used for when a light is inside the perimeter of an occluder and the
    /// edges need to be reversed.
    pub index: u32,
    /// Only used for polygonal occluders, the index of the first vertex of the chain in the global vertex buffer.
    pub min_v: u32,
    /// Only used for polygonal occluders, the length of the vertex chain.
    pub length: u32,
    /// The minimum distance from the occluder to the light source. This is used to accelerate GPU computations,
    /// because a point can't be blocked by this occluder if it's distance is greater than the point's own
    /// distance to the light soruce.
    pub distance: f32,
}

impl Default for OccluderPointer {
    fn default() -> Self {
        Self {
            index: 0,
            min_v: 0,
            length: 0,
            distance: f32::MAX,
        }
    }
}

/// A global buffer in which all visible vertices are stored.
///
/// This is different from the [`BufferManager`] in order to use a specific allocation
/// that suits vertices better. They are quickly added on top of each other without keeping track
/// of their position for re-allocation. When an occluder disappears, it's number of vertices is simply
/// subtracted from the total lenght of the buffer, and the buffer refragments itself when
/// there is a significant amount of wasted space.  
#[derive(Resource)]
pub struct VertexBuffer {
    vertices: RawBufferVec<Vec2>,
    next_index: usize,
    empty_slots: u32,
    current_generation: u32,
}

impl FromWorld for VertexBuffer {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();

        Self::new(device, queue)
    }
}

impl VertexBuffer {
    fn new(device: &RenderDevice, queue: &RenderQueue) -> Self {
        let mut res = Self {
            vertices: RawBufferVec::<Vec2>::new(BufferUsages::STORAGE),
            next_index: 1,
            empty_slots: 0,
            current_generation: 0,
        };

        res.vertices.set_label("vertex buffer".into());

        // empty value is added so the buffer can be written to VRAM from the start

        res.vertices.push(default());
        res.vertices.write_buffer(device, queue);

        res
    }

    /// Get the binding of this buffer. It is guaranteed to exist.
    pub fn binding(&self) -> BindingResource<'_> {
        self.vertices.binding().unwrap()
    }

    /// Insert all of an occluder's vertices to this buffer. This
    /// function also automatically writes them to the GPU.  
    pub fn write_vertices(
        &mut self,
        occluder: &ExtractedOccluder,
        index: Option<BufferIndex>,
        device: &RenderDevice,
        queue: &RenderQueue,
        changed: bool,
    ) -> BufferIndex {
        if !changed
            && let Some(index) = index
            && index.generation == self.current_generation
        {
            return index;
        }

        let index = match index {
            None => self.next_index,
            Some(BufferIndex { index, generation }) => {
                if index < self.next_index && generation == self.current_generation {
                    index
                } else {
                    self.next_index
                }
            }
        };

        // change existent vertices
        if index < self.next_index {
            let mut last_index = index;
            for vertex in occluder.vertices_iter() {
                self.vertices.set(last_index as u32, vertex);
                last_index += 1;
            }

            self.vertices
                .write_buffer_range(queue, index..last_index)
                .expect("couldn't write range");

            // for vertex in self.vertices.values() {
            //     info!("{vertex}");
            // }

            return BufferIndex {
                index,
                generation: self.current_generation,
            };
        }

        // add new vertices
        for vertex in occluder.vertices_iter() {
            self.vertices.push(vertex);
            self.next_index += 1;
        }

        // if self.next_index % 2 == 1 {
        //     self.vertices.push(default());
        //     self.next_index += 1;
        // }

        if self.next_index >= self.vertices.capacity() {
            self.vertices.reserve(
                (self.next_index as f32 / 4096.0).ceil() as usize * 4096,
                device,
            );
            self.vertices.write_buffer(device, queue);
        } else {
            self.vertices
                .write_buffer_range(queue, index..self.next_index)
                .expect("couldn't write range");
        }

        // info!(
        //     "Vertex buffer capacity: {}, length: {}, empty slots: {}",
        //     self.vertices.capacity(),
        //     self.vertices.len(),
        //     self.empty_slots
        // );

        BufferIndex {
            index,
            generation: self.current_generation,
        }
    }

    /// Called at the end of a frame. Potentially triggers refragmentation.
    pub fn pass(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.empty_slots > 500 && self.empty_slots > self.vertices.capacity() as u32 / 2 {
            let old_generation = self.current_generation;
            *self = Self::new(device, queue);
            self.current_generation = old_generation + 1;
        }
    }

    /// Called by an occluder to subtract it's total number of vertices from the allocated space.
    pub fn free_indices(&mut self, n_indices: u32, generation: u32) {
        if generation != self.current_generation {
            return;
        }

        self.empty_slots += n_indices;
    }
}

/// An index given and returned to the various buffer structures.
///
/// This is used for storing an entity's slot in the buffer, and
/// contains a generation to keep track of buffer refragmentations.
#[derive(Clone, Copy)]
pub struct BufferIndex {
    pub index: usize,
    pub generation: u32,
}
