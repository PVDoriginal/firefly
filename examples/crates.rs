use bevy::{
    color::palettes::css::RED, image::ImageLoaderSettings, prelude::*, sprite::Anchor,
    window::PrimaryWindow,
};
use bevy_firefly::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()));
    app.add_plugins((FireflyPlugin, FireflyGizmosPlugin));

    app.add_systems(Startup, setup);
    app.add_systems(Update, move_light);

    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = 0.3;

    commands.spawn((
        Camera2d,
        Projection::Orthographic(proj),
        FireflyConfig {
            ambient_color: Color::srgb(0.5, 0.2, 0.7),
            ambient_brightness: 0.4,
            ..default()
        },
    ));

    let mut sprite = Sprite::from_image(asset_server.load("crate.png"));
    sprite.anchor = Anchor::Custom(vec2(0.0, -0.5 + 3.0 / 16.0));

    commands.spawn((
        sprite,
        NormalMap::from_file("crate_normal.png", &asset_server),
        Transform::from_translation(vec3(0., -20., 20.)),
        // Occluder2d::rectangle(10., 5.),
    ));

    let mut sprite = Sprite::from_image(asset_server.load("vase.png"));
    sprite.anchor = Anchor::Custom(vec2(0.0, -0.5 + 3.0 / 16.0));

    commands.spawn((
        sprite,
        NormalMap::from_file("vase_normal.png", &asset_server),
        Transform::from_translation(vec3(0., 20., -20.)),
        // Occluder2d::round_rectangle(7., 1., 3.),
    ));

    commands.spawn((PointLight2d::default(), Transform::default()));
}

fn move_light(
    mut light: Single<&mut Transform, With<PointLight2d>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut gizmos: Gizmos,
) {
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

    light.translation = cursor_position.extend(0.);
}
