//! This example demonstrates how to apply a shader over the lightmap before it is being applied to the camera's output.
//!
//! This is greatly inspired by Bevy's post processing example: https://bevy.org/examples/shaders/custom-post-processing/.
//! Check out that example for more in-depth comments and explanations.

use bevy::{
    camera::{CameraOutputMode, ManualTextureViewHandle, RenderTarget, visibility::RenderLayers},
    color::palettes::css::{RED, WHITE},
    core_pipeline::{FullscreenShader, core_2d::graph::Core2d, prepass::DepthPrepass},
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        Extract, Render, RenderApp, RenderStartup, RenderSystems,
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
        sync_world::RenderEntity,
        texture::{CachedTexture, TextureCache},
        view::{ExtractedView, Hdr, ViewTarget},
    },
    window::{PrimaryWindow, WindowRef},
};
use bevy_firefly::{ApplyLightmapLabel, CreateLightmapLabel, LightMapTexture, prelude::*};

// The shader that will apply the noise over the lightmap.
const MULTIPLY_SHADER_ASSET_PATH: &str = "shaders/multiply.wgsl";

// Simple shader which writes the pixels of a texture unto another texture.
const TRANSFER_SHADER_ASSET_PATH: &str = "shaders/transfer.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FireflyPlugin, FOVPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, move_light)
        .run();
}

#[derive(Component, Clone)]
struct MainCamera;

#[derive(Component, Clone)]
struct VisibilityCamera;

#[derive(Component)]
struct RedLight;

fn setup(mut commands: Commands) {
    commands.spawn((
        MainCamera,
        Camera2d,
        Hdr,
        FireflyConfig::default(),
        RenderLayers::layer(0),
    ));

    commands.spawn((
        VisibilityCamera,
        Camera2d,
        Camera {
            order: -1,
            output_mode: CameraOutputMode::Skip,
            ..default()
        },
        FireflyConfig::default(),
        RenderLayers::layer(1),
    ));

    commands.spawn((
        RedLight,
        PointLight2d {
            color: Color::Srgba(RED),
            intensity: 10.0,
            radius: 100.0,
            ..default()
        },
        Transform::from_translation(vec3(-30.0, 0.0, 0.0)),
        RenderLayers::layer(0),
    ));

    commands.spawn((
        PointLight2d {
            intensity: 10.0,
            radius: 50.0,
            falloff: Falloff::None,
            ..default()
        },
        Transform::from_translation(vec3(30.0, 0.0, 0.0)),
        RenderLayers::layer(1),
    ));
}

#[derive(Component)]
pub struct ExtractedMainCamera {
    pub visibility_camera: Entity,
}

fn move_light(
    mut light: Single<&mut Transform, With<RedLight>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
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

struct FOVPlugin;

impl Plugin for FOVPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(ExtractSchedule, extract_visibility_camera);

        render_app
            .add_systems(RenderStartup, (init_fov_pipeline, init_transfer_pipeline))
            .add_systems(Render, prepare_empty_texture.in_set(RenderSystems::Prepare))
            .add_render_graph_node::<ViewNodeRunner<FOVNode>>(Core2d, FOVLabel)
            .add_render_graph_edges(
                Core2d,
                // `NoiseLabel` is added between `CreateLightmapLabel` and `ApplyLightmapLabel`.
                // This makes our render pass execute after the lightmap is created but before it is applied to the camera.
                (CreateLightmapLabel, FOVLabel, ApplyLightmapLabel),
            );
    }
}

fn extract_visibility_camera(
    main_camera: Extract<Single<RenderEntity, With<MainCamera>>>,
    visibility_camera: Extract<Single<RenderEntity, With<VisibilityCamera>>>,
    mut commands: Commands,
) {
    commands.entity(**main_camera).insert(ExtractedMainCamera {
        visibility_camera: **visibility_camera,
    });
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct FOVLabel;

#[derive(Default)]
struct FOVNode;

impl ViewNode for FOVNode {
    type ViewQuery = (
        // We need to query the lightmap texture in order to change it.
        Read<LightMapTexture>,
        Read<EmptyTexture>,
        Read<ExtractedMainCamera>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (lightmap, empty_texture, main_camera): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let fov_pipeline_data = world.resource::<FOVPipeline>();
        let transfer_pipeline_data = world.resource::<TransferPipeline>();

        let pipeline_cache = world.resource::<PipelineCache>();

        let (Some(fov_pipeline), Some(transfer_pipeline)) = (
            pipeline_cache.get_render_pipeline(fov_pipeline_data.pipeline_id),
            pipeline_cache.get_render_pipeline(transfer_pipeline_data.pipeline_id),
        ) else {
            return Ok(());
        };

        info!("looking on {}", main_camera.visibility_camera);
        let visibility_lightmap = world
            .entity(main_camera.visibility_camera)
            .get_components::<&LightMapTexture>()
            .unwrap();
        // Reading a texture and outputting to it at the same time is not supported, so two render passes are made:
        // One to read the lightmap, apply the noise, and output to a temporary texture (`EmptyTexture`),
        // and another to read from the temporary texture and output back to the lightmap.
        //
        // This is why `TransferPipeline` and `transfer.wgsl` are needed.

        // The first render pass
        {
            let fov_bind_group = render_context.render_device().create_bind_group(
                "fov_bind_group",
                &pipeline_cache.get_bind_group_layout(&fov_pipeline_data.layout),
                &BindGroupEntries::sequential((
                    // Binding the lightmap to the shader
                    &lightmap.0.default_view,
                    &visibility_lightmap.0.default_view,
                    &fov_pipeline_data.sampler,
                )),
            );

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("fov_pass"),
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
            render_pass.set_render_pipeline(fov_pipeline);
            render_pass.set_bind_group(0, &fov_bind_group, &[]);
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
struct FOVPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

fn init_fov_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    // We need to define the bind group layout used for our pipeline
    let layout = BindGroupLayoutDescriptor::new(
        "fov_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // The main lightmap texture
                texture_2d(TextureSampleType::Float { filterable: true }),
                // The visibility lightmap texture
                texture_2d(TextureSampleType::Float { filterable: true }),
                // The sampler that will be used to sample the screen texture
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());

    let shader = asset_server.load(MULTIPLY_SHADER_ASSET_PATH);

    let vertex_state = fullscreen_shader.to_vertex_state();
    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("fov_pipeline".into()),
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
    commands.insert_resource(FOVPipeline {
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
