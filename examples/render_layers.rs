use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    color::palettes::css::{BLUE, RED},
    ecs::entity_disabling::Disabled,
    prelude::*,
    render::view::Hdr,
    window::{PrimaryWindow, WindowRef},
};
use bevy_firefly::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    app.add_plugins(FireflyPlugin);

    app.add_systems(Startup, setup);

    app.run();
}

fn setup(mut commands: Commands) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = 0.15;

    let window2 = commands
        .spawn(Window {
            title: "Second window".into(),
            ..default()
        })
        .id();

    let window3 = commands
        .spawn(Window {
            title: "Third window".into(),
            ..default()
        })
        .id();

    commands.spawn((
        Camera2d,
        Hdr::default(),
        Projection::Orthographic(proj.clone()),
        FireflyConfig::default(),
        RenderTarget::Window(WindowRef::Primary),
        RenderLayers::layer(0),
    ));

    commands.spawn((
        Camera2d,
        Hdr::default(),
        Projection::Orthographic(proj.clone()),
        FireflyConfig::default(),
        RenderTarget::Window(WindowRef::Entity(window2)),
        RenderLayers::layer(1),
    ));

    commands.spawn((
        Camera2d,
        Hdr::default(),
        Projection::Orthographic(proj.clone()),
        FireflyConfig::default(),
        RenderTarget::Window(WindowRef::Entity(window3)),
        RenderLayers::layer(0).union(&RenderLayers::layer(1)),
    ));

    commands.spawn((
        PointLight2d {
            radius: 100.0,
            intensity: 4.0,
            color: Color::Srgba(RED),
            core: LightCore::from_radius_boost(5.0, 5.0),
            ..default()
        },
        Transform::from_translation(vec3(-15.0, 0.0, 0.0)),
        RenderLayers::layer(0),
    ));

    commands.spawn((
        PointLight2d {
            radius: 100.0,
            intensity: 4.0,
            color: Color::Srgba(BLUE),
            core: LightCore::from_radius_boost(10.0, 4.0),
            ..default()
        },
        Transform::from_translation(vec3(15.0, 0.0, 0.0)),
        RenderLayers::layer(1),
    ));

    commands.spawn((
        Occluder2d::rectangle(30.0, 30.0),
        Transform::from_translation(vec3(-30.0, -20.0, 0.0)),
        RenderLayers::layer(0),
    ));

    commands.spawn((
        Occluder2d::rectangle(30.0, 30.0),
        Transform::from_translation(vec3(30.0, -20.0, 0.0)),
        RenderLayers::layer(1),
    ));
}
