use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::{
    color::palettes::css::{BLUE, RED},
    input::mouse::MouseWheel,
    prelude::*,
    window::PrimaryWindow,
};
use bevy_firefly::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins((DefaultPlugins, FireflyPlugin, FireflyGizmosPlugin))
        .insert_resource(FireflyGizmoStyle {
            light_inner_color: Color::NONE,
            light_outer_color: Color::NONE,
            ..default()
        });

    app.add_systems(Startup, setup)
        .add_systems(Update, (rotate_occluders, move_light));

    app.run();
}

#[derive(Component)]
#[require(Transform)]
struct OccluderParent;

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        FireflyConfig {
            ambient_brightness: 0.3,
            ..default()
        },
    ));

    commands.spawn(PointLight2d {
        color: Color::Srgba(Srgba::rgb(0.8, 0.2, 0.3)),
        intensity: 10.0,
        radius: 400.0,
        core: LightCore::from_radius_boost(20.0, 10.0),
        angle: LightAngle {
            inner: 30.0,
            outer: 50.0,
        },
        ..default()
    });

    let unit_size = 30.0;

    let angle_step = TAU / 7.0;

    let radius = 150.0;

    let angles: Vec<f32> = (0..7).rev().map(|i| i as f32 * angle_step).collect();

    commands.spawn(OccluderParent).with_children(|spawner| {
        // F
        spawner.spawn((
            Occluder2d::polygon(f(unit_size)).unwrap(),
            transform(angles[0], radius),
        ));

        // I
        spawner
            .spawn((
                Occluder2d::polygon(i(unit_size)).unwrap(),
                transform(angles[1], radius),
            ))
            .with_child({
                (
                    Occluder2d::circle(unit_size * 0.5),
                    Transform::from_translation(vec3(0.0, 4.0 * unit_size, 0.0)),
                )
            });

        // R
        spawner.spawn((
            Occluder2d::polygon(r(unit_size)).unwrap(),
            transform(angles[2], radius),
        ));

        // E
        spawner.spawn((
            Occluder2d::polygon(e(unit_size)).unwrap(),
            transform(angles[3], radius),
        ));

        // F
        spawner.spawn((
            Occluder2d::polygon(f(unit_size)).unwrap(),
            transform(angles[4], radius),
        ));

        // L
        spawner.spawn((
            Occluder2d::polygon(l(unit_size)).unwrap(),
            transform(angles[5], radius),
        ));

        // Y
        spawner.spawn((
            Occluder2d::polygon(y(unit_size)).unwrap(),
            transform(angles[6], radius),
        ));
    });
}

fn rotate_occluders(mut parent: Single<&mut Transform, With<OccluderParent>>, time: Res<Time>) {
    parent.rotate_z(time.delta_secs() * 0.5);
}

fn transform(angle: f32, radius: f32) -> Transform {
    Transform::from_translation(vec3(angle.cos() * radius, angle.sin() * radius, 0.0))
        .with_rotation(Quat::from_rotation_z(angle - FRAC_PI_2))
}

fn f(unit_size: f32) -> Vec<Vec2> {
    vec![
        vec2(-1.0, 0.0),
        vec2(-1.0, 5.0),
        vec2(1.0, 5.0),
        vec2(1.0, 4.0),
        vec2(0.0, 4.0),
        vec2(0.0, 3.0),
        vec2(1.0, 3.0),
        vec2(1.0, 2.0),
        vec2(0.0, 2.0),
        vec2(0.0, 0.0),
    ]
    .iter()
    .map(|v| v * unit_size)
    .collect()
}

fn i(unit_size: f32) -> Vec<Vec2> {
    vec![
        vec2(-0.5, 0.0),
        vec2(-0.5, 3.0),
        vec2(0.5, 3.0),
        vec2(0.5, 0.0),
    ]
    .iter()
    .map(|v| v * unit_size)
    .collect()
}

fn r(unit_size: f32) -> Vec<Vec2> {
    vec![
        vec2(-1.5, 0.0),
        vec2(-1.5, 5.0),
        vec2(0.5, 5.0),
        vec2(1.65, 3.5),
        vec2(1.0, 2.0),
        vec2(2.0, 0.0),
        vec2(1.0, 0.0),
        vec2(0.0, 2.0),
        vec2(-0.5, 2.0),
        vec2(-0.5, 0.0),
    ]
    .iter()
    .map(|v| v * unit_size)
    .collect()
}

fn e(unit_size: f32) -> Vec<Vec2> {
    vec![
        vec2(-1.0, 0.0),
        vec2(-1.0, 5.0),
        vec2(1.0, 5.0),
        vec2(1.0, 4.0),
        vec2(0.0, 4.0),
        vec2(0.0, 3.0),
        vec2(1.0, 3.0),
        vec2(1.0, 2.0),
        vec2(0.0, 2.0),
        vec2(0.0, 1.0),
        vec2(1.0, 1.0),
        vec2(1.0, 0.0),
    ]
    .iter()
    .map(|v| v * unit_size)
    .collect()
}

fn l(unit_size: f32) -> Vec<Vec2> {
    vec![
        vec2(-1.0, 0.0),
        vec2(-1.0, 5.0),
        vec2(0.0, 5.0),
        vec2(0.0, 1.0),
        vec2(2.0, 1.0),
        vec2(2.0, 0.0),
    ]
    .iter()
    .map(|v| v * unit_size)
    .collect()
}

fn y(unit_size: f32) -> Vec<Vec2> {
    vec![
        vec2(-0.5, 0.0),
        vec2(-0.5, 2.0),
        vec2(-1.5, 5.0),
        vec2(-0.5, 5.0),
        vec2(0.0, 3.0),
        vec2(0.5, 5.0),
        vec2(1.5, 5.0),
        vec2(0.5, 2.0),
        vec2(0.5, 0.0),
    ]
    .iter()
    .map(|v| v * unit_size)
    .collect()
}

fn move_light(
    mut light: Single<(&mut Transform, &mut PointLight2d)>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut scroll: MessageReader<MouseWheel>,
    mut gizmos: Gizmos,
) {
    for scroll_event in scroll.read() {
        light.1.core.radius += scroll_event.y * 5.0;
        light.1.core.radius = light.1.core.radius.max(0.0);
    }

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

    light.0.translation = cursor_position.extend(0.);
}
