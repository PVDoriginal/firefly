use bevy::{
    color::palettes::css::{BLUE, PINK},
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    prelude::*,
    render::{
        RenderApp, RenderSet,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{NodeRunError, RenderGraphApp, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, PipelineCache, RenderPassColorAttachment,
            RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderStages, ShaderType, TextureFormat, TextureSampleType,
            binding_types::{sampler, texture_2d, uniform_buffer},
        },
        renderer::RenderDevice,
        view::ViewTarget,
    },
};

#[derive(Component, Reflect)]
pub struct Occluder {
    pub shape: OccluderShape,
}

#[derive(Reflect)]
pub enum OccluderShape {
    Rectangle { width: f32, height: f32 },
}

#[derive(Component, Reflect)]
pub struct Light {
    pub shape: LightShape,
}

#[derive(Reflect)]
pub enum LightShape {
    Point,
}

pub struct FireflyPlugin;

impl Plugin for FireflyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_gizmos);

        app.add_plugins(ExtractComponentPlugin::<LightingSettings>::default());
        app.add_plugins(UniformComponentPlugin::<LightingSettings>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<LightingNode>>(Core2d, LightingLabel)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    LightingLabel,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<LightingPipeline>();
    }
}

fn draw_gizmos(
    mut gizmos: Gizmos,
    lights: Query<&Transform, With<Light>>,
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct LightingLabel;

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct LightingSettings {
    pub intensity: f32,
    // WebGL2 structs must be 16 byte aligned.
    #[cfg(feature = "webgl2")]
    _webgl2_padding: Vec3,
}

#[derive(Default)]
struct LightingNode;

impl ViewNode for LightingNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static LightingSettings,
        &'static DynamicUniformIndex<LightingSettings>,
    );

    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        (view_target, _, settings_index): bevy::ecs::query::QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let lighting_pipeline = world.resource::<LightingPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(lighting_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<LightingSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "lighting_bind_group",
            &lighting_pipeline.layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &lighting_pipeline.sampler,
                settings_binding.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("lighting_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
struct LightingPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}
const SHADER_ASSET_PATH: &str = "shaders/post_processing.wgsl";

impl FromWorld for LightingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // We need to define the bind group layout used for our pipeline
        let layout = render_device.create_bind_group_layout(
            "lighting_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // The sampler that will be used to sample the screen texture
                    sampler(SamplerBindingType::Filtering),
                    // The settings uniform that will control the effect
                    uniform_buffer::<LightingSettings>(true),
                ),
            ),
        );
        // We can create the sampler here since it won't change at runtime and doesn't depend on the view
        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        // Get the shader handle
        let shader = world.load_asset(SHADER_ASSET_PATH);
        // This will setup a fullscreen triangle for the vertex state.
        let pipeline_id = world
            .resource_mut::<PipelineCache>()
            // This will add the pipeline to the cache and queue its creation
            .queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("post_process_pipeline".into()),
                layout: vec![layout.clone()],
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader,
                    // Make sure this matches the entry point of your shader.
                    // It can be anything as long as it matches here and in the shader.
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::bevy_default(),
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                    shader_defs: default(),
                    entry_point: "fragment".into(),
                }),
                push_constant_ranges: default(),
                primitive: default(),
                depth_stencil: default(),
                multisample: default(),
                zero_initialize_workgroup_memory: default(),
            });

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}
