use std::{collections::VecDeque, ops::Range};

use bevy::{
    camera::visibility::{VisibilityClass, add_visibility_class},
    color::palettes::css::WHITE,
    ecs::{
        component::Tick,
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
        render_resource::{BindGroup, BufferVec, GpuArrayBuffer, ShaderType, UniformBuffer},
        sync_world::SyncToRenderWorld,
        view::{ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewUniformOffset},
    },
};

use crate::{
    LightBatchSetKey,
    occluders::{UniformOccluder, UniformVertex},
    phases::LightmapPhase,
    pipelines::LightmapCreationPipeline,
};

/// Point light with adjustable fields.
#[derive(Component, Clone, Reflect)]
#[require(
    SyncToRenderWorld,
    Transform,
    VisibilityClass,
    ViewVisibility,
    LightHeight
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
            angle: 360.0,
            cast_shadows: true,
            offset: Vec3::ZERO,
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
    pub n_rounds: u32,
    pub n_poly: u32,
}

#[derive(Component)]
pub(crate) struct LightBuffers {
    pub light: UniformBuffer<UniformPointLight>,
    pub occluders: BufferVec<PolyOccluderPointer>,
    pub rounds: BufferVec<u32>,
}

#[derive(ShaderType, Clone, Copy, Default)]
pub(crate) struct PolyOccluderPointer {
    pub index: u32,
    pub min_v: u32,
    pub length: u32,
    pub term: u32,
}

/// This resource handles giving lights indices and redistributing unused indices
#[derive(Resource, Default)]
pub(crate) struct LightIndices {
    next_index: u32,
    discarded: VecDeque<u32>,
}

impl LightIndices {
    pub fn take_index(&mut self) -> u32 {
        let index = match self.discarded.pop_back() {
            Some(index) => index,
            None => self.next_index,
        };

        self.next_index += 1;
        index
    }

    pub fn return_index(&mut self, index: u32) {
        self.discarded.push_front(index);
    }
}

#[derive(Component)]
#[require(SyncToRenderWorld)]
pub(crate) struct LightIndex(pub u32);

pub(crate) struct LightPlugin;
impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightIndices>();

        app.add_systems(Update, assign_light_indices);
        app.add_observer(discard_light_index);

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

fn assign_light_indices(
    lights: Populated<Entity, Added<PointLight2d>>,
    mut indices: ResMut<LightIndices>,
    mut commands: Commands,
) {
    for light in lights {
        commands
            .entity(light)
            .insert(LightIndex(indices.take_index()));
    }
}

fn discard_light_index(
    trigger: On<Remove, PointLight2d>,
    lights: Query<&LightIndex>,
    mut indices: ResMut<LightIndices>,
) {
    if let Ok(light) = lights.get(trigger.entity) {
        indices.return_index(light.0);
        return;
    }
    warn!("Can't find light index for entity: {}", trigger.entity);
}

#[derive(Resource, Deref, DerefMut, Default)]
pub(crate) struct LightBatches(pub HashMap<(RetainedViewEntity, Entity), LightBatch>);

#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) struct LightBatch {
    pub id: Entity,
    pub range: Range<u32>,
}

#[derive(Resource, Default)]
pub(crate) struct LightBindGroups {
    pub values: HashMap<Entity, BindGroup>,
}

fn queue_lights(
    light_draw_functions: Res<DrawFunctions<LightmapPhase>>,
    lightmap_pipeline: Res<LightmapCreationPipeline>,
    mut lightmap_phases: ResMut<ViewBinnedRenderPhases<LightmapPhase>>,
    views: Query<(&ExtractedView, &RenderVisibleEntities)>,
) {
    let draw_lightmap_function = light_draw_functions.read().id::<DrawLightmap>();

    for (view, visible_entities) in &views {
        let Some(lightmap_phase) = lightmap_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        for (render_entity, visible_entity) in visible_entities.iter::<PointLight2d>() {
            let batch_set_key = LightBatchSetKey {
                pipeline: lightmap_pipeline.pipeline_id,
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

pub(crate) type DrawLightmap = (SetItemPipeline, SetLightTextureBindGroup<0>, DrawLightBatch);

pub(crate) struct SetLightTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetLightTextureBindGroup<I> {
    type Param = (SRes<LightBindGroups>, SRes<LightBatches>);
    type ViewQuery = (Read<ExtractedView>, Read<ViewUniformOffset>);
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        (view, view_uniform_offset): ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        (image_bind_groups, batches): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();
        let Some(batch) = batches.get(&(view.retained_view_entity, item.entity())) else {
            return RenderCommandResult::Skip;
        };

        pass.set_bind_group(
            I,
            image_bind_groups.values.get(&batch.id).unwrap(),
            &[view_uniform_offset.offset],
        );
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
