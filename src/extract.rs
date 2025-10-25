use bevy::{
    prelude::*,
    render::{
        Extract, RenderApp, extract_component::ExtractComponent,
        gpu_component_array_buffer::GpuComponentArrayBufferPlugin, render_resource::ShaderType,
        sync_world::RenderEntity,
    },
};

use crate::Occluder;

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub(crate) struct ExtractedPointLight {
    pub pos: Vec2,
}

#[derive(Component, Default, Clone, ExtractComponent)]
pub(crate) struct ExtractedOccluder {
    pub vertices: Vec<Vec2>,
    pub concave: bool,
    pub closed: bool,
}
pub(crate) struct ExtractPlugin;
impl Plugin for ExtractPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(ExtractSchedule, (extract_point_lights, extract_occluders));
    }
}

fn extract_point_lights(
    mut commands: Commands,
    point_lights: Extract<
        Query<(&RenderEntity, &GlobalTransform), With<crate::prelude::PointLight>>,
    >,
) {
    for (render_entity, global_transform) in &point_lights {
        commands
            .entity(render_entity.id())
            .insert(ExtractedPointLight {
                pos: global_transform.translation().truncate(),
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
                vertices: occluder
                    .shape
                    .vertices()
                    .iter()
                    .map(|x| x + global_transform.translation().truncate())
                    .collect(),
                concave: occluder.shape.is_concave(),
                closed: occluder.shape.is_closed(),
            });
    }
}
