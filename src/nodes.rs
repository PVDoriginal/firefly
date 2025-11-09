use bevy::{
    ecs::system::lifetimeless::Read,
    prelude::*,
    render::{
        render_graph::{NodeRunError, ViewNode},
        render_resource::{
            BindGroupEntries, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipeline, TextureView,
        },
        renderer::RenderContext,
        texture::CachedTexture,
        view::{ViewTarget, ViewUniformOffset, ViewUniforms},
    },
};

use crate::{
    EmptyLightMapTexture, IntermediaryLightMapTexture, LightMapTexture,
    lights::LightSet,
    occluders::OccluderSet,
    pipelines::{LightmapApplicationPipeline, LightmapCreationPipeline, TransferTexturePipeline},
    prepare::{BufferedFireflyConfig, LightingDataBuffer},
    sprites::SpriteStencilTexture,
};

#[derive(Default)]
pub(crate) struct CreateLightmapNode;

#[derive(Default)]
pub(crate) struct ApplyLightmapNode;

impl ViewNode for CreateLightmapNode {
    type ViewQuery = (
        Read<ViewUniformOffset>,
        Read<LightMapTexture>,
        Read<IntermediaryLightMapTexture>,
        Read<EmptyLightMapTexture>,
        Read<ViewTarget>,
        Read<SpriteStencilTexture>,
    );

    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        (view_offset, lightmap, inter_lightmap, empty_lightmap, _, sprite_stencil_texture): bevy::ecs::query::QueryItem<
            'w,
            Self::ViewQuery,
        >,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();

        let c_pipeline = world.resource::<LightmapCreationPipeline>();
        let Some(c_render_pipeline) = pipeline_cache.get_render_pipeline(c_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let t_pipeline = world.resource::<TransferTexturePipeline>();
        let Some(t_render_pipeline) = pipeline_cache.get_render_pipeline(t_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let view_buffer = world.resource::<ViewUniforms>();

        let Some(data_binding) = world.resource::<LightingDataBuffer>().0.binding() else {
            return Ok(());
        };

        let lights = world.resource::<LightSet>();
        let occluder_set = world.resource::<OccluderSet>();

        // if there are no lights, clear the lightmap and return
        if lights.0.is_empty() {
            transfer_texture(
                &empty_lightmap.0,
                &lightmap.0,
                render_context,
                t_pipeline,
                t_render_pipeline,
            );
            return Ok(());
        }

        for (i, light) in lights.0.iter().enumerate() {
            {
                // info!("rendering light #{i}");
                let (meta, vertices) = &occluder_set.0[i];

                let (Some(meta), Some(vertices)) = (meta.binding(), vertices.binding()) else {
                    return Ok(());
                };

                let bind_group = render_context.render_device().create_bind_group(
                    "create lightmap bind group",
                    &c_pipeline.layout,
                    &BindGroupEntries::sequential((
                        view_buffer.uniforms.binding().unwrap(),
                        &inter_lightmap.0.default_view,
                        &c_pipeline.sampler,
                        data_binding.clone(),
                        light.binding().unwrap(),
                        meta.clone(),
                        vertices.clone(),
                        &sprite_stencil_texture.0.default_view,
                        &c_pipeline.stencil_sampler,
                    )),
                );

                let mut render_pass =
                    render_context.begin_tracked_render_pass(RenderPassDescriptor {
                        label: Some("create lightmap pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &lightmap.0.default_view,
                            resolve_target: None,
                            ops: default(),
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                render_pass.set_render_pipeline(c_render_pipeline);
                render_pass.set_bind_group(0, &bind_group, &[view_offset.offset]);
                render_pass.draw(0..3, 0..1);
            }

            transfer_texture(
                &lightmap.0,
                &inter_lightmap.0,
                render_context,
                t_pipeline,
                t_render_pipeline,
            );
        }

        transfer_texture(
            &empty_lightmap.0,
            &inter_lightmap.0,
            render_context,
            t_pipeline,
            t_render_pipeline,
        );

        Ok(())
    }
}

fn transfer_texture(
    src: &CachedTexture,
    dst: &CachedTexture,
    render_context: &mut RenderContext,
    t_pipeline: &TransferTexturePipeline,
    t_render_pipeline: &RenderPipeline,
) {
    let bind_group = render_context.render_device().create_bind_group(
        "transfer texture bind group",
        &t_pipeline.layout,
        &BindGroupEntries::sequential((&src.default_view, &t_pipeline.sampler)),
    );

    let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("transfer texture pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &dst.default_view,
            resolve_target: None,
            ops: default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    render_pass.set_render_pipeline(t_render_pipeline);
    render_pass.set_bind_group(0, &bind_group, &[]);
    render_pass.draw(0..3, 0..1);
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
        (config, view_target, light_map_texture): bevy::ecs::query::QueryItem<'w, Self::ViewQuery>,
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
