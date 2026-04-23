//! Module containg `Render Graph Nodes` used by Firefly.  

use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_phase::{ViewBinnedRenderPhases, ViewSortedRenderPhases},
        render_resource::{
            BindGroupEntries, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            SpecializedRenderPipelines, TextureAspect, TextureDimension, TextureFormat,
            TextureUsages, TextureViewDescriptor, TextureViewDimension,
        },
        renderer::RenderContext,
        view::{ExtractedView, ViewTarget},
    },
};

use crate::{
    CombinedLightMapTextures, LightMapTexture, LightmapPhase, NormalMapTexture,
    SpriteStencilTexture,
    data::{CombinedLightmaps, ExtractedCombineLightmapTo},
    phases::SpritePhase,
    pipelines::{LightmapApplicationPipeline, SpecializedApplicationPipeline},
    prepare::BufferedFireflyConfig,
};

/// Node used to create the lightmap.
#[derive(Default)]
pub struct CreateLightmapNode;

impl ViewNode for CreateLightmapNode {
    type ViewQuery = (
        &'static ExtractedView,
        Read<LightMapTexture>,
        Option<Read<ExtractedCombineLightmapTo>>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view, lightmap_texture, combine_lightmap_to): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(lightmap_phases) = world.get_resource::<ViewBinnedRenderPhases<LightmapPhase>>()
        else {
            return Ok(());
        };

        let view_entity = graph.view_entity();

        let Some(lightmap_phase) = lightmap_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        let view = if let Some(combine_lightmap_to) = combine_lightmap_to {
            let lightmap = world
                .get::<CombinedLightMapTextures>(combine_lightmap_to.0)
                .unwrap();

            let hdr = world
                .get::<ExtractedView>(combine_lightmap_to.0)
                .unwrap()
                .hdr;

            let format = match hdr {
                true => ViewTarget::TEXTURE_FORMAT_HDR,
                false => TextureFormat::bevy_default(),
            };
            // &lightmap.0.default_view
            &lightmap.0.texture.create_view(&TextureViewDescriptor {
                label: "layer of combined lightmap texture array".into(),
                format: Some(format),
                dimension: Some(TextureViewDimension::D2Array),
                usage: Some(TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING),
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: combine_lightmap_to.1,
                array_layer_count: Some(1),
            })
        } else {
            &lightmap_texture.0.default_view
        };
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("lightmap pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: default(),
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Err(err) = lightmap_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the stencil phase {err:?}");
        }
        Ok(())
    }
}

/// Node used to apply the lightmap over the fullscreen view.
#[derive(Default)]
pub struct ApplyLightmapNode;

impl ViewNode for ApplyLightmapNode {
    type ViewQuery = (
        Read<SpecializedApplicationPipeline>,
        Read<BufferedFireflyConfig>,
        Read<ViewTarget>,
        Read<LightMapTexture>,
        Option<Read<CombinedLightMapTextures>>,
        Has<ExtractedCombineLightmapTo>,
    );

    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            pipeline_id,
            config,
            view_target,
            light_map_texture,
            combined_textures,
            is_combined_to
        ): bevy::ecs::query::QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> std::result::Result<(), NodeRunError> {
        if is_combined_to {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<LightmapApplicationPipeline>();

        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.id) else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();
        let Some(config) = config.0.binding() else {
            return Ok(());
        };

        let format = match view_target.is_hdr() {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        let bind_group = if !pipeline_id.is_combined {
            render_context.render_device().create_bind_group(
                "apply lightmap bind group simple",
                &pipeline_cache.get_bind_group_layout(&pipeline.layout_simple),
                &BindGroupEntries::sequential((
                    post_process.source,
                    &light_map_texture.0.default_view,
                    &pipeline.sampler,
                    config,
                )),
            )
        } else {
            let combined_view =
                combined_textures
                    .unwrap()
                    .0
                    .texture
                    .create_view(&TextureViewDescriptor {
                        label: "combined lightmap texture array".into(),
                        format: Some(format),
                        dimension: Some(TextureViewDimension::D2Array),
                        usage: Some(
                            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                        ),
                        aspect: TextureAspect::All,
                        base_mip_level: 0,
                        mip_level_count: Some(1),
                        base_array_layer: 0,
                        array_layer_count: None,
                    });

            render_context.render_device().create_bind_group(
                "apply lightmap bind group combined",
                &pipeline_cache.get_bind_group_layout(&pipeline.layout_combined),
                &BindGroupEntries::sequential((
                    post_process.source,
                    &light_map_texture.0.default_view,
                    &pipeline.sampler,
                    config,
                    &combined_view,
                )),
            )
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("apply lightmap pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: default(),
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct SpriteNode;
impl ViewNode for SpriteNode {
    type ViewQuery = (
        &'static ExtractedView,
        Read<SpriteStencilTexture>,
        Read<NormalMapTexture>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view, stencil_texture, normal_map_texture): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(sprite_phases) = world.get_resource::<ViewSortedRenderPhases<SpritePhase>>()
        else {
            return Ok(());
        };

        let view_entity = graph.view_entity();

        let Some(sprite_phase) = sprite_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("stencil pass"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: &stencil_texture.0.default_view,
                    resolve_target: None,
                    ops: default(),
                    depth_slice: None,
                }),
                Some(RenderPassColorAttachment {
                    view: &normal_map_texture.0.default_view,
                    resolve_target: None,
                    ops: default(),
                    depth_slice: None,
                }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Err(err) = sprite_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the stencil phase {err:?}");
        }

        Ok(())
    }
}
