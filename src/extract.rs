use bevy::{
    prelude::*,
    render::{
        Extract, RenderApp, extract_component::ExtractComponentPlugin, sync_world::RenderEntity,
    },
};

use crate::{
    data::FireflyConfig,
    lights::{ExtractedPointLight, PointLight},
    occluders::ExtractedOccluder,
    prelude::Occluder,
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
    }
}

fn extract_lights(
    mut commands: Commands,
    lights: Extract<Query<(&RenderEntity, &GlobalTransform, &PointLight)>>,
) {
    for (entity, transform, _light) in &lights {
        commands.entity(entity.id()).insert(ExtractedPointLight {
            pos: transform.translation().truncate(),
        });
    }
}

fn extract_occluders(
    mut commands: Commands,
    occluders: Extract<Query<(&RenderEntity, &Occluder, &GlobalTransform)>>,
) {
    for (render_entity, occluder, global_transform) in &occluders {
        commands
            .entity(render_entity.id())
            .insert(ExtractedOccluder {
                pos: global_transform.translation().truncate(),
                shape: occluder.shape.clone(),
            });
    }
}
