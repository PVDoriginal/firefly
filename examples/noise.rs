//! This example demonstrates how to apply a shader over the lightmap before it is being applied to the camera's output.
//!
//! This is greatly inspired by Bevy's post processing example: https://bevy.org/examples/shaders/custom-post-processing/.
//! Check out that example for more in-depth comments and explanations.

use bevy::{
    color::palettes::css::{RED, WHITE},
    core_pipeline::{FullscreenShader, core_2d::graph::Core2d},
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, Operations,
            PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
            TextureUsages,
            binding_types::{sampler, texture_2d, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice},
        texture::{CachedTexture, TextureCache},
        view::{ExtractedView, Hdr, ViewTarget},
    },
    window::PrimaryWindow,
};
use bevy_firefly::{ApplyLightmapLabel, CreateLightmapLabel, LightMapTexture, prelude::*};

// The shader that will apply the noise over the lightmap.
const NOISE_SHADER_ASSET_PATH: &str = "shaders/noise.wgsl";

// Simple shader which writes the pixels of a texture unto another texture.
const TRANSFER_SHADER_ASSET_PATH: &str = "shaders/transfer.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FireflyPlugin, LightmapEditPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_time, move_light))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Hdr,
        FireflyConfig::default(),
        NoiseSettings { time: 0.0, kf: 3.0 },
        Transform::default(),
    ));

    commands.spawn((
        PointLight2d {
            color: Color::Srgba(WHITE),
            intensity: 3.0,
            range: 200.,
            ..default()
        },
        Transform::default(),
    ));

    commands.spawn((
        Occluder2d::circle(20.0).with_opacity(0.8),
        Transform::from_translation(vec3(60., 0., 0.)),
    ));
    commands.spawn((
        Occluder2d::circle(20.0).with_opacity(0.8),
        Transform::from_translation(vec3(-60., 0., 0.)),
    ));
    commands.spawn((
        Occluder2d::circle(20.0).with_opacity(0.8),
        Transform::from_translation(vec3(0., 60., 0.)),
    ));
    commands.spawn((
        Occluder2d::circle(20.0).with_opacity(0.8),
        Transform::from_translation(vec3(0., -60., 0.)),
    ));
}

fn update_time(mut settings: Single<&mut NoiseSettings>, time: Res<Time>) {
    settings.time = time.elapsed_secs();
}

fn move_light(
    mut light: Single<&mut Transform, With<PointLight2d>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut gizmos: Gizmos,
) {
    if !buttons.pressed(MouseButton::Left) {
        return;
    }

    let Some(cursor_position) = window
        .cursor_position()
        .and_then(|cursor| camera.0.viewport_to_world_2d(&camera.1, cursor).ok())
    else {
        return;
    };

    gizmos.circle_2d(Isometry2d::from_translation(cursor_position), 5., RED);

    light.translation = cursor_position.extend(0.);
}

struct LightmapEditPlugin;

impl Plugin for LightmapEditPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<NoiseSettings>::default(),
            UniformComponentPlugin::<NoiseSettings>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(RenderStartup, (init_noise_pipeline, init_transfer_pipeline))
            .add_systems(Render, prepare_empty_texture.in_set(RenderSystems::Prepare))
            .add_render_graph_node::<ViewNodeRunner<NoiseNode>>(Core2d, NoiseLabel)
            .add_render_graph_edges(
                Core2d,
                // `NoiseLabel` is added between `CreateLightmapLabel` and `ApplyLightmapLabel`.
                // This makes our render pass execute after the lightmap is created but before it is applied to the camera.
                (CreateLightmapLabel, NoiseLabel, ApplyLightmapLabel),
            );
    }
}

// System to prepare the empty texture before the render pass
fn prepare_empty_texture(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    view_targets: Query<(Entity, &ViewTarget, &ExtractedView)>,
) {
    for (entity, view_target, view) in &view_targets {
        let format = match view.hdr {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        let empty_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("empty_texture"),
                size: view_target.main_texture().size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        commands.entity(entity).insert(EmptyTexture(empty_texture));
    }
}

// Extra temporary texture used to transfer the noise shader output back into the lightmap.
#[derive(Component)]
struct EmptyTexture(pub CachedTexture);

// Component that will be extracted and written to the shader.
#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
struct NoiseSettings {
    // time updated each frame
    time: f32,
    // constant used to generate the noise
    kf: f32,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct NoiseLabel;

#[derive(Default)]
struct NoiseNode;

impl ViewNode for NoiseNode {
    type ViewQuery = (
        &'static NoiseSettings,
        &'static DynamicUniformIndex<NoiseSettings>,
        // We need to query the lightmap texture in order to change it.
        Read<LightMapTexture>,
        Read<EmptyTexture>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (_noise_settings, settings_index, lightmap, empty_texture): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let noise_pipeline_data = world.resource::<NoisePipeline>();
        let transfer_pipeline_data = world.resource::<TransferPipeline>();

        let pipeline_cache = world.resource::<PipelineCache>();

        let (Some(noise_pipeline), Some(transfer_pipeline)) = (
            pipeline_cache.get_render_pipeline(noise_pipeline_data.pipeline_id),
            pipeline_cache.get_render_pipeline(transfer_pipeline_data.pipeline_id),
        ) else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<NoiseSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        // Reading a texture and outputting to it at the same time is not supported, so two render passes are made:
        // One to read the lightmap, apply the noise, and output to a temporary texture (`EmptyTexture`),
        // and another to read from the temporary texture and output back to the lightmap.
        //
        // This is why `TransferPipeline` and `transfer.wgsl` are needed.

        // The first render pass
        {
            let noise_bind_group = render_context.render_device().create_bind_group(
                "noise_bind_group",
                &pipeline_cache.get_bind_group_layout(&noise_pipeline_data.layout),
                &BindGroupEntries::sequential((
                    // Binding the lightmap to the shader
                    &lightmap.0.default_view,
                    &noise_pipeline_data.sampler,
                    settings_binding.clone(),
                )),
            );

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("noise_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    // Setting the output to be EmptyTexture
                    view: &empty_texture.0.default_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_render_pipeline(noise_pipeline);
            render_pass.set_bind_group(0, &noise_bind_group, &[settings_index.index()]);
            render_pass.draw(0..3, 0..1);
        }

        // The second render pass
        {
            let transfer_bind_group = render_context.render_device().create_bind_group(
                "tranfer_bind_group",
                &pipeline_cache.get_bind_group_layout(&transfer_pipeline_data.layout),
                &BindGroupEntries::sequential((
                    // Binding the temporary texture to the shader
                    &empty_texture.0.default_view,
                    &transfer_pipeline_data.sampler,
                )),
            );

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("transfer_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    // Setting the output to be the lightmap
                    view: &lightmap.0.default_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_render_pipeline(transfer_pipeline);
            render_pass.set_bind_group(0, &transfer_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        Ok(())
    }
}

#[derive(Resource)]
struct NoisePipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

fn init_noise_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    // We need to define the bind group layout used for our pipeline
    let layout = BindGroupLayoutDescriptor::new(
        "noise_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // The lightmap texture
                texture_2d(TextureSampleType::Float { filterable: true }),
                // The sampler that will be used to sample the screen texture
                sampler(SamplerBindingType::Filtering),
                // The settings uniform that will control the effect
                uniform_buffer::<NoiseSettings>(true),
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());

    let shader = asset_server.load(NOISE_SHADER_ASSET_PATH);

    let vertex_state = fullscreen_shader.to_vertex_state();
    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("noise_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: vertex_state,
        fragment: Some(FragmentState {
            shader,
            targets: vec![Some(ColorTargetState {
                // NOTE: if not using HDR, change the format to `TextureFormat::bevy_default()`.
                format: ViewTarget::TEXTURE_FORMAT_HDR,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });
    commands.insert_resource(NoisePipeline {
        layout,
        sampler,
        pipeline_id,
    });
}

// This pipeline simply transfers all pixels from one texture into another.
#[derive(Resource)]
struct TransferPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

fn init_transfer_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "transfer_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // The input texture
                texture_2d(TextureSampleType::Float { filterable: true }),
                // The sampler that will be used to sample the screen texture
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let shader = asset_server.load(TRANSFER_SHADER_ASSET_PATH);

    let vertex_state = fullscreen_shader.to_vertex_state();
    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("noise_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: vertex_state,
        fragment: Some(FragmentState {
            shader,
            targets: vec![Some(ColorTargetState {
                // NOTE: if not using HDR, change the format to `TextureFormat::bevy_default()`.
                format: ViewTarget::TEXTURE_FORMAT_HDR,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });
    commands.insert_resource(TransferPipeline {
        layout,
        sampler,
        pipeline_id,
    });
}
