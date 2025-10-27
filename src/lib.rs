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

pub mod prelude {
    pub use crate::app::{FireflyGizmosPlugin, FireflyPlugin};
    pub use crate::data::FireflyConfig;
    pub use crate::lights::{LightColor, PointLight};
    pub use crate::occluders::Occluder;
    pub use crate::{ApplyLightmapLabel, CreateLightmapLabel};
}

#[derive(Component)]
struct LightMapTexture(pub CachedTexture);

#[derive(Component)]
struct IntermediaryLightMapTexture(pub CachedTexture);

#[derive(Component)]
struct EmptyLightMapTexture(pub CachedTexture);

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CreateLightmapLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ApplyLightmapLabel;

const CREATE_LIGHTMAP_SHADER: Handle<Shader> = weak_handle!("6e9647ff-b9f8-41ce-8d83-9bd91ae31898");
const APPLY_LIGHTMAP_SHADER: Handle<Shader> = weak_handle!("72c4f582-83b6-47b6-a200-b9f0e408df72");
const TRANSFER_SHADER: Handle<Shader> = weak_handle!("206fb81e-58e7-4dd0-b4f5-c39892e23fc6");
const TYPES_SHADER: Handle<Shader> = weak_handle!("dac0fb7e-a64f-4923-8e31-6912f3fc8551");
const UTILS_SHADER: Handle<Shader> = weak_handle!("1471f256-f404-4388-bb2f-ca6b8047ef7e");
