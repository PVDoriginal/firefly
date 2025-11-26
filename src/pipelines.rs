use std::borrow::Cow;

use bevy::{
    core_pipeline::{
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
        tonemapping::get_lut_bind_group_layout_entries,
    },
    ecs::system::SystemState,
    image::{ImageSampler, TextureFormatPixelInfo},
    prelude::*,
    render::{
        mesh::{PrimitiveTopology, VertexBufferLayout, VertexFormat},
        render_resource::{
            BindGroupLayout, BindGroupLayoutEntries, BlendState, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, FrontFace, GpuArrayBuffer, PipelineCache,
            PolygonMode, PrimitiveState, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderDefVal, ShaderStages, SpecializedRenderPipeline,
            TexelCopyBufferLayout, TextureFormat, TextureSampleType, TextureViewDescriptor,
            VertexAttribute, VertexState, VertexStepMode,
            binding_types::{sampler, texture_2d, uniform_buffer},
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{DefaultImageSampler, GpuImage},
        view::ViewUniform,
    },
    sprite::SpritePipelineKey,
};

use crate::{
    APPLY_LIGHTMAP_SHADER, CREATE_LIGHTMAP_SHADER, SPRITE_SHADER, TRANSFER_SHADER,
    data::UniformFireflyConfig,
    lights::UniformPointLight,
    occluders::{UniformOccluder, UniformRoundOccluder, UniformVertex},
};

#[derive(Resource)]
pub(crate) struct LightmapCreationPipeline {
    pub layout: BindGroupLayout,
    pub sampler: Sampler,
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
                    // point light
                    uniform_buffer::<UniformPointLight>(false),
                    // occluders
                    GpuArrayBuffer::<UniformOccluder>::binding_layout(render_device),
                    // sequences
                    GpuArrayBuffer::<u32>::binding_layout(render_device),
                    // vertices
                    GpuArrayBuffer::<UniformVertex>::binding_layout(render_device),
                    // round occluders
                    GpuArrayBuffer::<UniformRoundOccluder>::binding_layout(render_device),
                    // sprite stencil
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // sprite normal map
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    // sprite ids
                    GpuArrayBuffer::<f32>::binding_layout(render_device),
                    //config,
                    uniform_buffer::<UniformFireflyConfig>(false),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

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

#[derive(Resource)]
#[allow(dead_code)]
pub(crate) struct SpriteStencilPipeline {
    pub view_layout: BindGroupLayout,
    pub material_layout: BindGroupLayout,
    pub dummy_white_gpu_image: GpuImage,
}

impl FromWorld for SpriteStencilPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<DefaultImageSampler>,
            Res<RenderQueue>,
        )> = SystemState::new(world);
        let (render_device, default_sampler, render_queue) = system_state.get_mut(world);

        let tonemapping_lut_entries = get_lut_bind_group_layout_entries();
        let view_layout = render_device.create_bind_group_layout(
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

        let material_layout = render_device.create_bind_group_layout(
            "sprite_material_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );
        let dummy_white_gpu_image = {
            let image = Image::default();
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = match image.sampler {
                ImageSampler::Default => (**default_sampler).clone(),
                ImageSampler::Descriptor(ref descriptor) => {
                    render_device.create_sampler(&descriptor.as_wgpu())
                }
            };

            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                texture.as_image_copy(),
                image.data.as_ref().expect("Image has no data"),
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width() * format_size as u32),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );
            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: image.texture_descriptor.size,
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };

        SpriteStencilPipeline {
            view_layout,
            material_layout,
            dummy_white_gpu_image,
        }
    }
}

impl SpecializedRenderPipeline for SpriteStencilPipeline {
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
                // @location(3) id: f32,
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: 48,
                    shader_location: 3,
                },
                // @location(4) i_uv_offset_scale: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 52,
                    shader_location: 4,
                },
                // @location(5) z: f32,
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
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![instance_rate_vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: SPRITE_SHADER,
                shader_defs,
                entry_point: "fragment_stencil".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba32Float, //format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
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

#[derive(Resource)]
#[allow(dead_code)]
pub(crate) struct SpriteNormalMapsPipeline {
    pub view_layout: BindGroupLayout,
    pub material_layout: BindGroupLayout,
    pub dummy_white_gpu_image: GpuImage,
}

impl FromWorld for SpriteNormalMapsPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<DefaultImageSampler>,
            Res<RenderQueue>,
        )> = SystemState::new(world);
        let (render_device, default_sampler, render_queue) = system_state.get_mut(world);

        let tonemapping_lut_entries = get_lut_bind_group_layout_entries();
        let view_layout = render_device.create_bind_group_layout(
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

        let material_layout = render_device.create_bind_group_layout(
            "sprite_material_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );
        let dummy_white_gpu_image = {
            let image = Image::default();
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = match image.sampler {
                ImageSampler::Default => (**default_sampler).clone(),
                ImageSampler::Descriptor(ref descriptor) => {
                    render_device.create_sampler(&descriptor.as_wgpu())
                }
            };

            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                texture.as_image_copy(),
                image.data.as_ref().expect("Image has no data"),
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width() * format_size as u32),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );
            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: image.texture_descriptor.size,
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };

        SpriteNormalMapsPipeline {
            view_layout,
            material_layout,
            dummy_white_gpu_image,
        }
    }
}

impl SpecializedRenderPipeline for SpriteNormalMapsPipeline {
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
                // @location(3) id: f32,
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: 48,
                    shader_location: 3,
                },
                // @location(4) i_uv_offset_scale: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 52,
                    shader_location: 4,
                },
                // @location(5) z: f32,
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
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![instance_rate_vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: SPRITE_SHADER,
                shader_defs,
                entry_point: "fragment_normal".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba32Float,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
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
            label: Some("sprite_normal_maps_pipeline".into()),
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        }
    }
}
