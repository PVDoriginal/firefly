use bevy::{color::palettes::css::RED, prelude::*};
use bevy_firefly::prelude::*;

// Very basic example. Spawns a red light in the center of the screen, and a few round occluders surrounding it.
// You can press the arrow keys to move the light.

fn main() {
    let mut app = App::new();

    app.add_plugins((DefaultPlugins, FireflyPlugin, FireflyGizmosPlugin));
    app.add_systems(Startup, setup);
    app.add_systems(Update, move_light);

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, FireflyConfig::default(), Transform::default()));
    commands.spawn((
        PointLight2d {
            color: Color::Srgba(RED),
            intensity: 1.0,
            range: 200.,
            ..default()
        },
        Transform::default(),
    ));

    commands.spawn((
        Occluder2d::circle(20.0),
        Transform::from_translation(vec3(60., 0., 0.)),
    ));
    commands.spawn((
        Occluder2d::circle(20.0),
        Transform::from_translation(vec3(-60., 0., 0.)),
    ));
    commands.spawn((
        Occluder2d::circle(20.0),
        Transform::from_translation(vec3(0., 60., 0.)),
    ));
    commands.spawn((
        Occluder2d::circle(20.0),
        Transform::from_translation(vec3(0., -60., 0.)),
    ));
}

fn move_light(
    mut light: Single<&mut Transform, With<PointLight2d>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut gizmos: Gizmos,
) {
    let speed = 30.;

    gizmos.circle_2d(
        Isometry2d::from_translation(light.translation.truncate()),
        5.,
        RED,
    );

    if keys.pressed(KeyCode::ArrowLeft) {
        light.translation += vec3(-speed * time.delta_secs(), 0., 0.);
    }
    if keys.pressed(KeyCode::ArrowRight) {
        light.translation += vec3(speed * time.delta_secs(), 0., 0.);
    }
    if keys.pressed(KeyCode::ArrowUp) {
        light.translation += vec3(0., speed * time.delta_secs(), 0.);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        light.translation += vec3(0., -speed * time.delta_secs(), 0.);
    }
}
