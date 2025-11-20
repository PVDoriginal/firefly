use bevy::{
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
};
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
    pub ambient_color: Color,
    /// Brightness for the ambient light. If 0 and no lights are present, everything will be completely black.
    pub ambient_brightness: f32,
    /// Whether you want to use light bands or not.
    ///
    /// Light bands divide the lightmap's texture into a certain number of bands, creating a stylized look.
    pub light_bands: Option<u32>,
    /// Whether you want to use soft shadows or not.
    ///
    /// Softness corresponds to the angle that the soft shadows are stretched over. Should be between 0 and 1 (corresponding to 0 to 90 degress).
    pub softness: Option<f32>,
}

#[derive(ShaderType, Clone, Default)]
pub(crate) struct UniformFireflyConfig {
    pub ambient_color: Vec3,
    pub ambient_brightness: f32,
    pub light_bands: u32,
    pub softness: f32,
}
