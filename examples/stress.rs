use bevy::{
    color::palettes::{
        self,
        css::{BLUE, GREEN, PURPLE, RED},
    },
    diagnostic::{
        DiagnosticPath, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        LogDiagnosticsPlugin, LogDiagnosticsState, SystemInformationDiagnosticsPlugin,
    },
    prelude::*,
};
use bevy_firefly::prelude::*;
use rand::{Rng, rng, seq::IndexedRandom};

#[derive(Resource)]
struct Timers {
    light_timer: Timer,
    occluder_timer: Timer,
}

const LIGHT_FREQ: f32 = 2.2;
const OCCLUDER_FREQ: f32 = 0.1;
const HEIGHT: f32 = 20000.0;
const WIDTH: f32 = 40000.0;

const MOVE_FREQ: f32 = 1.0;

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

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: bevy::window::PresentMode::Immediate,
                ..default()
            }),
            ..default()
        }),
        FireflyPlugin,
        FireflyGizmosPlugin,
    ));

    app.add_plugins((
        LogDiagnosticsPlugin::default(),
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin::default(),
        // SystemInformationDiagnosticsPlugin,
        // bevy::render::diagnostic::RenderDiagnosticsPlugin,
    ));

    app.add_systems(Startup, setup);

    app.add_systems(Update, (change_scale, move_camera));

    app.add_systems(Update, (spawn_lights, move_lights));
    app.add_systems(Update, (spawn_occluders, move_occluders));

    app.add_systems(Update, filters_inputs).add_systems(
        Update,
        update_commands.run_if(
            resource_exists_and_changed::<LogDiagnosticsStatus>
                .or(resource_exists_and_changed::<LogDiagnosticsFilters>),
        ),
    );

    app.init_resource::<Timers>();
    app.init_resource::<LogDiagnosticsStatus>();
    app.init_resource::<LogDiagnosticsFilters>();

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
            ..default()
        },
        Projection::Orthographic(proj),
    ));
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
        projection.scale += 5. * time.delta_secs();
    }
    if keys.pressed(KeyCode::ArrowRight) {
        projection.scale = (projection.scale - 5. * time.delta_secs()).max(0.5);
    }
}

const CAMERA_SPEED: f32 = 2000.0;
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

const COLORS: [Color; 3] = [Color::Srgba(RED), Color::Srgba(BLUE), Color::Srgba(GREEN)];

fn spawn_lights(mut commands: Commands, mut timers: ResMut<Timers>, time: Res<Time>) {
    if timers.light_timer.tick(time.delta()).just_finished() {
        let mut rng = rng();

        let x = rng.random_range(-WIDTH / 2.0..WIDTH / 2.0);

        let r = rng.random_range(1000.0..10000.0);

        commands.spawn((
            PointLight2d {
                color: *COLORS.choose(&mut rng).unwrap(),
                intensity: 1.,
                range: r,
                // cast_shadows: false,
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
    let mut rng = rng();
    for (id, mut transform, light) in &mut lights {
        let r = rng.random_range(0.0..1.0);

        if r <= MOVE_FREQ {
            transform.translation.y -= time.delta_secs() * 600.0;

            if transform.translation.y + light.range < -HEIGHT / 2.0 {
                commands.entity(id).despawn();
            }
        }

        gizmos.circle_2d(
            Isometry2d::from_translation(transform.translation.truncate()),
            5.,
            light.color,
        );
    }
}

fn spawn_occluders(mut commands: Commands, mut timers: ResMut<Timers>, time: Res<Time>) {
    if timers.occluder_timer.tick(time.delta()).just_finished() {
        let mut rng = rng();

        let x = rng.random_range(-WIDTH / 2.0..WIDTH / 2.0);

        let occluder_type = rng.random_range(0..4);
        let occluder = match occluder_type {
            0 => Occluder2d::round_rectangle(
                rng.random_range(10.0..40.0),
                rng.random_range(10.0..40.0),
                rng.random_range(10.0..40.0),
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
    let mut rng = rng();
    for (id, mut transform) in &mut occluders {
        let r = rng.random_range(0.0..1.0);

        if r <= MOVE_FREQ {
            transform.translation.y += time.delta_secs() * 500.0;

            if transform.translation.y > HEIGHT / 2.0 + 60. {
                commands.entity(id).despawn();
            }

            transform.rotate_z(3. * time.delta_secs());
        }
    }
}

// this is all copied from https://github.com/bevyengine/bevy/tree/latest/examples/diagnostics

const FRAME_TIME_DIAGNOSTICS: [DiagnosticPath; 3] = [
    FrameTimeDiagnosticsPlugin::FPS,
    FrameTimeDiagnosticsPlugin::FRAME_COUNT,
    FrameTimeDiagnosticsPlugin::FRAME_TIME,
];
const ENTITY_COUNT_DIAGNOSTICS: [DiagnosticPath; 1] = [EntityCountDiagnosticsPlugin::ENTITY_COUNT];
const SYSTEM_INFO_DIAGNOSTICS: [DiagnosticPath; 4] = [
    SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE,
    SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE,
    SystemInformationDiagnosticsPlugin::SYSTEM_CPU_USAGE,
    SystemInformationDiagnosticsPlugin::SYSTEM_MEM_USAGE,
];

fn filters_inputs(
    keys: Res<ButtonInput<KeyCode>>,
    mut status: ResMut<LogDiagnosticsStatus>,
    mut filters: ResMut<LogDiagnosticsFilters>,
    mut log_state: ResMut<LogDiagnosticsState>,
) {
    if keys.just_pressed(KeyCode::KeyQ) {
        *status = match *status {
            LogDiagnosticsStatus::Enabled => {
                log_state.disable_filtering();
                LogDiagnosticsStatus::Disabled
            }
            LogDiagnosticsStatus::Disabled => {
                log_state.enable_filtering();
                if filters.frame_time {
                    enable_filters(&mut log_state, FRAME_TIME_DIAGNOSTICS);
                }
                if filters.entity_count {
                    enable_filters(&mut log_state, ENTITY_COUNT_DIAGNOSTICS);
                }
                if filters.system_info {
                    enable_filters(&mut log_state, SYSTEM_INFO_DIAGNOSTICS);
                }
                LogDiagnosticsStatus::Enabled
            }
        };
    }

    let enabled = *status == LogDiagnosticsStatus::Enabled;
    if keys.just_pressed(KeyCode::Digit1) {
        filters.frame_time = !filters.frame_time;
        if enabled {
            if filters.frame_time {
                enable_filters(&mut log_state, FRAME_TIME_DIAGNOSTICS);
            } else {
                disable_filters(&mut log_state, FRAME_TIME_DIAGNOSTICS);
            }
        }
    }
    if keys.just_pressed(KeyCode::Digit2) {
        filters.entity_count = !filters.entity_count;
        if enabled {
            if filters.entity_count {
                enable_filters(&mut log_state, ENTITY_COUNT_DIAGNOSTICS);
            } else {
                disable_filters(&mut log_state, ENTITY_COUNT_DIAGNOSTICS);
            }
        }
    }
    if keys.just_pressed(KeyCode::Digit3) {
        filters.system_info = !filters.system_info;
        if enabled {
            if filters.system_info {
                enable_filters(&mut log_state, SYSTEM_INFO_DIAGNOSTICS);
            } else {
                disable_filters(&mut log_state, SYSTEM_INFO_DIAGNOSTICS);
            }
        }
    }
}

fn enable_filters(
    log_state: &mut LogDiagnosticsState,
    diagnostics: impl IntoIterator<Item = DiagnosticPath>,
) {
    log_state.extend_filter(diagnostics);
}

fn disable_filters(
    log_state: &mut LogDiagnosticsState,
    diagnostics: impl IntoIterator<Item = DiagnosticPath>,
) {
    for diagnostic in diagnostics {
        log_state.remove_filter(&diagnostic);
    }
}

fn update_commands(
    mut commands: Commands,
    log_commands: Single<Entity, With<LogDiagnosticsCommands>>,
    status: Res<LogDiagnosticsStatus>,
    filters: Res<LogDiagnosticsFilters>,
) {
    let enabled = *status == LogDiagnosticsStatus::Enabled;
    let alpha = if enabled { 1. } else { 0.25 };
    let enabled_color = |enabled| {
        if enabled {
            Color::from(palettes::tailwind::GREEN_400)
        } else {
            Color::from(palettes::tailwind::RED_400)
        }
    };
    commands
        .entity(*log_commands)
        .despawn_related::<Children>()
        .insert(children![
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(5),
                    ..default()
                },
                children![
                    Text::new("[Q] Toggle filtering:"),
                    (
                        Text::new(format!("{:?}", *status)),
                        TextColor(enabled_color(enabled))
                    )
                ]
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(5),
                    ..default()
                },
                children![
                    (
                        Text::new("[1] Frame times:"),
                        TextColor(Color::WHITE.with_alpha(alpha))
                    ),
                    (
                        Text::new(format!("{:?}", filters.frame_time)),
                        TextColor(enabled_color(filters.frame_time).with_alpha(alpha))
                    )
                ]
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(5),
                    ..default()
                },
                children![
                    (
                        Text::new("[2] Entity count:"),
                        TextColor(Color::WHITE.with_alpha(alpha))
                    ),
                    (
                        Text::new(format!("{:?}", filters.entity_count)),
                        TextColor(enabled_color(filters.entity_count).with_alpha(alpha))
                    )
                ]
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(5),
                    ..default()
                },
                children![
                    (
                        Text::new("[3] System info:"),
                        TextColor(Color::WHITE.with_alpha(alpha))
                    ),
                    (
                        Text::new(format!("{:?}", filters.system_info)),
                        TextColor(enabled_color(filters.system_info).with_alpha(alpha))
                    )
                ]
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(5),
                    ..default()
                },
                children![
                    (
                        Text::new("[4] Render diagnostics:"),
                        TextColor(Color::WHITE.with_alpha(alpha))
                    ),
                    (
                        Text::new("Private"),
                        TextColor(enabled_color(false).with_alpha(alpha))
                    )
                ]
            ),
        ]);
}

#[derive(Debug, Default, PartialEq, Eq, Resource)]
enum LogDiagnosticsStatus {
    /// No filtering, showing all logs
    #[default]
    Disabled,
    /// Filtering enabled, showing only subset of logs
    Enabled,
}

#[derive(Default, Resource)]
struct LogDiagnosticsFilters {
    frame_time: bool,
    entity_count: bool,
    system_info: bool,
    #[expect(
        dead_code,
        reason = "Currently the diagnostic paths referent to RenderDiagnosticPlugin are private"
    )]
    render_diagnostics: bool,
}

#[derive(Component)]
/// Marks the UI node that has instructions on how to change the filtering
struct LogDiagnosticsCommands;
