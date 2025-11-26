use std::ops::Range;

use crate::phases::{NormalPhase, Stencil2d};
use crate::pipelines::{SpriteNormalMapsPipeline, SpriteStencilPipeline};
use crate::utils::{compute_slices_on_asset_event, compute_slices_on_sprite_change};

use bevy::render::RenderDebugFlags;
use bevy::{
    asset::AssetEvents,
    core_pipeline::{
        core_2d::{AlphaMask2d, Opaque2d},
        tonemapping::{DebandDither, Tonemapping},
    },
    ecs::{
        prelude::*,
        query::ROQueryItem,
        system::{SystemParamItem, lifetimeless::*},
    },
    math::{Affine3A, FloatOrd},
    platform::collections::HashMap,
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        batching::sort_binned_render_phase,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, SortedRenderPhasePlugin, TrackedRenderPass,
            ViewSortedRenderPhases, sort_phase_system,
        },
        render_resource::*,
        view::{ExtractedView, Msaa, RenderVisibleEntities, RetainedViewEntity, ViewUniformOffset},
    },
};

use bevy::sprite::{Mesh2dPipeline, SpritePipelineKey, SpriteSystem, queue_material2d_meshes};

use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;

pub(crate) struct ExtractedSlice {
    pub offset: Vec2,
    pub rect: Rect,
    pub size: Vec2,
}

pub(crate) struct ExtractedSprite {
    pub main_entity: Entity,
    pub render_entity: Entity,
    pub transform: GlobalTransform,
    /// Change the on-screen size of the sprite
    /// Asset ID of the [`Image`] of this sprite
    /// PERF: storing an `AssetId` instead of `Handle<Image>` enables some optimizations (`ExtractedSprite` becomes `Copy` and doesn't need to be dropped)
    pub image_handle_id: AssetId<Image>,
    pub normal_handle_id: Option<AssetId<Image>>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub kind: ExtractedSpriteKind,
    pub id: f32,
}

pub(crate) enum ExtractedSpriteKind {
    /// A single sprite with custom sizing and scaling options
    Single {
        anchor: Vec2,
        rect: Option<Rect>,
        scaling_mode: Option<ScalingMode>,
        custom_size: Option<Vec2>,
    },
    /// Indexes into the list of [`ExtractedSlice`]s stored in the [`ExtractedSlices`] resource
    /// Used for elements composed from multiple sprites such as text or nine-patched borders
    Slices { indices: Range<usize> },
}

#[derive(Resource, Default)]
pub(crate) struct ExtractedSprites {
    //pub sprites: HashMap<(Entity, MainEntity), ExtractedSprite>,
    pub sprites: Vec<ExtractedSprite>,
}

#[derive(Resource, Default)]
pub(crate) struct ExtractedSlices {
    pub slices: Vec<ExtractedSlice>,
}

#[derive(Resource, Default)]
pub(crate) struct SpriteAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct SpriteInstance {
    // Affine 4x3 transposed to 3x4
    pub i_model_transpose: [Vec4; 3],
    pub id: f32,
    pub i_uv_offset_scale: [f32; 4],
    pub z: f32,
    pub _padding: [f32; 2],
}

impl SpriteInstance {
    #[inline]
    pub fn from(transform: &Affine3A, uv_offset_scale: &Vec4, id: f32, z: f32) -> Self {
        let transpose_model_3x3 = transform.matrix3.transpose();
        Self {
            i_model_transpose: [
                transpose_model_3x3.x_axis.extend(transform.translation.x),
                transpose_model_3x3.y_axis.extend(transform.translation.y),
                transpose_model_3x3.z_axis.extend(transform.translation.z),
            ],
            id,
            z,
            i_uv_offset_scale: uv_offset_scale.to_array(),
            _padding: [0., 0.],
        }
    }
}

#[derive(Resource)]
pub(crate) struct SpriteMeta {
    pub sprite_index_buffer: RawBufferVec<u32>,
    pub sprite_instance_buffer: RawBufferVec<SpriteInstance>,
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            sprite_index_buffer: RawBufferVec::<u32>::new(BufferUsages::INDEX),
            sprite_instance_buffer: RawBufferVec::<SpriteInstance>::new(BufferUsages::VERTEX),
        }
    }
}

#[derive(Component)]
pub(crate) struct SpriteViewBindGroup {
    pub value: BindGroup,
}

#[derive(Resource, Deref, DerefMut, Default)]
pub(crate) struct SpriteStencilBatches(pub HashMap<(RetainedViewEntity, Entity), SpriteBatch>);

#[derive(Resource, Deref, DerefMut, Default)]
pub(crate) struct SpriteNormalBatches(pub HashMap<(RetainedViewEntity, Entity), SpriteBatch>);

#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) struct SpriteBatch {
    pub image_handle_id: AssetId<Image>,
    pub range: Range<u32>,
}

#[derive(Resource, Default)]
pub(crate) struct ImageBindGroups {
    pub values: HashMap<AssetId<Image>, BindGroup>,
}

/// **Component** you can add to an entity that also has a **Sprite**, containing the corresponding **sprite's normal map**.
///
/// The image *MUST* correspond **1:1** with the size and format of the sprite image.
/// E.g. if the sprite image is a sprite sheet, the normal map will also need to be a sprite sheet of **exactly the same dimensions, padding, etc.**
///
/// # Example
///
/// ```
/// commands.spawn((
///     Sprite::from_image(asset_server.load("some_sprite.png")),
///     NormalMap(asset_server.load("some_sprite_normal.png")),
/// ));
/// ```
/// See [Sprite] for more information on using sprites.
#[derive(Component)]
pub struct NormalMap(pub Handle<Image>);

pub(crate) struct SpritesPlugin;
impl Plugin for SpritesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SortedRenderPhasePlugin::<Stencil2d, Mesh2dPipeline>::new(
            RenderDebugFlags::default(),
        ));
        app.add_plugins(SortedRenderPhasePlugin::<NormalPhase, Mesh2dPipeline>::new(
            RenderDebugFlags::default(),
        ));

        app.add_systems(
            PostUpdate,
            ((
                compute_slices_on_asset_event.before(AssetEvents),
                compute_slices_on_sprite_change,
            )
                .in_set(SpriteSystem::ComputeSlices),),
        );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<SpecializedRenderPipelines<SpriteStencilPipeline>>()
                .init_resource::<SpecializedRenderPipelines<SpriteNormalMapsPipeline>>()
                .init_resource::<DrawFunctions<Stencil2d>>()
                .init_resource::<DrawFunctions<NormalPhase>>()
                .init_resource::<SpriteMeta>()
                .init_resource::<ExtractedSprites>()
                .init_resource::<ExtractedSlices>()
                .init_resource::<SpriteAssetEvents>()
                .add_render_command::<Stencil2d, DrawSpriteStencil>()
                .add_render_command::<NormalPhase, DrawSpriteNormal>()
                .init_resource::<ViewSortedRenderPhases<Stencil2d>>()
                .init_resource::<ViewSortedRenderPhases<NormalPhase>>()
                .add_systems(
                    Render,
                    (
                        sort_phase_system::<Stencil2d>.in_set(RenderSet::PhaseSort),
                        sort_phase_system::<NormalPhase>.in_set(RenderSet::PhaseSort),
                        queue_sprites
                            .in_set(RenderSet::Queue)
                            .ambiguous_with(queue_material2d_meshes::<ColorMaterial>),
                        sort_binned_render_phase::<Opaque2d>.in_set(RenderSet::PhaseSort),
                        sort_binned_render_phase::<AlphaMask2d>.in_set(RenderSet::PhaseSort),
                    ),
                );
        };
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<SpriteStencilBatches>()
                .init_resource::<SpriteNormalBatches>()
                .init_resource::<SpriteStencilPipeline>()
                .init_resource::<SpriteNormalMapsPipeline>();
        }
    }
}

fn queue_sprites(
    mut view_entities: Local<FixedBitSet>,

    stencil_draw_functions: Res<DrawFunctions<Stencil2d>>,
    normal_draw_functions: Res<DrawFunctions<NormalPhase>>,

    sprite_pipeline: Res<SpriteStencilPipeline>,
    normal_pipeline: Res<SpriteNormalMapsPipeline>,

    mut stencil_pipelines: ResMut<SpecializedRenderPipelines<SpriteStencilPipeline>>,
    mut normal_pipelines: ResMut<SpecializedRenderPipelines<SpriteNormalMapsPipeline>>,

    pipeline_cache: Res<PipelineCache>,
    extracted_sprites: Res<ExtractedSprites>,

    mut stencil_phases: ResMut<ViewSortedRenderPhases<Stencil2d>>,
    mut normal_phases: ResMut<ViewSortedRenderPhases<NormalPhase>>,

    mut views: Query<(
        &RenderVisibleEntities,
        &ExtractedView,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
) {
    let draw_stencil_function = stencil_draw_functions.read().id::<DrawSpriteStencil>();
    let draw_normal_function = normal_draw_functions.read().id::<DrawSpriteNormal>();

    for (visible_entities, view, msaa, tonemapping, dither) in &mut views {
        let Some(stencil_phase) = stencil_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(normal_phase) = normal_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let msaa_key = SpritePipelineKey::from_msaa_samples(msaa.samples());
        let mut view_key = SpritePipelineKey::from_hdr(view.hdr) | msaa_key;

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= SpritePipelineKey::TONEMAP_IN_SHADER;
                view_key |= match tonemapping {
                    Tonemapping::None => SpritePipelineKey::TONEMAP_METHOD_NONE,
                    Tonemapping::Reinhard => SpritePipelineKey::TONEMAP_METHOD_REINHARD,
                    Tonemapping::ReinhardLuminance => {
                        SpritePipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE
                    }
                    Tonemapping::AcesFitted => SpritePipelineKey::TONEMAP_METHOD_ACES_FITTED,
                    Tonemapping::AgX => SpritePipelineKey::TONEMAP_METHOD_AGX,
                    Tonemapping::SomewhatBoringDisplayTransform => {
                        SpritePipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
                    }
                    Tonemapping::TonyMcMapface => SpritePipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
                    Tonemapping::BlenderFilmic => SpritePipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
                };
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= SpritePipelineKey::DEBAND_DITHER;
            }
        }

        let stencil_pipeline =
            stencil_pipelines.specialize(&pipeline_cache, &sprite_pipeline, view_key);
        let normal_pipeline =
            normal_pipelines.specialize(&pipeline_cache, &normal_pipeline, view_key);

        view_entities.clear();
        view_entities.extend(
            visible_entities
                .iter::<Sprite>()
                .map(|(_, e)| e.index() as usize),
        );

        stencil_phase.items.reserve(extracted_sprites.sprites.len());
        normal_phase.items.reserve(extracted_sprites.sprites.len());

        for (index, extracted_sprite) in extracted_sprites.sprites.iter().enumerate() {
            let view_index = extracted_sprite.main_entity.index();

            if !view_entities.contains(view_index as usize) {
                continue;
            }

            // These items will be sorted by depth with other phase items
            let sort_key = FloatOrd(extracted_sprite.transform.translation().z);

            // Add the item to the render phase
            stencil_phase.add(Stencil2d {
                draw_function: draw_stencil_function,
                pipeline: stencil_pipeline,
                entity: (
                    extracted_sprite.render_entity,
                    extracted_sprite.main_entity.into(),
                ),
                sort_key,
                // `batch_range` is calculated in `prepare_sprite_image_bind_groups`
                batch_range: 0..0,
                extra_index: PhaseItemExtraIndex::None,
                extracted_index: index,
                indexed: true,
            });

            if extracted_sprite.normal_handle_id.is_some() {
                normal_phase.add(NormalPhase {
                    draw_function: draw_normal_function,
                    pipeline: normal_pipeline,
                    entity: (
                        extracted_sprite.render_entity,
                        extracted_sprite.main_entity.into(),
                    ),
                    sort_key,
                    // `batch_range` is calculated in `prepare_sprite_image_bind_groups`z
                    batch_range: 0..0,
                    extra_index: PhaseItemExtraIndex::None,
                    extracted_index: index,
                    indexed: true,
                });
            }
        }
    }
}

pub(crate) type DrawSpriteStencil = (
    SetItemPipeline,
    SetSpriteViewBindGroup<0>,
    SetSpriteStencilTextureBindGroup<1>,
    DrawSpriteStencilBatch,
);

pub(crate) type DrawSpriteNormal = (
    SetItemPipeline,
    SetSpriteViewBindGroup<0>,
    SetSpriteNormalTextureBindGroup<1>,
    DrawSpriteNormalBatch,
);

pub(crate) struct SetSpriteViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteViewBindGroup<I> {
    type Param = ();
    type ViewQuery = (Read<ViewUniformOffset>, Read<SpriteViewBindGroup>);
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        (view_uniform, sprite_view_bind_group): ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<()>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &sprite_view_bind_group.value, &[view_uniform.offset]);
        RenderCommandResult::Success
    }
}
pub(crate) struct SetSpriteStencilTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteStencilTextureBindGroup<I> {
    type Param = (SRes<ImageBindGroups>, SRes<SpriteStencilBatches>);
    type ViewQuery = Read<ExtractedView>;
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, Self::ViewQuery>,
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
            image_bind_groups
                .values
                .get(&batch.image_handle_id)
                .unwrap(),
            &[],
        );
        RenderCommandResult::Success
    }
}

pub(crate) struct SetSpriteNormalTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteNormalTextureBindGroup<I> {
    type Param = (SRes<ImageBindGroups>, SRes<SpriteNormalBatches>);
    type ViewQuery = Read<ExtractedView>;
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, Self::ViewQuery>,
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
            image_bind_groups
                .values
                .get(&batch.image_handle_id)
                .unwrap(),
            &[],
        );
        RenderCommandResult::Success
    }
}

pub(crate) struct DrawSpriteStencilBatch;
impl<P: PhaseItem> RenderCommand<P> for DrawSpriteStencilBatch {
    type Param = (SRes<SpriteMeta>, SRes<SpriteStencilBatches>);
    type ViewQuery = Read<ExtractedView>;
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<()>,
        (sprite_meta, batches): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let sprite_meta = sprite_meta.into_inner();
        let Some(batch) = batches.get(&(view.retained_view_entity, item.entity())) else {
            return RenderCommandResult::Skip;
        };

        pass.set_index_buffer(
            sprite_meta.sprite_index_buffer.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.set_vertex_buffer(
            0,
            sprite_meta
                .sprite_instance_buffer
                .buffer()
                .unwrap()
                .slice(..),
        );
        pass.draw_indexed(0..6, 0, batch.range.clone());
        RenderCommandResult::Success
    }
}

pub(crate) struct DrawSpriteNormalBatch;
impl<P: PhaseItem> RenderCommand<P> for DrawSpriteNormalBatch {
    type Param = (SRes<SpriteMeta>, SRes<SpriteNormalBatches>);
    type ViewQuery = Read<ExtractedView>;
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<()>,
        (sprite_meta, batches): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let sprite_meta = sprite_meta.into_inner();
        let Some(batch) = batches.get(&(view.retained_view_entity, item.entity())) else {
            return RenderCommandResult::Skip;
        };

        pass.set_index_buffer(
            sprite_meta.sprite_index_buffer.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.set_vertex_buffer(
            0,
            sprite_meta
                .sprite_instance_buffer
                .buffer()
                .unwrap()
                .slice(..),
        );
        pass.draw_indexed(0..6, 0, batch.range.clone());
        RenderCommandResult::Success
    }
}
