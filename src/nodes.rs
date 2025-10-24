use bevy::{
    ecs::system::lifetimeless::Read,
    prelude::*,
    reflect::TypeData,
    render::{
        render_graph::{NodeRunError, ViewNode},
        render_phase::TrackedRenderPass,
        render_resource::{
            BindGroupEntries, DynamicUniformBuffer, GpuArrayBuffer, Operations, PipelineCache,
            RenderPassColorAttachment, RenderPassDescriptor, TextureDescriptor, TextureDimension,
            TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
            UniformBuffer, binding_types::uniform_buffer,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue, WgpuWrapper},
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    },
};

use crate::{
    EmptyLightMapTexture, IntermediaryLightMapTexture, LightMapTexture,
    extract::{self, ExtractedPointLight},
    pipelines::{LightmapApplicationPipeline, LightmapCreationPipeline, TransferTexturePipeline},
    prepare::{LightingDataBuffer, Lights},
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
    );

    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        (view_offset, lightmap, inter_lightmap, empty_lightmap, _): bevy::ecs::query::QueryItem<
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

        let lights = world.resource::<Lights>();

        for light in &lights.0 {
            {
                let bind_group = render_context.render_device().create_bind_group(
                    "create lightmap bind group",
                    &c_pipeline.layout,
                    &BindGroupEntries::sequential((
                        view_buffer.uniforms.binding().unwrap(),
                        &inter_lightmap.0.default_view,
                        &c_pipeline.sampler,
                        data_binding.clone(),
                        light.binding().unwrap(),
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

            {
                let bind_group = render_context.render_device().create_bind_group(
                    "transfer texture bind group",
                    &t_pipeline.layout,
                    &BindGroupEntries::sequential((&lightmap.0.default_view, &t_pipeline.sampler)),
                );

                let mut render_pass =
                    render_context.begin_tracked_render_pass(RenderPassDescriptor {
                        label: Some("transfer texture pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &inter_lightmap.0.default_view,
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
        }

        {
            let bind_group = render_context.render_device().create_bind_group(
                "transfer texture bind group",
                &t_pipeline.layout,
                &BindGroupEntries::sequential((
                    &empty_lightmap.0.default_view,
                    &t_pipeline.sampler,
                )),
            );

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("transfer texture pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &inter_lightmap.0.default_view,
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

        Ok(())
    }
}

impl ViewNode for ApplyLightmapNode {
    type ViewQuery = (Read<ViewTarget>, Read<LightMapTexture>);
    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_target, light_map_texture): bevy::ecs::query::QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> std::result::Result<(), NodeRunError> {
        let pipeline = world.resource::<LightmapApplicationPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "apply lightmap bind group",
            &pipeline.layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &light_map_texture.0.default_view,
                &pipeline.sampler,
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
