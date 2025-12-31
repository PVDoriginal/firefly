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
            softness: Some(0.5),
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

    // occluders

    commands.spawn((
        Occluder2d::rectangle(22.0, 14.0),
        Transform::from_translation(vec3(-322., 164., 0.)),
    ));

    commands.spawn((
        Occluder2d::rectangle(10., 10.),
        Transform::from_translation(vec3(-166., 56., 0.)),
    ));

    commands.spawn((
        Occluder2d::rectangle(10., 10.),
        Transform::from_translation(vec3(-417., 106., 0.)),
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

    commands.spawn((
        Occluder2d::polyline(vec![
            vec2(-97., 108.),
            vec2(-58., 163.),
            vec2(-25., 105.),
            vec2(-109., 53.),
        ])
        .unwrap(),
        Transform::default(),
    ));

    commands.spawn((
        Occluder2d::polygon(vec![
            vec2(-358., 276.),
            vec2(-342., 208.),
            vec2(-429., 172.),
            vec2(-428., 135.),
            vec2(-482., 158.),
            vec2(-475., 231.),
            vec2(-438., 290.),
            vec2(-388., 299.),
            vec2(-380., 278.),
        ])
        .unwrap(),
        Transform::default(),
    ));

    commands.spawn((
        Occluder2d::circle(23.),
        Transform::from_translation(vec3(-216., -33., 0.)),
    ));

    commands.spawn((
        Occluder2d::rectangle(69., 10.),
        Transform::from_translation(vec3(-387., 81., 0.)),
    ));

    commands.spawn((
        Occluder2d::polygon(vec![
            vec2(-249., 243.),
            vec2(-262., 163.),
            vec2(-161., 147.),
            vec2(-135., 237.),
            vec2(-216., 261.),
        ])
        .unwrap(),
        Transform::default(),
    ));

    commands.spawn((
        Occluder2d::round_rectangle(53., 38., 23.),
        Transform::from_translation(vec3(-58., -1., 0.)),
    ));

    commands.spawn((
        Occluder2d::rectangle(16., 76.),
        Transform::from_translation(vec3(-18., 211., 0.)),
    ));

    commands.spawn((
        Occluder2d::rectangle(10., 20.),
        Transform::from_translation(vec3(-335., 133., 0.)),
    ));

    commands.spawn((
        Occluder2d::rectangle(15., 40.),
        Transform::from_translation(vec3(-258., 278., 0.)),
    ));

    commands.spawn((
        Occluder2d::rectangle(10., 10.),
        Transform::from_translation(vec3(-203., 91., 0.)),
    ));

    commands.spawn((
        Occluder2d::capsule(65., 11.),
        Transform::from_translation(vec3(-127., -102., 0.))
            .with_rotation(Quat::from_rotation_z(0.5)),
    ));

    commands.spawn((
        Occluder2d::rectangle(10., 10.),
        Transform::from_translation(vec3(-109., 210., 0.)),
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
