//! Module containg `Render Graph Nodes` used by Firefly.  

use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_phase::{ViewBinnedRenderPhases, ViewSortedRenderPhases},
        render_resource::{
            BindGroupEntries, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        },
        renderer::RenderContext,
        view::{ExtractedView, ViewTarget},
    },
};

use crate::{
    LightMapTexture, LightmapPhase, NormalMapTexture, SpriteStencilTexture, phases::SpritePhase,
    pipelines::LightmapApplicationPipeline, prepare::BufferedFireflyConfig,
};

/// Node used to create the lightmap.
#[derive(Default)]
pub struct CreateLightmapNode;

/// Node used to apply the lightmap over the fullscreen view.
#[derive(Default)]
pub struct ApplyLightmapNode;

impl ViewNode for CreateLightmapNode {
    type ViewQuery = (&'static ExtractedView, Read<LightMapTexture>);

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view, lightmap_texture): QueryItem<'w, '_, Self::ViewQuery>,
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

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("lightmap pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &lightmap_texture.0.default_view,
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

impl ViewNode for ApplyLightmapNode {
    type ViewQuery = (
        Read<BufferedFireflyConfig>,
        Read<ViewTarget>,
        Read<LightMapTexture>,
    );
    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (config, view_target, light_map_texture): bevy::ecs::query::QueryItem<
            'w,
            '_,
            Self::ViewQuery,
        >,
        world: &'w World,
    ) -> std::result::Result<(), NodeRunError> {
        let pipeline = world.resource::<LightmapApplicationPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();
        let Some(config) = config.0.binding() else {
            return Ok(());
        };

        let bind_group = render_context.render_device().create_bind_group(
            "apply lightmap bind group",
            &pipeline_cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                post_process.source,
                &light_map_texture.0.default_view,
                &pipeline.sampler,
                config,
            )),
        );

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
