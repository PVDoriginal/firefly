use bevy::{
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
};
#[derive(Component, Default, Clone, ExtractComponent, Reflect)]
pub(crate) struct ExtractedWorldData {
    pub camera_pos: Vec2,
}

#[derive(Component, ExtractComponent, Clone, Reflect)]
pub struct FireflyConfig {
    pub ambient_color: Color,
    pub ambient_brightness: f32,
    pub light_bands: Option<u32>,
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformFireflyConfig {
    pub ambient_color: Vec3,
    pub ambient_brightness: f32,
    pub light_bands: u32,
}
