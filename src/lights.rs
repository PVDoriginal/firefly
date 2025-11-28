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
#[require(SyncToRenderWorld, Transform)]
pub struct PointLight2d {
    /// **Color** of the point light. **Alpha is ignored**.
    pub color: Color,

    /// **Intensity** of the point light.
    ///
    /// **Defaults to 1.**
    pub intensity: f32,

    /// **Outer range** of the point light.
    pub range: f32,

    /// **Inner range** of the point light. Should be **less than the normal range**.
    ///
    /// The light will have **no falloff** (full intensity) within this range.
    ///
    /// **Defaults to 0.**
    pub inner_range: f32,

    /// **Type of falloff** for this light.
    ///
    /// **Defaults to Inverse Square.**
    pub falloff: Falloff,

    /// **Angle in degrees** of the point light. **Between 0 and 360.**
    ///
    /// 0 - No light;
    /// 360 - Full light going in all direction.
    ///
    /// **Relative to the direction the entity's facing.**
    ///
    /// **Defaults to 360**.
    pub angle: f32,

    /// Whether this light should **cast shadows** or not with the existent **occluders**.
    ///
    /// **Defaults to true**
    pub cast_shadows: bool,

    /// **Height** fields that's used for certain kinds of normal mapping.
    ///
    /// **Should be non-negative**.  
    pub height: f32,
}

/// An enum for the **falloff type**.  
#[derive(Clone, Copy, Reflect)]
pub enum Falloff {
    /// The intensity decreases **inversely proportial to the square distance** towards the inner light source.  
    InverseSquare,
    /// The intensity decreases **linearly with the distance** towards the inner light source.
    Linear,
}

impl Default for PointLight2d {
    fn default() -> Self {
        Self {
            color: bevy::prelude::Color::Srgba(WHITE),
            intensity: 1.,
            range: 100.,
            inner_range: 0.,
            falloff: Falloff::InverseSquare,
            angle: 360.0,
            cast_shadows: true,
            height: 0.,
        }
    }
}

#[derive(Component, Clone)]
pub(crate) struct ExtractedPointLight {
    pub pos: Vec2,
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub inner_range: f32,
    pub falloff: Falloff,
    pub angle: f32,
    pub cast_shadows: bool,
    pub dir: Vec2,
    pub z: f32,
    pub height: f32,
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
    pub inner_range: f32,
    pub falloff: u32,
    pub angle: f32,
    pub dir: Vec2,
    pub z: f32,
    pub height: f32,
}

#[derive(Resource, Default)]
pub(crate) struct LightSet(pub Vec<UniformBuffer<UniformPointLight>>);
