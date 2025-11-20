use bevy::{
    color::palettes::css::WHITE,
    prelude::*,
    render::{
        render_resource::{ShaderType, UniformBuffer},
        sync_world::SyncToRenderWorld,
    },
};

/// Point light with adjustable fields.
#[derive(Component, Clone, Reflect)]
#[require(SyncToRenderWorld)]
pub struct PointLight2d {
    /// Color of the point light. Alpha is ignored.
    pub color: Color,
    /// Intensity of the point light.
    pub intensity: f32,
    /// Range of the point light.
    pub range: f32,
}

impl Default for PointLight2d {
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
    pub z: f32,
}

impl PartialEq for ExtractedPointLight {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos && self.range == other.range
    }
}

#[derive(Component, Default, Clone, ShaderType)]
pub(crate) struct UniformPointLight {
    pub pos: Vec2,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
    pub z: f32,
}

#[derive(Resource, Default)]
pub(crate) struct LightSet(pub Vec<UniformBuffer<UniformPointLight>>);
