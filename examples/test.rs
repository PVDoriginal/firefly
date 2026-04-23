use bevy::{
    camera::{CameraOutputMode, visibility::RenderLayers},
    color::palettes::css::{GREEN, PURPLE},
    prelude::*,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, setup);
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            output_mode: CameraOutputMode::Skip,
            ..default()
        },
        RenderLayers::layer(1),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(materials.add(Color::from(PURPLE))),
        Transform::from_translation(vec3(-100.0, 0.0, 0.0)).with_scale(Vec3::splat(128.)),
        RenderLayers::layer(0),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(materials.add(Color::from(GREEN))),
        Transform::from_translation(vec3(100.0, 0.0, 0.0)).with_scale(Vec3::splat(128.)),
        RenderLayers::layer(1),
    ));
}
