use std::f32::consts::{FRAC_2_PI, FRAC_PI_2};

use bevy::{
    asset::load_internal_asset,
    color::palettes::css::{BLUE, PINK},
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    prelude::*,
    render::{
        RenderApp,
        render_graph::{RenderGraphApp, ViewNodeRunner},
    },
};

use crate::{
    extract::ExtractPlugin,
    nodes::{ApplyLightmapNode, CreateLightmapNode},
    occluders::OccluderShape,
    pipelines::{LightmapApplicationPipeline, LightmapCreationPipeline, TransferTexturePipeline},
    sprites::{SpriteStencilLabel, SpritesPlugin},
    *,
};
use crate::{prelude::*, prepare::PreparePlugin};

pub struct FireflyPlugin;

impl Plugin for FireflyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<crate::prelude::PointLight>();
        app.register_type::<FireflyConfig>();

        // app.add_systems(Startup, stress_test);

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
        load_internal_asset!(
            app,
            SPRITE_SHADER,
            "../shaders/sprite.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins((PreparePlugin, ExtractPlugin));
        app.add_plugins(SpritesPlugin);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<CreateLightmapNode>>(
                Core2d,
                CreateLightmapLabel,
            )
            .add_render_graph_node::<ViewNodeRunner<ApplyLightmapNode>>(Core2d, ApplyLightmapLabel);

        // render_app.add_render_graph_edges(Core2d, (, CreateLightmapLabel));

        render_app.add_render_graph_edges(
            Core2d,
            (
                Node2d::MainTransparentPass,
                SpriteStencilLabel,
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

fn stress_test(mut commands: Commands) {
    for _ in 0..5 {
        commands.spawn((
            Name::new("Point Light"),
            lights::PointLight::default(),
            Transform::default(),
        ));
    }
}

pub struct FireflyGizmosPlugin;
impl Plugin for FireflyGizmosPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_gizmos);
    }
}

const GIZMO_COLOR: Color = bevy::prelude::Color::Srgba(PINK);

fn draw_gizmos(
    mut gizmos: Gizmos,
    lights: Query<&Transform, With<crate::prelude::PointLight>>,
    occluders: Query<(&Transform, &Occluder)>,
) {
    for transform in &lights {
        let isometry = Isometry2d::from_translation(transform.translation.truncate());
        gizmos.circle_2d(isometry, 10., BLUE);
    }

    for (transform, occluder) in &occluders {
        let isometry = Isometry2d::from_translation(transform.translation.truncate());

        match occluder.shape().clone() {
            OccluderShape::Rectangle { width, height } => {
                gizmos.rect_2d(isometry, vec2(width, height), GIZMO_COLOR);
            }
            OccluderShape::Polygon { vertices, .. } => {
                for line in vertices.windows(2) {
                    gizmos.line_2d(line[0], line[1], GIZMO_COLOR);
                }
                gizmos.line_2d(vertices[0], vertices[vertices.len() - 1], GIZMO_COLOR);
            }
            OccluderShape::Polyline { vertices, .. } => {
                for line in vertices.windows(2) {
                    gizmos.line_2d(line[0], line[1], GIZMO_COLOR);
                }
            }
            OccluderShape::RoundRectangle {
                width,
                height,
                radius,
            } => {
                let center = transform.translation.truncate();
                let width = width / 2.;
                let height = height / 2.;

                // top line
                gizmos.line_2d(
                    center + vec2(-width, height + radius),
                    center + vec2(width, height + radius),
                    GIZMO_COLOR,
                );

                // right line
                gizmos.line_2d(
                    center + vec2(width + radius, height),
                    center + vec2(width + radius, -height),
                    GIZMO_COLOR,
                );

                // bottom line
                gizmos.line_2d(
                    center + vec2(-width, -height - radius),
                    center + vec2(width, -height - radius),
                    GIZMO_COLOR,
                );

                // left line
                gizmos.line_2d(
                    center + vec2(-width - radius, height),
                    center + vec2(-width - radius, -height),
                    GIZMO_COLOR,
                );

                // top-left arc
                gizmos.arc_2d(
                    Isometry2d {
                        translation: center + vec2(-width, height),
                        rotation: Rot2 { cos: 1., sin: 0. },
                    },
                    FRAC_PI_2,
                    radius,
                    GIZMO_COLOR,
                );

                // top-right arc
                gizmos.arc_2d(
                    Isometry2d {
                        translation: center + vec2(width, height),
                        rotation: Rot2 { cos: 0., sin: -1. },
                    },
                    FRAC_PI_2,
                    radius,
                    GIZMO_COLOR,
                );

                // bottom-right arc
                gizmos.arc_2d(
                    Isometry2d {
                        translation: center + vec2(width, -height),
                        rotation: Rot2 { cos: -1., sin: 0. },
                    },
                    FRAC_PI_2,
                    radius,
                    GIZMO_COLOR,
                );

                // bottom-left arc
                gizmos.arc_2d(
                    Isometry2d {
                        translation: center + vec2(-width, -height),
                        rotation: Rot2 { cos: 0., sin: 1. },
                    },
                    FRAC_PI_2,
                    radius,
                    GIZMO_COLOR,
                );
            }
        }
    }
}
