use std::borrow::Cow;

use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState,
            ColorWrites, FragmentState, GpuArrayBuffer, PipelineCache, RenderPipelineDescriptor,
            Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, TextureFormat,
            TextureSampleType,
            binding_types::{sampler, texture_2d, uniform_buffer},
        },
        renderer::RenderDevice,
        view::ViewUniform,
    },
};

use crate::{
    APPLY_LIGHTMAP_SHADER, CREATE_LIGHTMAP_SHADER, TRANSFER_SHADER,
    data::{UniformFireflyConfig, UniformMeta},
    lights::UniformPointLight,
    occluders::{UniformOccluder, UniformRoundOccluder, UniformVertex},
};

#[derive(Resource)]
pub(crate) struct LightmapCreationPipeline {
    pub layout: BindGroupLayout,
    pub sampler: Sampler,
    pub stencil_sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
}

#[derive(Resource)]
pub(crate) struct LightmapApplicationPipeline {
    pub layout: BindGroupLayout,
    pub sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
}

#[derive(Resource)]
pub(crate) struct TransferTexturePipeline {
    pub layout: BindGroupLayout,
    pub sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for LightmapCreationPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "create lightmap layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    // view uniform
                    uniform_buffer::<ViewUniform>(true),
                    // previous lightmap
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // sampler
                    sampler(SamplerBindingType::Filtering),
                    // meta
                    uniform_buffer::<UniformMeta>(false),
                    // point light
                    uniform_buffer::<UniformPointLight>(false),
                    // occluders
                    GpuArrayBuffer::<UniformOccluder>::binding_layout(render_device),
                    // vertices
                    GpuArrayBuffer::<UniformVertex>::binding_layout(render_device),
                    // round occluders
                    GpuArrayBuffer::<UniformRoundOccluder>::binding_layout(render_device),
                    // sprite stencil
                    texture_2d(TextureSampleType::Float { filterable: false }),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());
        let stencil_sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let pipeline_id = new_pipeline(
            world,
            Some(Cow::Borrowed("lightmap creation pipeline")),
            layout.clone(),
            CREATE_LIGHTMAP_SHADER,
            Cow::Borrowed("fragment"),
            TextureFormat::Rgba16Float,
        );

        Self {
            layout,
            sampler,
            stencil_sampler,
            pipeline_id,
        }
    }
}

impl FromWorld for LightmapApplicationPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "apply lightmap layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<UniformFireflyConfig>(false),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let pipeline_id = new_pipeline(
            world,
            Some(Cow::Borrowed("lightmap application pipeline")),
            layout.clone(),
            APPLY_LIGHTMAP_SHADER,
            Cow::Borrowed("fragment"),
            TextureFormat::bevy_default(),
        );

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}

impl FromWorld for TransferTexturePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "transfer texture layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let pipeline_id = new_pipeline(
            world,
            Some(Cow::Borrowed("transfer texture pipeline")),
            layout.clone(),
            TRANSFER_SHADER,
            Cow::Borrowed("fragment"),
            TextureFormat::Rgba16Float,
        );

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}

fn new_pipeline(
    world: &mut World,
    label: Option<Cow<'static, str>>,
    layout: BindGroupLayout,
    shader: Handle<Shader>,
    entry: Cow<'static, str>,
    format: TextureFormat,
) -> CachedRenderPipelineId {
    world
        .resource_mut::<PipelineCache>()
        .queue_render_pipeline(RenderPipelineDescriptor {
            label,
            layout: vec![layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader,
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                shader_defs: default(),
                entry_point: entry,
            }),
            push_constant_ranges: default(),
            primitive: default(),
            depth_stencil: default(),
            multisample: default(),
            zero_initialize_workgroup_memory: default(),
        })
}
