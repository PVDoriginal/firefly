use std::{collections::VecDeque, time::Duration, usize};

use bevy::{
    color::palettes::css::WHITE,
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        extract_resource::ExtractResource,
        render_resource::{
            BindingResource, Buffer, BufferBinding, BufferDescriptor, BufferUsages, BufferVec,
            RawBufferVec, ShaderType, encase::private::WriteInto,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use bytemuck::NoUninit;

use crate::occluders::OccluderIndex;
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
