use bevy::{
    prelude::*,
    render::{
        Extract, RenderApp, extract_component::ExtractComponentPlugin, sync_world::RenderEntity,
    },
};

use crate::{
    data::{ExtractedWorldData, FireflyConfig},
    lights::{ExtractedPointLight, Falloff, PointLight2d},
    occluders::{ExtractedOccluder, Occluder2dShape},
    prelude::Occluder2d,
};

pub(crate) struct ExtractPlugin;
impl Plugin for ExtractPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<FireflyConfig>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(ExtractSchedule, extract_lights_occluders);
        render_app.add_systems(ExtractSchedule, extract_world_data);
    }
}

fn extract_world_data(
    mut commands: Commands,
    camera: Extract<Query<(&RenderEntity, &GlobalTransform, &FireflyConfig, &Camera2d)>>,
) {
    for (entity, transform, _, _) in &camera {
        commands.entity(entity.id()).insert(ExtractedWorldData {
            camera_pos: transform.translation().truncate(),
        });
    }
}

fn extract_lights_occluders(
    mut commands: Commands,
    camera: Extract<Single<(&GlobalTransform, &Projection), With<FireflyConfig>>>,
    lights: Extract<Query<(&RenderEntity, &GlobalTransform, &PointLight2d)>>,
    occluders: Extract<Query<(&RenderEntity, &Occluder2d, &GlobalTransform)>>,
) {
    let Projection::Orthographic(projection) = camera.1 else {
        return;
    };

    let camera_rect = Rect {
        min: projection.area.min + camera.0.translation().truncate(),
        max: projection.area.max + camera.0.translation().truncate(),
    };

    let mut light_rect = Rect::default();
    for (entity, transform, light) in &lights {
        let pos = transform.translation().truncate();

        if (Rect {
            min: pos - light.range,
            max: pos + light.range,
        })
        .intersect(camera_rect)
        .is_empty()
        {
            continue;
        }

        commands.entity(entity.id()).insert(ExtractedPointLight {
            pos,
            color: light.color,
            intensity: light.intensity,
            range: light.range,
            z: transform.translation().z,
            inner_range: light.inner_range,
            falloff: light.falloff,
            angle: light.angle,
            cast_shadows: light.cast_shadows,
            dir: (transform.rotation() * Vec3::Y).xy(),
            height: light.height,
        });

        light_rect = light_rect.union(camera_rect.union_point(pos).intersect(Rect {
            min: pos - light.range,
            max: pos + light.range,
        }));
    }

    for (render_entity, occluder, global_transform) in &occluders {
        let mut rect = occluder.rect();
        rect.min += global_transform.translation().truncate();
        rect.max += global_transform.translation().truncate();

        if rect.intersect(light_rect).is_empty() {
            continue;
        }

        commands
            .entity(render_entity.id())
            .insert(ExtractedOccluder {
                pos: global_transform.translation().truncate(),
                rot: global_transform.rotation().to_euler(EulerRot::XYZ).2,
                shape: occluder.shape().clone(),
                rect,
                z: global_transform.translation().z,
                color: occluder.color,
                opacity: occluder.opacity,
                ignored_sprites: occluder.ignored_sprites.clone(),
                height: occluder.height,
            });
    }
}
