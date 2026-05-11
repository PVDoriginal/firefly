//! This example demonstrates using multiple lightmaps and render layers.

use bevy::{
    camera::{CameraOutputMode, visibility::RenderLayers},
    color::palettes::css::{BLUE, RED},
    prelude::*,
    render::view::Hdr,
    window::PrimaryWindow,
};
use bevy_firefly::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins((DefaultPlugins, FireflyPlugin, FireflyGizmosPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (drag_objects, move_camera));

    app.insert_resource(FireflyGizmoStyle {
        // Making the point light gizmos invisible for aesthetic reasons.
        light_inner_color: Color::srgba(0.0, 0.0, 0.0, 0.0),
        light_outer_color: Color::srgba(0.0, 0.0, 0.0, 0.0),
        ..default()
    })
    .init_resource::<Dragged>();

    app.run();
}

#[derive(Component, Clone)]
struct MainCamera;

fn setup(mut commands: Commands) {
    // Spawning the main camera that will render the lights on layer 0.
    // Setting the `msaa_writeback` to off is important so the output from the other camera
    // doesn't 'leak' into this one.
    let main_camera = commands
        .spawn((
            MainCamera,
            Camera2d,
            Camera {
                msaa_writeback: MsaaWriteback::Off,
                ..default()
            },
            Hdr,
            FireflyConfig {
                ambient_brightness: 0.15,
                // You can set the 'combination mode', specifying
                // how the lightmaps that target this one will be combined over it.
                // This is optional and can be omitted in this case, since the default will be multiply.
                combination_mode: CombinationMode::Multiply,
                ..default()
            },
            RenderLayers::layer(0),
        ))
        .id();

    // Spawning a secondary 'visibility' camera that will render lights on layer 1.
    // It's important that this camera's order is lower than the main one, and that the
    // output mode is set to skip writing to the render target (window).
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            output_mode: CameraOutputMode::Skip,
            ..default()
        },
        FireflyConfig::default(),
        RenderLayers::layer(1),
        // Relationship which tells Firefly to combine the lightmap from this camera
        // to the main camera's lightmap.
        CombineLightmapTo(main_camera),
    ));

    // Blue light visible on the main camera.
    commands.spawn((
        PointLight2d {
            color: Color::Srgba(BLUE),
            intensity: 10.0,
            radius: 150.0,
            ..default()
        },
        Transform::from_translation(vec3(-30.0, 0.0, 0.0)),
        RenderLayers::layer(0),
    ));

    // Red light visible on the main camera.
    commands.spawn((
        PointLight2d {
            color: Color::Srgba(RED),
            intensity: 10.0,
            radius: 150.0,
            ..default()
        },
        Transform::from_translation(vec3(30.0, 0.0, 0.0)),
        RenderLayers::layer(0),
    ));

    // White lights visible on the second camera.
    // They're set to have no falloff for aesthetic reasons.
    commands.spawn((
        PointLight2d {
            intensity: 1.0,
            radius: 200.0,
            falloff: Falloff::None,
            core: LightCore::from_radius_boost(0.0, 0.0),
            ..default()
        },
        Transform::from_translation(vec3(30.0, 30.0, 0.0)),
        RenderLayers::layer(1),
    ));

    commands.spawn((
        PointLight2d {
            intensity: 1.0,
            radius: 200.0,
            falloff: Falloff::None,
            core: LightCore::from_radius_boost(0.0, 0.0),
            ..default()
        },
        Transform::from_translation(vec3(-30.0, -30.0, 0.0)),
        RenderLayers::layer(1),
    ));

    // Occluder which will affect all lights.
    commands.spawn((
        Occluder2d::polyline([vec2(0.0, -100.0), vec2(0.0, 100.0)]).unwrap(),
        RenderLayers::layer(0).with(1),
    ));

    // Occluders only affecting lights on layer 0 (the main lights).
    commands.spawn((
        Occluder2d::rectangle(10.0, 10.0),
        RenderLayers::layer(0),
        Transform::from_translation(vec3(50.0, -10.0, 0.0)),
    ));

    commands.spawn((
        Occluder2d::rectangle(10.0, 10.0),
        RenderLayers::layer(0),
        Transform::from_translation(vec3(-50.0, 20.0, 0.0)),
    ));
}

#[derive(Resource, Default)]
struct Dragged(pub Option<Entity>);

fn drag_objects(
    mut objects: Query<(Entity, &mut Transform), With<PointLight2d>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut dragged: ResMut<Dragged>,
    mut gizmos: Gizmos,
) {
    let Some(cursor_position) = window
        .cursor_position()
        .and_then(|cursor| camera.0.viewport_to_world_2d(&camera.1, cursor).ok())
    else {
        dragged.0 = None;
        return;
    };

    if buttons.pressed(MouseButton::Left)
        && let Some(dragged) = dragged.0
        && let Ok((_, mut transform)) = objects.get_mut(dragged)
    {
        transform.translation.x = cursor_position.x;
        transform.translation.y = cursor_position.y;
        gizmos.circle_2d(
            Isometry2d::from_translation(transform.translation.xy()),
            10.,
            RED,
        );
        return;
    }

    if let Some((hovered, transform)) = objects.iter().min_by(|(_, a), (_, b)| {
        a.translation
            .xy()
            .distance(cursor_position)
            .total_cmp(&b.translation.xy().distance(cursor_position))
    }) && transform.translation.xy().distance(cursor_position) < 30.
    {
        gizmos.circle_2d(
            Isometry2d::from_translation(transform.translation.xy()),
            10.,
            RED,
        );
        if buttons.just_pressed(MouseButton::Left) {
            dragged.0 = Some(hovered);
        }
    }

    if !buttons.pressed(MouseButton::Left) {
        dragged.0 = None;
    }
}

const CAMERA_SPEED: f32 = 60.0;
fn move_camera(
    mut cameras: Query<&mut Transform, With<Camera>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for mut camera in &mut cameras {
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
}
