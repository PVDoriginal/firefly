use bevy::{
    color::palettes::css::WHITE,
    prelude::*,
    render::{
        render_resource::{ShaderType, UniformBuffer},
        sync_world::SyncToRenderWorld,
    },
};

#[derive(Component, Clone, Reflect)]
#[require(SyncToRenderWorld)]
pub struct PointLight;

#[derive(Component, Default, Clone, Copy, ShaderType)]
pub(crate) struct ExtractedPointLight {
    pub pos: Vec2,
}

#[derive(Resource, Default)]
pub(crate) struct LightSet(pub Vec<UniformBuffer<ExtractedPointLight>>);

#[derive(Clone, Reflect)]
pub struct LightColor {
    pub color: Color,
    pub intensity: f32,
}

impl Default for LightColor {
    fn default() -> Self {
        Self {
            color: bevy::prelude::Color::Srgba(WHITE),
            intensity: 1.,
        }
    }
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformLightColor {
    pub color: Vec4,
    pub intensity: f32,
}
