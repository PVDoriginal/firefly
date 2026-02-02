use bevy::{
    camera::visibility::{VisibilityClass, add_visibility_class},
    color::palettes::css::WHITE,
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    ecs::{
        change_detection::Tick,
        query::ROQueryItem,
        system::{
            SystemParamItem,
            lifetimeless::{Read, SRes},
        },
    },
    platform::collections::HashMap,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        batching::sort_binned_render_phase,
        render_phase::{
            AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
            RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
            ViewBinnedRenderPhases,
        },
        render_resource::{
            BindGroup, PipelineCache, ShaderType, SpecializedRenderPipelines, StorageBuffer,
        },
        sync_world::SyncToRenderWorld,
        view::{ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewUniformOffset},
    },
};
use bytemuck::NoUninit;

use crate::{
    LightBatchSetKey,
    buffers::{BinBuffer, BufferIndex},
    change::Changes,
    phases::LightmapPhase,
    pipelines::{LightPipelineKey, LightmapCreationPipeline},
    visibility::VisibilityTimer,
};

/// Point light with adjustable fields.
#[derive(Component, Clone, Reflect)]
#[require(
    SyncToRenderWorld,
    Transform,
    VisibilityClass,
    ViewVisibility,
    VisibilityTimer,
    LightHeight,
    Changes
)]
#[component(on_add = add_visibility_class::<PointLight2d>)]
pub struct PointLight2d {
    /// Color of the point light. Alpha is ignored.
    ///
    /// **Default:** White.
    pub color: Color,

    /// Intensity of the point light.
    ///
    /// **Default:** 1.
    pub intensity: f32,

    /// Outer range of the point light.
    pub range: f32,

    /// Inner range of the point light. Should be less than the normal range.
    ///
    /// The light will have no falloff (full intensity) within this range.
    ///
    /// **Default:** 0.
    pub inner_range: f32,

    /// Type of falloff for this light.
    ///
    /// **Default:** [InverseSquare](Falloff::InverseSquare).
    pub falloff: Falloff,

    /// The intensity of this light's falloff effect.
    ///
    /// **Default:** 1.
    pub falloff_intensity: f32,

    /// Angle in degrees of the point light. Between 0 and 360.
    ///
    /// 0 - No light;
    /// 360 - Full light going in all direction.
    ///
    /// Relative to the direction the entity's facing.
    ///
    /// **Default:** 360.
    pub angle: f32,

    /// Whether this light should cast shadows or not with the existent occluders.
    ///
    /// **Performance Impact:** Major.
    ///
    /// **Default:** true.
    pub cast_shadows: bool,

    /// Offset position of the light.
    ///
    /// Useful if you want to add a light component on an entity and change it's position,
    /// without needing to create a child entity for it.
    ///
    /// **Default:** [Vec3::ZERO].
    pub offset: Vec3,
}

/// Optional component you can add to lights.
///
/// Describes the light's 2d height, useful for emulating 3d lighting in top-down 2d games.
///
/// This is currently used along with the normal maps.
///
/// **Default:** 0.   
#[derive(Component, Default, Reflect)]
pub struct LightHeight(pub f32);

/// An enum for the falloff type of a light.
///
/// **Default:** [InverseSquare](Falloff::InverseSquare).  
#[derive(Clone, Copy, Reflect)]
pub enum Falloff {
    /// The intensity decreases inversely proportial to the square distance towards the inner light source.  
    InverseSquare,
    /// The intensity decreases linearly with the distance towards the inner light source.
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
            falloff_intensity: 0.0,
            angle: 360.0,
            cast_shadows: true,
            offset: Vec3::ZERO,
        }
    }
}

/// The data that is extracted to the render world from a [`PointLight2d`].
#[derive(Component, Clone)]
#[require(BinBuffer, LightIndex, LightPointer)]
pub struct ExtractedPointLight {
    pub pos: Vec2,
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub inner_range: f32,
    pub falloff: Falloff,
    pub falloff_intensity: f32,
    pub angle: f32,
    pub cast_shadows: bool,
    pub dir: Vec2,
    pub z: f32,
    pub height: f32,
    pub changes: Changes,
}

impl PartialEq for ExtractedPointLight {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos && self.range == other.range
    }
}

/// Data that is sent to the GPU for each visible [`PointLight2d`].
#[repr(C)]
#[derive(Default, Clone, Copy, ShaderType, NoUninit)]
pub struct UniformPointLight {
    pub pos: Vec2,
    pub intensity: f32,
    pub range: f32,

    pub color: Vec4,
    pub inner_range: f32,
    pub falloff: u32,
    pub falloff_intensity: f32,
    pub angle: f32,

    pub dir: Vec2,

    pub z: f32,
    pub height: f32,
}

/// Render World component that contains the buffer a [`PointLight2d`] writes to each frame.   
#[derive(Component, Default)]
pub struct LightPointer(pub StorageBuffer<u32>);

/// Plugin responsible for functionality related to lights. Added automatically
/// by [`FireflyPlugin`](crate::prelude::FireflyPlugin).
pub struct LightPlugin;
impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<LightBindGroups>();
            render_app.init_resource::<DrawFunctions<LightmapPhase>>();
            render_app.init_resource::<ViewBinnedRenderPhases<LightmapPhase>>();

            render_app.add_render_command::<LightmapPhase, DrawLightmap>();

            render_app.add_systems(
                Render,
                sort_binned_render_phase::<LightmapPhase>.in_set(RenderSystems::PhaseSort),
            );

            render_app.add_systems(Render, queue_lights.in_set(RenderSystems::Queue));
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<LightBatches>();
        }
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub(crate) struct LightBatches(pub HashMap<(RetainedViewEntity, Entity), LightBatch>);

#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) struct LightBatch {
    pub id: Entity,
}

#[derive(Resource, Default)]
pub(crate) struct LightBindGroups {
    pub values: HashMap<Entity, BindGroup>,
}

#[derive(Component)]
pub(crate) struct LightLut(pub BindGroup);

fn queue_lights(
    light_draw_functions: Res<DrawFunctions<LightmapPhase>>,
    pipeline: Res<LightmapCreationPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LightmapCreationPipeline>>,
    mut lightmap_phases: ResMut<ViewBinnedRenderPhases<LightmapPhase>>,
    views: Query<(
        &ExtractedView,
        &RenderVisibleEntities,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
    pipeline_cache: Res<PipelineCache>,
) {
    let draw_lightmap_function = light_draw_functions.read().id::<DrawLightmap>();

    for (view, visible_entities, msaa, tonemapping, dither) in &views {
        let Some(lightmap_phase) = lightmap_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let msaa_key = LightPipelineKey::from_msaa_samples(msaa.samples());
        let mut view_key = LightPipelineKey::from_hdr(view.hdr) | msaa_key;

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= LightPipelineKey::TONEMAP_IN_SHADER;
                view_key |= match tonemapping {
                    Tonemapping::None => LightPipelineKey::TONEMAP_METHOD_NONE,
                    Tonemapping::Reinhard => LightPipelineKey::TONEMAP_METHOD_REINHARD,
                    Tonemapping::ReinhardLuminance => {
                        LightPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE
                    }
                    Tonemapping::AcesFitted => LightPipelineKey::TONEMAP_METHOD_ACES_FITTED,
                    Tonemapping::AgX => LightPipelineKey::TONEMAP_METHOD_AGX,
                    Tonemapping::SomewhatBoringDisplayTransform => {
                        LightPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
                    }
                    Tonemapping::TonyMcMapface => LightPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
                    Tonemapping::BlenderFilmic => LightPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
                };
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= LightPipelineKey::DEBAND_DITHER;
            }
        }

        let pipeline = pipelines.specialize(&pipeline_cache, &pipeline, view_key);

        for (render_entity, visible_entity) in visible_entities.iter::<PointLight2d>() {
            let batch_set_key = LightBatchSetKey {
                pipeline: pipeline,
                draw_function: draw_lightmap_function,
            };

            lightmap_phase.add(
                batch_set_key,
                (),
                (*render_entity, *visible_entity),
                InputUniformIndex::default(),
                BinnedRenderPhaseType::NonMesh,
                Tick::new(10),
            );
        }
    }
}

pub(crate) type DrawLightmap = (SetItemPipeline, SetLightTextureBindGroup, DrawLightBatch);

pub(crate) struct SetLightTextureBindGroup;
impl<P: PhaseItem> RenderCommand<P> for SetLightTextureBindGroup {
    type Param = (SRes<LightBindGroups>, SRes<LightBatches>);
    type ViewQuery = (Read<ExtractedView>, Read<ViewUniformOffset>, Read<LightLut>);
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        (view, view_uniform_offset, lut): ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        (image_bind_groups, batches): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();
        let Some(batch) = batches.get(&(view.retained_view_entity, item.entity())) else {
            return RenderCommandResult::Skip;
        };

        pass.set_bind_group(0, &lut.0, &[view_uniform_offset.offset]);
        pass.set_bind_group(1, image_bind_groups.values.get(&batch.id).unwrap(), &[]);

        RenderCommandResult::Success
    }
}

pub(crate) struct DrawLightBatch;
impl<P: PhaseItem> RenderCommand<P> for DrawLightBatch {
    type Param = ();
    type ViewQuery = Read<ExtractedView>;
    type ItemQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        _: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.draw(0..3, 0..1);
        RenderCommandResult::Success
    }
}

/// Buffer index that each visible light gets assigned
/// corresponding to its [`BufferManager`](crate::buffers::BufferManager) slot.  
#[derive(Component, Default)]
pub struct LightIndex(pub Option<BufferIndex>);
