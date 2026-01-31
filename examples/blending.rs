use bevy::{
    color::palettes::css::{BLUE, RED},
    prelude::*,
    render::view::Hdr,
};
use bevy_firefly::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()));
    app.add_plugins((FireflyPlugin /*FireflyGizmosPlugin*/,));

    app.add_systems(Startup, setup);

    app.run();
}

fn setup(mut commands: Commands) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = 0.15;

    commands.spawn((
        Camera2d,
        // Tonemapping::SomewhatBoringDisplayTransform,
        Hdr::default(),
        Projection::Orthographic(proj),
        FireflyConfig {
            // normal maps need to be explicitly enabled
            normal_mode: NormalMode::TopDown,
            ..default()
        },
    ));

    commands.spawn((
        PointLight2d {
            intensity: 4.0,
            color: Color::Srgba(RED),
            ..default()
        },
        Transform::from_translation(vec3(-30.0, 0.0, 0.0)),
    ));

    commands.spawn((
        PointLight2d {
            intensity: 4.0,
            color: Color::Srgba(BLUE),
            ..default()
        },
        Transform::from_translation(vec3(30.0, 0.0, 0.0)),
    ));
}
