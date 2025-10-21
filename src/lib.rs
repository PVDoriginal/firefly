use bevy::{
    color::palettes::css::{BLUE, PINK},
    prelude::*,
};

#[derive(Component, Reflect)]
pub struct Occluder {
    pub shape: OccluderShape,
}

#[derive(Reflect)]
pub enum OccluderShape {
    Rectangle { width: f32, height: f32 },
}

#[derive(Component, Reflect)]
pub struct Light {
    pub shape: LightShape,
}

#[derive(Reflect)]
pub enum LightShape {
    Point,
}

pub struct FireflyPlugin;

impl Plugin for FireflyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_gizmos);
    }
}

fn draw_gizmos(
    mut gizmos: Gizmos,
    lights: Query<&Transform, With<Light>>,
    occluders: Query<(&Transform, &Occluder)>,
) {
    for transform in &lights {
        let isometry = Isometry2d::from_translation(transform.translation.truncate());
        gizmos.circle_2d(isometry, 10., BLUE);
    }

    for (transform, occluder) in &occluders {
        let isometry = Isometry2d::from_translation(transform.translation.truncate());

        match occluder.shape {
            OccluderShape::Rectangle { width, height } => {
                gizmos.rect_2d(isometry, vec2(width, height), PINK);
            }
        }
    }
}
