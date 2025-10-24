use bevy::{
    asset::{load_internal_asset, weak_handle},
    color::palettes::css::{BLUE, PINK},
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    prelude::*,
    render::{
        RenderApp,
        render_graph::{RenderGraphApp, RenderLabel, ViewNodeRunner},
        sync_world::SyncToRenderWorld,
        texture::CachedTexture,
        view::{VisibilityClass, visibility},
    },
};

use crate::{
    extract::ExtractPlugin,
    nodes::{ApplyLightmapNode, CreateLightmapNode},
    pipelines::{LightmapApplicationPipeline, LightmapCreationPipeline, TransferTexturePipeline},
    prepare::{LightingData, OccluderMeta, PreparePlugin, Vertex},
};

mod extract;
mod nodes;
mod pipelines;
mod prepare;

#[derive(Component, Reflect)]
#[require(SyncToRenderWorld, VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<PointLight>)]
pub struct Occluder {
    pub shape: OccluderShape,
}

impl Occluder {
    pub(crate) fn vertices(&self) -> Vec<Vec2> {
        match self.shape {
            OccluderShape::Rectangle { width, height } => {
                let corner = vec2(width / 2., height / 2.);
                vec![
                    vec2(corner.x, corner.y),
                    vec2(corner.x, -corner.y),
                    vec2(-corner.x, -corner.y),
                    vec2(-corner.x, corner.y),
                ]
            }
        }
    }
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
struct LightMapTexture(pub CachedTexture);

#[derive(Component)]
struct IntermediaryLightMapTexture(pub CachedTexture);

#[derive(Component)]
struct EmptyLightMapTexture(pub CachedTexture);

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CreateLightmapLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ApplyLightmapLabel;

const CREATE_LIGHTMAP_SHADER: Handle<Shader> = weak_handle!("6e9647ff-b9f8-41ce-8d83-9bd91ae31898");
const APPLY_LIGHTMAP_SHADER: Handle<Shader> = weak_handle!("72c4f582-83b6-47b6-a200-b9f0e408df72");
const TRANSFER_SHADER: Handle<Shader> = weak_handle!("206fb81e-58e7-4dd0-b4f5-c39892e23fc6");
const TYPES_SHADER: Handle<Shader> = weak_handle!("dac0fb7e-a64f-4923-8e31-6912f3fc8551");
const UTILS_SHADER: Handle<Shader> = weak_handle!("1471f256-f404-4388-bb2f-ca6b8047ef7e");

pub struct FireflyPlugin;

impl Plugin for FireflyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_gizmos);

        load_internal_asset!(
            app,
            CREATE_LIGHTMAP_SHADER,
            "../shaders/create_lightmap.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            APPLY_LIGHTMAP_SHADER,
            "../shaders/apply_lightmap.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            TRANSFER_SHADER,
            "../shaders/transfer.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            TYPES_SHADER,
            "../shaders/types.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            UTILS_SHADER,
            "../shaders/utils.wgsl",
            Shader::from_wgsl
        );

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
        render_app.init_resource::<TransferTexturePipeline>();
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
