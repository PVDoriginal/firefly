use bevy::{
    prelude::*,
    render::{
        Extract, MainWorld, RenderApp, extract_component::ExtractComponent,
        gpu_component_array_buffer::GpuComponentArrayBufferPlugin, render_resource::ShaderType,
        sync_world::RenderEntity,
    },
};

use bevy::render::view::ViewUniform;

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub(crate) struct ExtractedPointLight {
    pub pos: Vec2,
}

pub(crate) struct ExtractPlugin;
impl Plugin for ExtractPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GpuComponentArrayBufferPlugin::<ExtractedPointLight>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(ExtractSchedule, extract_point_lights);
    }
}

fn extract_point_lights(
    mut commands: Commands,
    point_lights: Extract<Query<(&RenderEntity, &GlobalTransform), With<crate::PointLight>>>,
) {
    for (render_entity, global_transform) in &point_lights {
        commands
            .entity(render_entity.id())
            .insert(ExtractedPointLight {
                pos: global_transform.translation().truncate(),
            });
    }
}
