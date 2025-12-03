use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_phase::{ViewBinnedRenderPhases, ViewSortedRenderPhases},
        render_resource::{
            BindGroupEntries, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipeline,
        },
        renderer::RenderContext,
        texture::CachedTexture,
        view::{ExtractedView, ViewTarget},
    },
};

use crate::{
    LightMapTexture, LightmapPhase, NormalMapTexture, SpriteStencilTexture,
    phases::{NormalPhase, Stencil2d},
    pipelines::LightmapApplicationPipeline,
    prepare::BufferedFireflyConfig,
};

#[derive(Default)]
pub(crate) struct CreateLightmapNode;

#[derive(Default)]
pub(crate) struct ApplyLightmapNode;

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
            &pipeline.layout,
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
pub(crate) struct SpriteStencilNode;
impl ViewNode for SpriteStencilNode {
    type ViewQuery = (&'static ExtractedView, Read<SpriteStencilTexture>);

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view, stencil_texture): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(stencil_phases) = world.get_resource::<ViewSortedRenderPhases<Stencil2d>>() else {
            return Ok(());
        };

        let view_entity = graph.view_entity();

        let Some(stencil_phase) = stencil_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("stencil pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &stencil_texture.0.default_view,
                resolve_target: None,
                ops: default(),
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Err(err) = stencil_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the stencil phase {err:?}");
        }

        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct SpriteNormalNode;
impl ViewNode for SpriteNormalNode {
    type ViewQuery = (&'static ExtractedView, Read<NormalMapTexture>);

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view, normal_texture): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(normal_phases) = world.get_resource::<ViewSortedRenderPhases<NormalPhase>>()
        else {
            return Ok(());
        };

        let view_entity = graph.view_entity();

        let Some(normal_phase) = normal_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("normal pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &normal_texture.0.default_view,
                resolve_target: None,
                ops: default(),
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Err(err) = normal_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the normal phase {err:?}");
        }

        Ok(())
    }
}
