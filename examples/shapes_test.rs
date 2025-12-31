use bevy::{color::palettes::css::RED, prelude::*, window::PrimaryWindow};
use bevy_firefly::prelude::*;

// A simple example showcasing the different occluder shapes.
// You can click around the screen to reposition the light.

// This example also showcases hard (non-soft) shadows and light banding.

fn main() {
    let mut app = App::new();

    app.add_plugins((DefaultPlugins, FireflyPlugin, FireflyGizmosPlugin));
    app.add_systems(Startup, setup);
    app.add_systems(Update, (move_light, move_camera, despawn_debug));

    app.run();
}

fn setup(mut commands: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scale = 0.7;

    // camera
    commands.spawn((
        Camera2d,
        Projection::Orthographic(projection),
        FireflyConfig {
            ambient_brightness: 0.3,
            softness: None,
            light_bands: Some(4),
            ..default()
        },
        Transform::from_translation(vec3(-230., 75., 0.)),
    ));

    // light
    commands.spawn((
        PointLight2d {
            color: Color::srgb(1.0, 0.5, 1.0),
            intensity: 1.0,
            range: 250.,
            ..default()
        },
        Transform::default(),
    ));

    commands.spawn((
        Occluder2d::polygon(vec![
            vec2(-255., 91.),
            vec2(-237., 33.),
            vec2(-302., 6.),
            vec2(-358., 45.),
            vec2(-329., 81.),
            vec2(-289., 99.),
        ])
        .unwrap(),
        Transform::default(),
    ));
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

    info!("cursor: {cursor_position:?}");

    gizmos.circle_2d(Isometry2d::from_translation(cursor_position), 5., RED);

    light.translation = cursor_position.extend(0.);
}

const CAMERA_SPEED: f32 = 60.0;
fn move_camera(
    mut camera: Single<&mut Transform, With<FireflyConfig>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    if keys.pressed(KeyCode::KeyA) {
        camera.translation.x -= time.delta_secs() * CAMERA_SPEED;
    }
    if keys.pressed(KeyCode::KeyD) {
        camera.translation.x += time.delta_secs() * CAMERA_SPEED;
    }
    if keys.pressed(KeyCode::KeyS) {
        camera.translation.y -= time.delta_secs() * CAMERA_SPEED;
    }
    if keys.pressed(KeyCode::KeyW) {
        camera.translation.y += time.delta_secs() * CAMERA_SPEED;
    }
}

fn despawn_debug(
    occluders: Query<Entity, With<Occluder2d>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if keys.pressed(KeyCode::Enter) {
        if let Some(entity) = occluders.iter().last() {
            commands.entity(entity).despawn();
        }
    }
}
