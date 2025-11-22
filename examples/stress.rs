use bevy::{
    color::palettes::css::{BLUE, GREEN, PURPLE, RED},
    prelude::*,
};
use firefly::prelude::*;
use iyes_perf_ui::PerfUiPlugin;
use iyes_perf_ui::entries::PerfUiDefaultEntries;
use rand::{Rng, rng, seq::IndexedRandom};

#[derive(Resource)]
struct Timers {
    light_timer: Timer,
    occluder_timer: Timer,
}

const LIGHT_FREQ: f32 = 1.5;
const OCCLUDER_FREQ: f32 = 0.5;
const HEIGHT: f32 = 15000.0;
const WIDTH: f32 = 30000.0;

impl Default for Timers {
    fn default() -> Self {
        Timers {
            light_timer: Timer::from_seconds(LIGHT_FREQ, TimerMode::Repeating),
            occluder_timer: Timer::from_seconds(OCCLUDER_FREQ, TimerMode::Repeating),
        }
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins((DefaultPlugins, FireflyPlugin, FireflyGizmosPlugin))
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin)
        .add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);

    app.add_plugins(PerfUiPlugin);

    app.add_systems(Startup, setup);

    app.add_systems(Update, change_scale);

    app.add_systems(Update, (spawn_lights, move_lights));
    app.add_systems(Update, (spawn_occluders, move_occluders));

    app.init_resource::<Timers>();

    app.run();
}

fn setup(mut commands: Commands) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = 10.0;

    commands.spawn((
        Camera2d,
        Transform::default(),
        FireflyConfig {
            ambient_color: Color::Srgba(PURPLE),
            ambient_brightness: 0.7,
            light_bands: None,
            softness: None,
            z_sorting: false,
        },
        Projection::Orthographic(proj),
    ));
    commands.spawn(PerfUiDefaultEntries::default());
}

fn change_scale(
    projection: Single<&mut Projection>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Projection::Orthographic(ref mut projection) = *projection.into_inner() else {
        return;
    };

    if keys.pressed(KeyCode::ArrowLeft) {
        projection.scale += 30. * time.delta_secs();
    }
    if keys.pressed(KeyCode::ArrowRight) {
        projection.scale = (projection.scale - 30. * time.delta_secs()).max(0.5);
    }
}

const COLORS: [Color; 3] = [Color::Srgba(RED), Color::Srgba(BLUE), Color::Srgba(GREEN)];

fn spawn_lights(mut commands: Commands, mut timers: ResMut<Timers>, time: Res<Time>) {
    if timers.light_timer.tick(time.delta()).just_finished() {
        let mut rng = rng();

        let x = rng.random_range(-WIDTH / 2.0..WIDTH / 2.0);

        let r = rng.random_range(1000.0..7000.0);

        commands.spawn((
            PointLight2d {
                color: *COLORS.choose(&mut rng).unwrap(),
                intensity: 1.,
                range: r,
                ..default()
            },
            Transform::from_translation(vec3(x, HEIGHT / 2. + r, 0.)),
        ));
    }
}

fn move_lights(
    mut lights: Query<(Entity, &mut Transform, &PointLight2d)>,
    mut gizmos: Gizmos,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (id, mut transform, light) in &mut lights {
        transform.translation.y -= time.delta_secs() * 300.0;
        gizmos.circle_2d(
            Isometry2d::from_translation(transform.translation.truncate()),
            5.,
            light.color,
        );
        if transform.translation.y + light.range < -HEIGHT / 2.0 {
            commands.entity(id).despawn();
        }
    }
}

fn spawn_occluders(mut commands: Commands, mut timers: ResMut<Timers>, time: Res<Time>) {
    if timers.occluder_timer.tick(time.delta()).just_finished() {
        let mut rng = rng();

        let x = rng.random_range(-WIDTH / 2.0..WIDTH / 2.0);

        let occluder_type = rng.random_range(0..4);
        let occluder = match occluder_type {
            0 => Occluder2d::round_rectangle(
                rng.random_range(10.0..30.0),
                rng.random_range(10.0..30.0),
                rng.random_range(10.0..30.0),
            ),
            1 => {
                Occluder2d::polygon(vec![vec2(-20., -10.), vec2(0., 20.), vec2(20., -10.)]).unwrap()
            }
            2 => Occluder2d::polyline(vec![
                vec2(-30., -3.),
                vec2(-20., 2.),
                vec2(-12., -7.),
                vec2(-3., 5.),
                vec2(0., 0.),
                vec2(8., -4.),
                vec2(15., 6.),
                vec2(25., -7.),
                vec2(30., 5.),
            ])
            .unwrap(),
            _ => Occluder2d::polygon(vec![
                vec2(-15., -30.),
                vec2(15., -30.),
                vec2(30., 0.),
                vec2(15., 30.),
                vec2(0., 30.),
                vec2(-15., 30.),
                vec2(-30., 0.),
            ])
            .unwrap(),
        };

        commands.spawn((
            occluder,
            Transform::from_translation(vec3(x, -HEIGHT / 2.0 - rng.random_range(30.0..60.0), 0.))
                .with_rotation(Quat::from_rotation_z(rng.random_range(-4.0..4.0))),
        ));
    }
}

fn move_occluders(
    mut occluders: Query<(Entity, &mut Transform), With<Occluder2d>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (id, mut transform) in &mut occluders {
        transform.translation.y += time.delta_secs() * 700.0;

        if transform.translation.y > HEIGHT / 2.0 + 60. {
            commands.entity(id).despawn();
        }

        transform.rotate_z(3. * time.delta_secs());
    }
}
