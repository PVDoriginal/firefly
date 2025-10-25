use bevy::{
    color::palettes::css::WHITE,
    prelude::*,
    render::{
        sync_world::SyncToRenderWorld,
        view::{VisibilityClass, visibility},
    },
};

#[derive(Component, Reflect)]
#[require(SyncToRenderWorld, VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<PointLight>)]
pub struct PointLight;

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
