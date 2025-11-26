use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_phase::ViewSortedRenderPhases,
        render_resource::{
            BindGroupEntries, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipeline,
        },
        renderer::RenderContext,
        texture::CachedTexture,
        view::{ExtractedView, ViewTarget, ViewUniformOffset, ViewUniforms},
    },
};

use crate::{
    EmptyLightMapTexture, IntermediaryLightMapTexture, LightMapTexture, NormalMapTexture,
    SpriteStencilTexture,
    lights::LightSet,
    occluders::OccluderSet,
    phases::{NormalPhase, Stencil2d},
    pipelines::{LightmapApplicationPipeline, LightmapCreationPipeline, TransferTexturePipeline},
    prepare::BufferedFireflyConfig,
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
        Read<BufferedFireflyConfig>,
        Read<NormalMapTexture>,
    );

    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        (
            view_offset,
            lightmap,
            inter_lightmap,
            empty_lightmap,
            _,
            sprite_stencil_texture,
            config,
            normal_map,
        ): bevy::ecs::query::QueryItem<'w, Self::ViewQuery>,
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
                let (occluders, sequences, vertices, round_occluders, ids) = &occluder_set.0[i];

                let (Some(occluders), Some(vertices), Some(round_occluders), Some(ids)) = (
                    occluders.binding(),
                    vertices.binding(),
                    round_occluders.binding(),
                    ids.binding(),
                ) else {
                    return Ok(());
                };

                let bind_group = render_context.render_device().create_bind_group(
                    "create lightmap bind group",
                    &c_pipeline.layout,
                    &BindGroupEntries::sequential((
                        view_buffer.uniforms.binding().unwrap(),
                        &inter_lightmap.0.default_view,
                        &c_pipeline.sampler,
                        light.binding().unwrap(),
                        occluders.clone(),
                        sequences.binding().unwrap(),
                        vertices.clone(),
                        round_occluders.clone(),
                        &sprite_stencil_texture.0.default_view,
                        &normal_map.0.default_view,
                        ids.clone(),
                        config.0.binding().unwrap(),
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

#[derive(Default)]
pub(crate) struct SpriteStencilNode;
impl ViewNode for SpriteStencilNode {
    type ViewQuery = (&'static ExtractedView, Read<SpriteStencilTexture>);

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view, stencil_texture): QueryItem<'w, Self::ViewQuery>,
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
        (view, normal_texture): QueryItem<'w, Self::ViewQuery>,
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
