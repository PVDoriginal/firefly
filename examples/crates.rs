use bevy::{
    color::palettes::css::RED, image::ImageLoaderSettings, prelude::*, sprite::Anchor,
    window::PrimaryWindow,
};
use bevy_firefly::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()));
    app.add_plugins((FireflyPlugin /*FireflyGizmosPlugin*/,));

    app.init_resource::<Dragged>();

    app.add_systems(Startup, setup);
    app.add_systems(Update, (z_sorting, drag_objects));

    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = 0.15;

    commands.spawn((
        Camera2d,
        Projection::Orthographic(proj),
        FireflyConfig::default(),
    ));

    let mut sprite = Sprite::from_image(asset_server.load("crate.png"));
    sprite.anchor = Anchor::Custom(vec2(0.0, -0.5 + 3.0 / 18.0));

    commands.spawn((
        sprite,
        NormalMap::from_file("crate_normal.png", &asset_server),
        Transform::from_translation(vec3(0., -20., 20.)),
        Occluder2d::rectangle(12., 5.),
    ));

    let mut sprite = Sprite::from_image(asset_server.load("crate.png"));
    sprite.anchor = Anchor::Custom(vec2(0.0, -0.5 + 3.0 / 18.0));

    commands.spawn((
        sprite,
        NormalMap::from_file("crate_normal.png", &asset_server),
        Transform::from_translation(vec3(-20., 20., 0.)),
        Occluder2d::rectangle(12., 5.),
    ));

    let mut sprite = Sprite::from_image(asset_server.load("vase.png"));
    sprite.anchor = Anchor::Custom(vec2(0.0, -0.5 + 5.0 / 19.0));

    commands.spawn((
        sprite,
        NormalMap::from_file("vase_normal.png", &asset_server),
        Transform::from_translation(vec3(0., 20., 0.)),
        Occluder2d::round_rectangle(5.5, 0.5, 3.),
    ));

    let mut sprite = Sprite::from_image(asset_server.load("vase.png"));
    sprite.anchor = Anchor::Custom(vec2(0.0, -0.5 + 5.0 / 19.0));

    commands.spawn((
        sprite,
        NormalMap::from_file("vase_normal.png", &asset_server),
        Transform::from_translation(vec3(10., -20., 0.)),
        Occluder2d::round_rectangle(5.5, 0.5, 3.),
    ));

    commands.spawn((
        Sprite::from_image(asset_server.load("bonfire.png")),
        PointLight2d {
            range: 100.,
            height: 3.,
            color: Color::srgb(1.0, 0.8, 0.6),
            ..default()
        },
    ));

    let mut sprite = Sprite::from_image(asset_server.load("lamp.png"));
    sprite.anchor = Anchor::Custom(vec2(0.0, -0.5 + 5.0 / 32.0));

    commands
        .spawn((sprite, Transform::from_translation(vec3(20., 0., 0.))))
        .with_children(|r| {
            r.spawn((
                PointLight2d {
                    range: 100.,
                    height: 22.,
                    color: Color::srgb(0.8, 0.8, 1.0),
                    ..default()
                },
                Transform::from_translation(vec3(0., 22., 0.)),
            ));
        });
}

fn z_sorting(mut sprites: Query<&mut Transform, With<Sprite>>) {
    for mut transform in &mut sprites {
        transform.translation.z = -transform.translation.y;
    }
}

#[derive(Resource, Default)]
struct Dragged(pub Option<Entity>);

fn drag_objects(
    mut objects: Query<(Entity, &mut Transform), With<Sprite>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
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
            3.,
            RED,
        );
        return;
    }

    if let Some((hovered, transform)) = objects.iter().min_by(|(_, a), (_, b)| {
        a.translation
            .xy()
            .distance(cursor_position)
            .total_cmp(&b.translation.xy().distance(cursor_position))
    }) && transform.translation.xy().distance(cursor_position) < 4.
    {
        gizmos.circle_2d(
            Isometry2d::from_translation(transform.translation.xy()),
            3.,
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
