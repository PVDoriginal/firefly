use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_component::ExtractComponent,
        render_resource::{
            ShaderType, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
            UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::TextureCache,
        view::ViewTarget,
    },
};

use crate::{LightMapTexture, extract::ExtractedPointLight};

#[derive(Default, Clone, Copy, ShaderType)]
pub(crate) struct LightingData {
    pub n_lights: u32,
}

#[derive(Resource, Default)]
pub(crate) struct LightingDataBuffer(pub UniformBuffer<LightingData>);

pub(crate) struct PreparePlugin;

impl Plugin for PreparePlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<LightingDataBuffer>();
        render_app.add_systems(Render, prepare_data.in_set(RenderSet::Prepare));
        render_app.add_systems(Render, prepare_lightmap.in_set(RenderSet::Prepare));
    }
}

pub fn prepare_lightmap(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    view_targets: Query<(Entity, &ViewTarget)>,
) {
    for (entity, view_target) in &view_targets {
        let light_map_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("lightmap"),
                size: view_target.main_texture().size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        commands
            .entity(entity)
            .insert(LightMapTexture(light_map_texture));
    }
}

fn prepare_data(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    lights: Query<&ExtractedPointLight>,
    mut buffer: ResMut<LightingDataBuffer>,
) {
    let data = LightingData {
        n_lights: lights.iter().len() as u32,
    };

    buffer.0.set(data);
    buffer.0.write_buffer(&render_device, &render_queue);
}
