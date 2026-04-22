use bevy::{
    camera::visibility::RenderLayers,
    color::palettes::css::{BLUE, RED},
    prelude::*,
    render::view::Hdr,
};
use bevy_firefly::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()));
    app.add_plugins((FireflyPlugin, FireflyGizmosPlugin));

    app.add_systems(Startup, setup);

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
            ambient_brightness: 0.1,
            ..default()
        },
    ));

    commands.spawn((
        PointLight2d {
            radius: 50.0,
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
            radius: 50.0,
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
