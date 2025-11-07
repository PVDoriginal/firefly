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
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            color: bevy::prelude::Color::Srgba(WHITE),
            intensity: 1.,
            range: 100.,
        }
    }
}

#[derive(Component, Default, Clone)]
pub(crate) struct ExtractedPointLight {
    pub pos: Vec2,
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
}

#[derive(Component, Default, Clone, ShaderType)]
pub(crate) struct UniformPointLight {
    pub pos: Vec2,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
}

#[derive(Resource, Default)]
pub(crate) struct LightSet(pub Vec<UniformBuffer<UniformPointLight>>);

#[derive(Clone, Reflect)]
pub struct LightColor {
    pub color: Color,
    pub intensity: f32,
}
