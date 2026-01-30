use bevy::{
    color::palettes::css::{BLUE, RED},
    prelude::*,
};
use bevy_firefly::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()));
    app.add_plugins((FireflyPlugin /*FireflyGizmosPlugin*/,));

    app.add_systems(Startup, setup);

    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = 0.15;

    commands.spawn((
        Camera2d,
        Projection::Orthographic(proj),
        FireflyConfig {
            // normal maps need to be explicitly enabled
            normal_mode: NormalMode::TopDown,
            ..default()
        },
    ));

    commands.spawn((
        PointLight2d {
            color: Color::Srgba(RED),
            ..default()
        },
        Transform::from_translation(vec3(-50.0, 0.0, 0.0)),
    ));

    commands.spawn((
        PointLight2d {
            color: Color::Srgba(BLUE),
            ..default()
        },
        Transform::from_translation(vec3(50.0, 0.0, 0.0)),
    ));
}
