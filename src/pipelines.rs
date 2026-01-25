//! Module containing the custom `Render Pipelines` used by Firefly.

use std::borrow::Cow;

use bevy::{
    core_pipeline::{FullscreenShader, tonemapping::get_lut_bind_group_layout_entries},
    mesh::{PrimitiveTopology, VertexBufferLayout, VertexFormat},
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        render_resource::{
            BindGroupLayoutDescriptor, BindGroupLayoutEntries, BlendComponent, BlendFactor,
            BlendOperation, BlendState, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            FragmentState, FrontFace, PipelineCache, PolygonMode, PrimitiveState,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            SpecializedRenderPipeline, TextureFormat, TextureSampleType, VertexAttribute,
            VertexState, VertexStepMode,
            binding_types::{sampler, storage_buffer_read_only, texture_2d, uniform_buffer},
        },
        renderer::RenderDevice,
        view::ViewUniform,
    },
    shader::ShaderDefVal,
    sprite_render::SpritePipelineKey,
};

use crate::{
    APPLY_LIGHTMAP_SHADER, CREATE_LIGHTMAP_SHADER, SPRITE_SHADER,
    buffers::{Bin, BinCounts, N_BINS},
    data::UniformFireflyConfig,
    lights::UniformPointLight,
    occluders::{UniformOccluder, UniformRoundOccluder},
};

/// Plugin that initializes various Pipelines. Added automatically by [`FireflyPlugin`](crate::prelude::FireflyPlugin).
pub struct PipelinePlugin;

impl Plugin for PipelinePlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            RenderStartup,
            (
                init_lightmap_creation_pipeline,
                init_lightmap_application_pipeline,
                init_sprite_pipeline,
            ),
        );
    }
}

/// Pipeline that creates the lightmap from the relevant bindings.
#[derive(Resource)]
pub struct LightmapCreationPipeline {
    pub layout: BindGroupLayoutDescriptor,
    pub sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
}

/// Pipeline that applies the lightmap over the fullscreen view.
#[derive(Resource)]
pub struct LightmapApplicationPipeline {
    pub layout: BindGroupLayoutDescriptor,
    pub sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
}

fn init_lightmap_creation_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "create lightmap layout",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::FRAGMENT,
            (
                // view uniform
                (0, uniform_buffer::<ViewUniform>(true)),
                // sampler
                (1, sampler(SamplerBindingType::Filtering)),
                // point lights
                (2, storage_buffer_read_only::<UniformPointLight>(false)),
                (3, storage_buffer_read_only::<u32>(false)),
                // round occluders
                (4, storage_buffer_read_only::<UniformRoundOccluder>(false)),
                // poly occluders
                (5, storage_buffer_read_only::<UniformOccluder>(false)),
                // vertices
                (6, storage_buffer_read_only::<Vec2>(false)),
                // bins
                (7, storage_buffer_read_only::<[Bin; N_BINS]>(false)),
                (8, storage_buffer_read_only::<BinCounts>(false)),
                // sprite stencil
                (
                    9,
                    texture_2d(TextureSampleType::Float { filterable: false }),
                ),
                // sprite normal map
                (
                    10,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                // config,
                (11, uniform_buffer::<UniformFireflyConfig>(false)),
                // bins,
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let vertex_state = fullscreen_shader.to_vertex_state();

    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some(Cow::Borrowed("lightmap creation pipeline")),
        layout: vec![layout.clone()],
        vertex: vertex_state,
        fragment: Some(FragmentState {
            shader: CREATE_LIGHTMAP_SHADER,
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::Rgba16Float,
                blend: Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::Src,
                        dst_factor: BlendFactor::Dst,
                        operation: BlendOperation::Max,
                    },
                    alpha: BlendComponent::REPLACE,
                }),
                write_mask: ColorWrites::ALL,
            })],
            shader_defs: default(),
            entry_point: Some(Cow::Borrowed("fragment")),
        }),
        push_constant_ranges: default(),
        primitive: default(),
        depth_stencil: default(),
        multisample: default(),
        zero_initialize_workgroup_memory: default(),
    });

    commands.insert_resource(LightmapCreationPipeline {
        layout,
        sampler,
        pipeline_id,
    });
}

fn init_lightmap_application_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
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
    let vertex_state = fullscreen_shader.to_vertex_state();

    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some(Cow::Borrowed("lightmap application pipeline")),
        layout: vec![layout.clone()],
        vertex: vertex_state,
        fragment: Some(FragmentState {
            shader: APPLY_LIGHTMAP_SHADER,
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            shader_defs: default(),
            entry_point: Some(Cow::Borrowed("fragment")),
        }),
        push_constant_ranges: default(),
        primitive: default(),
        depth_stencil: default(),
        multisample: default(),
        zero_initialize_workgroup_memory: default(),
    });

    commands.insert_resource(LightmapApplicationPipeline {
        layout,
        sampler,
        pipeline_id,
    });
}

/// Pipeline that produces the stencil and normal textures from the sprite bindings.
#[derive(Resource)]
#[allow(dead_code)]
pub struct SpritePipeline {
    pub view_layout: BindGroupLayoutDescriptor,
    pub material_layout: BindGroupLayoutDescriptor,
}

fn init_sprite_pipeline(mut commands: Commands) {
    let tonemapping_lut_entries = get_lut_bind_group_layout_entries();
    let view_layout = BindGroupLayoutDescriptor::new(
        "sprite_view_layout",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::VERTEX_FRAGMENT,
            (
                (0, uniform_buffer::<ViewUniform>(true)),
                (
                    1,
                    tonemapping_lut_entries[0].visibility(ShaderStages::FRAGMENT),
                ),
                (
                    2,
                    tonemapping_lut_entries[1].visibility(ShaderStages::FRAGMENT),
                ),
            ),
        ),
    );

    let material_layout = BindGroupLayoutDescriptor::new(
        "sprite_material_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // sprite texture
                texture_2d(TextureSampleType::Float { filterable: true }),
                // normal map texture
                texture_2d(TextureSampleType::Float { filterable: true }),
                // sampler
                sampler(SamplerBindingType::Filtering),
                // dummy normal bool
                uniform_buffer::<u32>(false),
            ),
        ),
    );

    commands.insert_resource(SpritePipeline {
        view_layout,
        material_layout,
    });
}

impl SpecializedRenderPipeline for SpritePipeline {
    type Key = SpritePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.contains(SpritePipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_TEXTURE_BINDING_INDEX".into(),
                1,
            ));
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_SAMPLER_BINDING_INDEX".into(),
                2,
            ));

            let method = key.intersection(SpritePipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == SpritePipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
            {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(SpritePipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        let instance_rate_vertex_buffer_layout = VertexBufferLayout {
            array_stride: 80,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // @location(0) i_model_transpose_col0: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                // @location(1) i_model_transpose_col1: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 1,
                },
                // @location(2) i_model_transpose_col2: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 2,
                },
                // @location(3) i_uv_offset_scale: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 48,
                    shader_location: 3,
                },
                // @location(4) z: f32,
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: 64,
                    shader_location: 4,
                },
                // @location(5) height: f32,
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: 68,
                    shader_location: 5,
                },
            ],
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: SPRITE_SHADER,
                entry_point: Some("vertex".into()),
                shader_defs: shader_defs.clone(),
                buffers: vec![instance_rate_vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: SPRITE_SHADER,
                shader_defs,
                entry_point: Some("fragment".into()),
                targets: vec![
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba32Float, //format,
                        blend: Some(BlendState::ALPHA_BLENDING),
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba32Float,
                        blend: Some(BlendState::ALPHA_BLENDING),
                        write_mask: ColorWrites::ALL,
                    }),
                ],
            }),
            layout: vec![self.view_layout.clone(), self.material_layout.clone()],
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,

                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: default(),
            label: Some("sprite_stencil_pipeline".into()),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        }
    }
}
