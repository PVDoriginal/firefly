//! Module with logic for determining whether entities (lights, occluders) are visibile or not.
//! Visibility is based on whether they can affect what is rendered on-screen or not,
//! for instance occluders can be off-screen and still visible because they can block light
//! that would be otherwise visible on-screen.

use std::any::TypeId;

use bevy::{
    camera::visibility::{SetViewVisibility, VisibilitySystems, VisibleEntities, check_visibility},
    math::bounding::{Aabb2d, BoundingVolume, IntersectsVolume},
    prelude::*,
};

use crate::{
    data::FireflyConfig,
    lights::{LightHeight, PointLight2d},
    occluders::Occluder2dShape,
    prelude::Occluder2d,
};

/// Timer that starts ticking down when an entity no longer affects
/// what the player sees. When it finished, the [`NotVisible`] component
/// is added to the corresponding Render World entity.
#[derive(Component)]
pub struct VisibilityTimer(pub Timer);

/// Component added to Render World entities when they are no longer visible
/// in the Main World. Visibility is based on [`VisibilityTimer`].
#[derive(Component, Default)]
pub struct NotVisible;

impl Default for VisibilityTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(2.0, TimerMode::Once))
    }
}

#[derive(Component)]
pub struct OccluderAabb(pub Aabb2d);

impl Default for OccluderAabb {
    fn default() -> Self {
        Self(Aabb2d::new(default(), default()))
    }
}

/// Handles entity visibility. Added automatically through [`FireflyPlugin`](crate::prelude::FireflyPlugin).
pub struct VisibilityPlugin;

impl Plugin for VisibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightRect>();

        app.add_systems(Update, occluder_aabb);

        app.add_systems(
            PostUpdate,
            (mark_visible_lights, mark_visible_occluders)
                .chain()
                .in_set(VisibilitySystems::CheckVisibility)
                .after(check_visibility),
        );
    }
}

#[derive(Resource, Default)]
struct LightRect(pub Rect);

fn mark_visible_lights(
    mut lights: Query<(
        Entity,
        &GlobalTransform,
        &PointLight2d,
        &LightHeight,
        &mut ViewVisibility,
        &mut VisibilityTimer,
    )>,
    mut camera: Single<(&GlobalTransform, &mut VisibleEntities, &Projection), With<FireflyConfig>>,
    mut light_rect: ResMut<LightRect>,
    time: Res<Time>,
) {
    let Projection::Orthographic(projection) = camera.2 else {
        return;
    };

    let camera_aabb = Aabb2d {
        min: projection.area.min + camera.0.translation().truncate(),
        max: projection.area.max + camera.0.translation().truncate(),
    };

    let camera_rect = Rect {
        min: projection.area.min + camera.0.translation().truncate(),
        max: projection.area.max + camera.0.translation().truncate(),
    };

    light_rect.0 = Rect::EMPTY;

    for (entity, transform, light, height, mut visibility, mut visibility_timer) in &mut lights {
        let pos = transform.translation().truncate() - vec2(0.0, height.0) + light.offset.xy();

        let light_aabb = Aabb2d {
            min: pos - light.range,
            max: pos + light.range,
        };

        if light_aabb.intersects(&camera_aabb) {
            if !visibility.get() {
                visibility.set_visible();

                let visible_lights = camera.1.get_mut(TypeId::of::<PointLight2d>());
                visible_lights.push(entity);

                *visibility_timer = default();
            }

            light_rect.0 = light_rect
                .0
                .union(camera_rect.union_point(pos).intersect(Rect {
                    min: pos - light.range,
                    max: pos + light.range,
                }));
        }

        visibility_timer.0.tick(time.delta());
    }
}

fn mark_visible_occluders(
    mut camera: Single<&mut VisibleEntities, With<FireflyConfig>>,
    mut occluders: Query<(
        Entity,
        &OccluderAabb,
        &mut ViewVisibility,
        &mut VisibilityTimer,
    )>,
    light_rect: Res<LightRect>,
    time: Res<Time>,
) {
    let light_rect_aabb = Aabb2d {
        min: light_rect.0.min,
        max: light_rect.0.max,
    };

    for (entity, aabb, mut visibility, mut visibility_timer) in &mut occluders {
        if aabb.0.intersects(&light_rect_aabb) {
            if !visibility.get() {
                visibility.set_visible();

                let visible_occluders = camera.get_mut(TypeId::of::<Occluder2d>());
                visible_occluders.push(entity);

                *visibility_timer = default();
            }
        }

        visibility_timer.0.tick(time.delta());
    }
}

fn occluder_aabb(
    mut occluders: Query<
        (&Occluder2d, &GlobalTransform, &mut OccluderAabb),
        Or<(Changed<GlobalTransform>, Changed<Occluder2d>)>,
    >,
) {
    for (occluder, transform, mut rect) in &mut occluders {
        let isometry = Isometry2d {
            rotation: Rot2::radians(transform.rotation().to_euler(EulerRot::XYZ).2),
            translation: transform.translation().truncate() + occluder.offset.truncate(),
        };

        rect.0 = match occluder.shape() {
            Occluder2dShape::RoundRectangle {
                width,
                height,
                radius,
            } => Aabb2d {
                min: vec2(-width / 2.0, -height / 2.0) - radius,
                max: vec2(width / 2.0, height / 2.0) + radius,
            }
            .transformed_by(isometry.translation, isometry.rotation),

            Occluder2dShape::Polygon { vertices } => Aabb2d::from_point_cloud(isometry, vertices),
            Occluder2dShape::Polyline { vertices } => Aabb2d::from_point_cloud(isometry, vertices),
        }
    }
}
