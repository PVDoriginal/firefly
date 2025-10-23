use bevy::{
    color::palettes::css::{BLUE, PINK},
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::system::lifetimeless::Read,
    prelude::*,
    render::{
        RenderApp, RenderSet,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        gpu_component_array_buffer,
        render_graph::{NodeRunError, RenderGraphApp, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, Buffer, BufferDescriptor,
            BufferUsages, CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState,
            GpuArrayBuffer, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, TextureFormat, TextureSampleType,
            binding_types::{sampler, texture_2d, uniform_buffer},
        },
        renderer::RenderDevice,
        sync_world::SyncToRenderWorld,
        texture::CachedTexture,
        view::{
            ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms, VisibilityClass, visibility,
        },
    },
};

use crate::{
    extract::{ExtractPlugin, ExtractedPointLight},
    nodes::{ApplyLightmapNode, CreateLightmapNode},
    pipelines::{LightmapApplicationPipeline, LightmapCreationPipeline},
    prepare::{LightingData, LightingDataBuffer, PreparePlugin},
};

mod extract;
mod nodes;
mod pipelines;
mod prepare;

#[derive(Component, Reflect)]
pub struct Occluder {
    pub shape: OccluderShape,
}

#[derive(Reflect)]
pub enum OccluderShape {
    Rectangle { width: f32, height: f32 },
}

#[derive(Component, Reflect)]
#[require(SyncToRenderWorld, VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<PointLight>)]
pub struct PointLight;

#[derive(Reflect)]
pub enum LightShape {
    Point,
}

#[derive(Component)]
pub(crate) struct LightMapTexture(pub CachedTexture);

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CreateLightmapLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ApplyLightmapLabel;

pub struct FireflyPlugin;

impl Plugin for FireflyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_gizmos);

        app.add_plugins((PreparePlugin, ExtractPlugin));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<CreateLightmapNode>>(
                Core2d,
                CreateLightmapLabel,
            )
            .add_render_graph_node::<ViewNodeRunner<ApplyLightmapNode>>(Core2d, ApplyLightmapLabel);

        render_app.add_render_graph_edges(
            Core2d,
            (
                Node2d::Tonemapping,
                CreateLightmapLabel,
                ApplyLightmapLabel,
                Node2d::EndMainPassPostProcessing,
            ),
        );
    }
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<LightmapCreationPipeline>();
        render_app.init_resource::<LightmapApplicationPipeline>();
    }
}

fn draw_gizmos(
    mut gizmos: Gizmos,
    lights: Query<&Transform, With<PointLight>>,
    occluders: Query<(&Transform, &Occluder)>,
) {
    for transform in &lights {
        let isometry = Isometry2d::from_translation(transform.translation.truncate());
        gizmos.circle_2d(isometry, 10., BLUE);
    }

    for (transform, occluder) in &occluders {
        let isometry = Isometry2d::from_translation(transform.translation.truncate());

        match occluder.shape {
            OccluderShape::Rectangle { width, height } => {
                gizmos.rect_2d(isometry, vec2(width, height), PINK);
            }
        }
    }
}
