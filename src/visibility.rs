//! Module with logic for determining whether entities (lights, occluders) are visibile or not.
//! Visibility is based on whether they can affect what is rendered on-screen or not,
//! for instance occluders can be off-screen and still visible because they can block light
//! that would be otherwise visible on-screen.

use std::any::TypeId;

use bevy::{
    camera::visibility::{
        PreviousVisibleEntities, VisibilitySystems, VisibleEntities, check_visibility,
    },
    prelude::*,
};

use crate::{
    data::FireflyConfig,
    lights::{LightHeight, PointLight2d},
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

/// Handles entity visibility. Added automatically through [`FireflyPlugin`](crate::prelude::FireflyPlugin).
pub struct VisibilityPlugin;

impl Plugin for VisibilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightRect>();

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
    )>,
    mut camera: Single<(&GlobalTransform, &mut VisibleEntities, &Projection), With<FireflyConfig>>,
    mut previous_visible_entities: ResMut<PreviousVisibleEntities>,
    mut light_rect: ResMut<LightRect>,
) {
    let Projection::Orthographic(projection) = camera.2 else {
        return;
    };

    let camera_rect = Rect {
        min: projection.area.min + camera.0.translation().truncate(),
        max: projection.area.max + camera.0.translation().truncate(),
    };

    light_rect.0 = Rect::EMPTY;
    for (entity, transform, light, height, mut visibility) in &mut lights {
        let pos = transform.translation().truncate() - vec2(0.0, height.0) + light.offset.xy();

        if !(Rect {
            min: pos - light.range,
            max: pos + light.range,
        })
        .intersect(camera_rect)
        .is_empty()
        {
            if !**visibility {
                visibility.set();

                let visible_lights = camera.1.get_mut(TypeId::of::<PointLight2d>());
                visible_lights.push(entity);

                previous_visible_entities.remove(&entity);
            }

            light_rect.0 = light_rect
                .0
                .union(camera_rect.union_point(pos).intersect(Rect {
                    min: pos - light.range,
                    max: pos + light.range,
                }));
        }
    }
}

fn mark_visible_occluders(
    mut camera: Single<&mut VisibleEntities, With<FireflyConfig>>,
    mut occluders: Query<(
        Entity,
        &Occluder2d,
        &GlobalTransform,
        &mut ViewVisibility,
        &mut VisibilityTimer,
    )>,
    mut previous_visible_entities: ResMut<PreviousVisibleEntities>,
    light_rect: Res<LightRect>,
    time: Res<Time>,
) {
    for (entity, occluder, global_transform, mut visibility, mut visibility_timer) in &mut occluders
    {
        let mut rect = occluder.rect();
        rect.min += global_transform.translation().truncate() + occluder.offset.xy();
        rect.max += global_transform.translation().truncate() + occluder.offset.xy();

        if !rect.intersect(light_rect.0).is_empty() {
            if !**visibility {
                visibility.set();

                let visible_occluders = camera.get_mut(TypeId::of::<Occluder2d>());
                visible_occluders.push(entity);

                previous_visible_entities.remove(&entity);

                *visibility_timer = default();
            }
        }

        visibility_timer.0.tick(time.delta());
    }
}
