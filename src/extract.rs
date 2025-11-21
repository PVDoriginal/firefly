use bevy::{
    prelude::*,
    render::{
        Extract, RenderApp, extract_component::ExtractComponentPlugin, sync_world::RenderEntity,
    },
};

use crate::{
    data::{ExtractedWorldData, FireflyConfig},
    lights::{ExtractedPointLight, PointLight2d},
    occluders::ExtractedOccluder,
    prelude::Occluder2d,
};

pub(crate) struct ExtractPlugin;
impl Plugin for ExtractPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<FireflyConfig>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(ExtractSchedule, extract_lights);
        render_app.add_systems(ExtractSchedule, extract_occluders);
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

fn extract_lights(
    mut commands: Commands,
    lights: Extract<Query<(&RenderEntity, &GlobalTransform, &PointLight2d)>>,
) {
    for (entity, transform, light) in &lights {
        commands.entity(entity.id()).insert(ExtractedPointLight {
            pos: transform.translation().truncate(),
            color: light.color,
            intensity: light.intensity,
            range: light.range,
            z: transform.translation().z,
        });
    }
}

fn extract_occluders(
    mut commands: Commands,
    occluders: Extract<Query<(&RenderEntity, &Occluder2d, &GlobalTransform)>>,
) {
    for (render_entity, occluder, global_transform) in &occluders {
        let mut rect = occluder.rect();
        rect.min += global_transform.translation().truncate();
        rect.max += global_transform.translation().truncate();

        commands
            .entity(render_entity.id())
            .insert(ExtractedOccluder {
                pos: global_transform.translation().truncate(),
                rot: global_transform.rotation().to_axis_angle().1,
                shape: occluder.shape().clone(),
                rect,
                z: global_transform.translation().z,
                color: occluder.color,
                opacity: occluder.opacity,
                ignored_sprites: occluder.ignored_sprites.clone(),
            });
    }
}
