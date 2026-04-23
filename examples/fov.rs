//! This example demonstrates how to apply a shader over the lightmap before it is being applied to the camera's output.
//!
//! This is greatly inspired by Bevy's post processing example: https://bevy.org/examples/shaders/custom-post-processing/.
//! Check out that example for more in-depth comments and explanations.

use bevy::{
    camera::{CameraOutputMode, visibility::RenderLayers},
    color::palettes::css::RED,
    prelude::*,
    render::view::Hdr,
    window::PrimaryWindow,
};
use bevy_firefly::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FireflyPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, move_light)
        .run();
}

#[derive(Component, Clone)]
struct MainCamera;

#[derive(Component)]
struct RedLight;

fn setup(mut commands: Commands) {
    let main_camera = commands
        .spawn((
            MainCamera,
            Camera2d,
            Camera {
                msaa_writeback: MsaaWriteback::Off,
                ..default()
            },
            Hdr,
            FireflyConfig::default(),
            RenderLayers::layer(0),
        ))
        .id();

    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            output_mode: CameraOutputMode::Skip,
            ..default()
        },
        FireflyConfig::default(),
        RenderLayers::layer(1),
        CombineLightmapTo(main_camera),
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
            intensity: 1.0,
            radius: 50.0,
            falloff: Falloff::None,
            core: LightCore::from_radius_boost(0.0, 0.0),
            ..default()
        },
        Transform::from_translation(vec3(30.0, 0.0, 0.0)),
        RenderLayers::layer(1),
    ));
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
