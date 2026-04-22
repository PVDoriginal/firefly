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
    app.add_plugins(FireflyPlugin);

    app.add_systems(Startup, setup);
    app.add_systems(Update, toggle_layers);

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
        RenderLayers::layer(1).union(&RenderLayers::layer(0)),
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

#[derive(Default)]
enum LayerState {
    First,
    Second,
    #[default]
    Both,
}

impl LayerState {
    fn toggle(&self) -> LayerState {
        match *self {
            LayerState::First => LayerState::Second,
            LayerState::Second => LayerState::Both,
            LayerState::Both => LayerState::First,
        }
    }
}

fn toggle_layers(
    mut state: Local<LayerState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut camera: Single<&mut RenderLayers, With<Camera2d>>,
) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    *state = state.toggle();

    **camera = match *state {
        LayerState::First => RenderLayers::layer(0),
        LayerState::Second => RenderLayers::layer(1),
        LayerState::Both => RenderLayers::layer(0).union(&RenderLayers::layer(1)),
    }
}
