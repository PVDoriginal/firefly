use bevy::{
    color::palettes::css::{BLUE, RED},
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    render::view::Hdr,
};
use bevy_firefly::prelude::*;
use rand::{Rng, rng};

const GRID_SIZE: (usize, usize) = (100, 100);
const BLOCK_SIZE: f32 = 10.0;
const FREQ_BLOCK: f32 = 0.4;
const FREQ_LIGHT: f32 = 0.05;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()));
    app.add_plugins((FireflyPlugin,));

    app.add_systems(Startup, setup);
    app.add_systems(Update, move_camera);

    app.add_plugins((
        LogDiagnosticsPlugin::default(),
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin::default(),
        // SystemInformationDiagnosticsPlugin,
        // bevy::render::diagnostic::RenderDiagnosticsPlugin,
    ));

    // app.add_systems(
    //     Update,
    //     update_commands.run_if(
    //         resource_exists_and_changed::<LogDiagnosticsStatus>
    //             .or(resource_exists_and_changed::<LogDiagnosticsFilters>),
    //     ),
    // );

    app.run();
}

fn setup(mut commands: Commands) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = 0.15;

    commands.spawn((
        Camera2d,
        Hdr::default(),
        Projection::Orthographic(proj),
        FireflyConfig {
            ambient_brightness: 0.2,
            ..default()
        },
    ));

    let mut rng = rng();

    let start_pos = -vec2(GRID_SIZE.0 as f32, GRID_SIZE.1 as f32) * 0.5 * BLOCK_SIZE;

    for i in 0..GRID_SIZE.0 {
        for j in 0..GRID_SIZE.1 {
            let r: f32 = rng.random();

            let pos = (start_pos + vec2(BLOCK_SIZE * i as f32, BLOCK_SIZE * j as f32)).extend(0.0);

            if r < FREQ_BLOCK {
                commands.spawn((
                    Occluder2d::rectangle(BLOCK_SIZE, BLOCK_SIZE),
                    Transform::from_translation(pos),
                ));
            } else if r < FREQ_LIGHT + FREQ_BLOCK {
                commands.spawn((
                    PointLight2d {
                        color: Color::Srgba(RED),
                        intensity: 10.0,
                        radius: BLOCK_SIZE * 4.0,
                        ..default()
                    },
                    Transform::from_translation(pos),
                ));
            }
        }
    }
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

    if keys.just_pressed(KeyCode::Space) {
        camera.translation = Vec3::ZERO;
    }
}
