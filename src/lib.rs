use bevy::{
    asset::weak_handle,
    prelude::*,
    render::{render_graph::RenderLabel, texture::CachedTexture},
};

pub mod app;
pub mod data;
pub mod lights;
pub mod occluders;

mod extract;
mod nodes;
mod pipelines;
mod prepare;
mod sprites;

pub mod prelude {
    pub use crate::app::{FireflyGizmosPlugin, FireflyPlugin};
    pub use crate::data::FireflyConfig;
    pub use crate::lights::PointLight2d;
    pub use crate::occluders::Occluder2d;
    pub use crate::{ApplyLightmapLabel, CreateLightmapLabel};
}

#[derive(Component)]
struct LightMapTexture(pub CachedTexture);

#[derive(Component)]
struct IntermediaryLightMapTexture(pub CachedTexture);

#[derive(Component)]
struct EmptyLightMapTexture(pub CachedTexture);

/// Render graph label for creating the lightmap.
///
/// Useful if you want to add your own render passes before / after it.   
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CreateLightmapLabel;

/// Render graph label for when the lightmap is applied over the camera view.
///
/// Useful if you want to add your own render passes before / after it.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ApplyLightmapLabel;

const CREATE_LIGHTMAP_SHADER: Handle<Shader> = weak_handle!("6e9647ff-b9f8-41ce-8d83-9bd91ae31898");
const APPLY_LIGHTMAP_SHADER: Handle<Shader> = weak_handle!("72c4f582-83b6-47b6-a200-b9f0e408df72");
const TRANSFER_SHADER: Handle<Shader> = weak_handle!("206fb81e-58e7-4dd0-b4f5-c39892e23fc6");
const TYPES_SHADER: Handle<Shader> = weak_handle!("dac0fb7e-a64f-4923-8e31-6912f3fc8551");
const UTILS_SHADER: Handle<Shader> = weak_handle!("1471f256-f404-4388-bb2f-ca6b8047ef7e");
const SPRITE_SHADER: Handle<Shader> = weak_handle!("00f40f01-5069-4f1c-a69c-a6bd5ca3983e");
