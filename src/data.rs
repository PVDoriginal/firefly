use std::{collections::VecDeque, time::Duration};

use bevy::{
    color::palettes::css::WHITE,
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        render_resource::{
            BindingResource, Buffer, BufferDescriptor, BufferUsages, BufferVec, ShaderType,
            encase::private::WriteInto,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use bytemuck::NoUninit;
#[derive(Component, Default, Clone, ExtractComponent, Reflect)]
pub(crate) struct ExtractedWorldData {
    pub camera_pos: Vec2,
}

/// Component that needs to be added to a camera in order to have it render lights.
///
/// # Panics
/// Panics if added to multiple cameras at once.
#[derive(Component, ExtractComponent, Clone, Reflect)]
pub struct FireflyConfig {
    /// Ambient light that will be added over all other lights.  
    ///
    /// **Default:** White.
    pub ambient_color: Color,

    /// Brightness for the ambient light. If 0 and no lights are present, everything will be completely black.
    ///
    /// **Default:** 0.
    pub ambient_brightness: f32,

    /// Whether you want to use light bands or not.
    ///
    /// Light bands divide the lightmap's texture into a certain number of bands, creating a stylized look.
    ///
    /// **Performance Impact:** None.
    ///
    /// **Default:** None.
    pub light_bands: Option<u32>,

    /// Whether you want to use soft shadows or not.
    ///
    /// Softness corresponds to the angle that the soft shadows are stretched over. Should be between 0 and 1 (corresponding to 0 to 90 degress).
    ///
    /// **Performance Impact:** Minor.
    ///
    /// **Default:** Some(0.7).
    pub softness: Option<f32>,

    /// Whether to use occlusion z-sorting or not.
    ///
    /// If this is enabled, shadows cast by occluders won't affect sprites with a higher z position.
    ///
    /// Very useful for top-down games.
    ///
    /// **Performance Impact:** None.
    ///
    /// **Default:** true.
    pub z_sorting: bool,

    /// Field that controls how the normal maps are applied relative to perspective.
    ///
    /// **Performance Impact:** Very minor.
    ///
    /// **Default:** [None](NormalMapMode::None).
    pub normal_mode: NormalMode,

    /// This will control how much the normal map is attenuated before being applied.
    ///
    /// Inside the shader, we perform `mix(normal_map, vec3f(0), attenuation)` to decrease the 'hardness' of the normal map.
    ///
    /// This has the effect of pulling all channels towards (128, 128, 128), making the overall lighting over the surface more plain.
    ///
    /// **Default:** 0.5.
    pub normal_attenuation: f32,
}

/// Options for how the normal maps should be read and used.
///
/// In order to fully use normal maps, you will need to add the [NormalMap](crate::prelude::NormalMap) component to Sprites.
///
/// **Default:** [None](NormalMapMode::None).
#[derive(Clone, Reflect)]
pub enum NormalMode {
    /// No normal maps will be used in rendering.
    None,

    /// This will make it the normal mapping simply be based on the (x, y, z) difference between each light and sprite.
    ///
    /// [LightHeight](crate::prelude::LightHeight) and [SpriteHeight](crate::prelude::SpriteHeight) will be completely ignored.
    ///
    /// This is recommended for classic 2d perspectives, such as those of side-scroller games.   
    Simple,

    /// This will make the normal mapping be based on the difference between the light's and sprite's x-axis and z-axis, but for the y-axis
    /// it will use the [LightHeight](crate::prelude::LightHeight) and [SpriteHeight](crate::prelude::SpriteHeight) components.
    ///
    /// This is recommended for 2d perspectives where you want to simulate 3d lighting, such as top-down games.
    TopDown,
}

impl Default for FireflyConfig {
    fn default() -> Self {
        Self {
            ambient_color: Color::Srgba(WHITE),
            ambient_brightness: 0.0,
            light_bands: None,
            softness: Some(0.7),
            z_sorting: true,
            normal_mode: NormalMode::None,
            normal_attenuation: 0.5,
        }
    }
}

#[derive(ShaderType, Clone)]
pub(crate) struct UniformFireflyConfig {
    pub ambient_color: Vec3,
    pub ambient_brightness: f32,
    pub light_bands: u32,
    pub softness: f32,
    pub z_sorting: u32,
    pub normal_mode: u32,
    pub normal_attenuation: f32,
}

const BUFFER_TIMEOUT: f32 = 0.2;

/// This Resource handles when and where entities go in the buffer that's then passed to the GPU.
/// It gives and frees indicies, and decides when to refragment the whole buffer.
///
/// This is used by lights and occluders.
#[derive(Resource)]
pub struct BufferManager<T: ShaderType + WriteInto + Default + NoUninit> {
    buffer: BufferVec<T>,
    timeouts: Vec<Timer>,
    next_index: usize,
    free_indices: VecDeque<usize>,
    write_min: usize,
    write_max: usize,
    rewrite: bool,
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
        self.free_indices
            .pop_back()
            .unwrap_or_else(|| self.next_index)
    }

    fn new(device: &RenderDevice, queue: &RenderQueue) -> Self {
        let mut res = Self {
            buffer: BufferVec::<T>::new(BufferUsages::STORAGE),
            timeouts: default(),
            next_index: default(),
            free_indices: default(),
            write_min: default(),
            write_max: default(),
            rewrite: false,
        };

        // empty value is added so the buffer can be written to VRAM from the start
        res.buffer.push(default());
        res.buffer.write_buffer(device, queue);

        res
    }

    /// Called by an entity to pass it's value to the buffer and get back it's index.
    ///
    /// Value can be None, meaning the entity is still active but it's data didn't change.
    ///
    /// Index can be None, meaning the entity didn't have an index already assigned.
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

        if index < self.next_index {
            self.timeouts[index as usize] = Timer::from_seconds(BUFFER_TIMEOUT, TimerMode::Once);

            let mut view = self.buffer.buffer().unwrap().get_mapped_range_mut(
                (index + 1) as u64 * T::min_size().get()..(index + 2) as u64 * T::min_size().get(),
            );

            for chunk in view.chunks_exact_mut(T::min_size().get() as usize) {
                chunk.clone_from_slice(bytemuck::bytes_of(value));
            }
        } else {
            self.next_index += 1;
            self.buffer.push(*value);
            self.timeouts
                .push(Timer::from_seconds(BUFFER_TIMEOUT, TimerMode::Once));
            self.rewrite = true;
        }

        self.write_min = self.write_min.min(index + 1);
        self.write_min = self.write_max.max(index + 1);

        index
    }

    /// Flush the changes at the end of a render frame.
    ///
    /// This times out entities that haven't been active in a while, and efficiently passes all current changes to the GPU.
    pub fn flush(&mut self, delta: Duration, device: &RenderDevice, queue: &RenderQueue) {
        if self.rewrite {
            self.buffer.write_buffer(device, queue);
            self.rewrite = false;
        } else {
            self.buffer
                .write_buffer_range(queue, self.write_min as usize..self.write_max as usize + 1)
                .expect("couldn't write to buffer");

            for (i, timeout) in self.timeouts.iter_mut().enumerate() {
                if timeout.tick(delta).just_finished() {
                    self.free_indices.push_front(i);
                }
            }

            // Refragmentation. Because of wasted space the buffer will empty itself and pass all-new data next frame. This can be optimized.
            if self.free_indices.len() > self.next_index as usize / 2 {
                *self = Self::new(device, queue);
            }
        }
    }
}
