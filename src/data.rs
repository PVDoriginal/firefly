use bevy::{
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
};

use crate::lights::{LightColor, UniformLightColor};

#[derive(Default, Clone, Copy, ShaderType)]
pub(crate) struct UniformMeta {
    pub n_occluders: u32,
}

#[derive(Component, ExtractComponent, Clone, Reflect)]
pub struct FireflyConfig {
    pub global_light: LightColor,
    pub light_bands: Option<u32>,
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformFireflyConfig {
    pub global_light: UniformLightColor,
    pub light_bands: u32,
}
