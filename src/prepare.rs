use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        render_resource::{
            GpuArrayBuffer, ShaderType, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::TextureCache,
        view::ViewTarget,
    },
};

use crate::{
    EmptyLightMapTexture, IntermediaryLightMapTexture, LightMapTexture, Occluder,
    extract::{ExtractedOccluder, ExtractedPointLight},
};

#[derive(Default, Clone, Copy, ShaderType)]
pub(crate) struct LightingData {
    pub n_occluders: u32,
}

#[derive(Resource)]
pub(crate) struct OccluderSet {
    pub meta: GpuArrayBuffer<OccluderMeta>,
    pub vertices: Vec<GpuArrayBuffer<Vertex>>,
}

impl FromWorld for OccluderSet {
    fn from_world(world: &mut World) -> Self {
        Self {
            meta: GpuArrayBuffer::<OccluderMeta>::new(world.resource::<RenderDevice>()),
            vertices: default(),
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct LightingDataBuffer(pub UniformBuffer<LightingData>);

#[derive(ShaderType, Clone)]
pub(crate) struct OccluderMeta {
    pub n_vertices: u32,
}

#[derive(ShaderType, Clone)]
pub(crate) struct Vertex {
    pub angle: f32,
    pub pos: Vec2,
}

#[derive(Resource, Default)]
pub(crate) struct Lights(pub Vec<UniformBuffer<ExtractedPointLight>>);

pub(crate) struct PreparePlugin;

impl Plugin for PreparePlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<LightingDataBuffer>();
        render_app.init_resource::<Lights>();

        render_app.add_systems(Render, prepare_data.in_set(RenderSet::Prepare));
        render_app.add_systems(Render, prepare_lightmap.in_set(RenderSet::Prepare));
    }
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<OccluderSet>();
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

        let inter_light_map_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("intermediary lightmap"),
                size: view_target.main_texture().size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let empty_light_map_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("empty lightmap"),
                size: view_target.main_texture().size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        commands.entity(entity).insert((
            LightMapTexture(light_map_texture),
            IntermediaryLightMapTexture(inter_light_map_texture),
            EmptyLightMapTexture(empty_light_map_texture),
        ));
    }
}

fn prepare_data(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    lights: Query<&ExtractedPointLight>,
    occluders: Query<&ExtractedOccluder>,
    mut buffer: ResMut<LightingDataBuffer>,
    mut lights_res: ResMut<Lights>,
    mut occluder_set: ResMut<OccluderSet>,
) {
    let data = LightingData {
        n_occluders: occluders.iter().len() as u32,
    };

    *lights_res = default();
    for light in lights {
        let mut buffer = UniformBuffer::<ExtractedPointLight>::default();
        buffer.set(*light);
        buffer.write_buffer(&render_device, &render_queue);

        lights_res.0.push(buffer);
    }

    buffer.0.set(data);
    buffer.0.write_buffer(&render_device, &render_queue);

    *occluder_set = OccluderSet {
        meta: GpuArrayBuffer::<OccluderMeta>::new(&render_device),
        vertices: default(),
    };

    for occluder in occluders {
        occluder_set.meta.push(OccluderMeta {
            n_vertices: occluder.vertices.len() as u32,
        });

        for light in lights {
            let mut vertices_buffer = GpuArrayBuffer::<Vertex>::new(&render_device);

            let mut vertices: Vec<_> = occluder
                .vertices
                .iter()
                .map(|&pos| Vertex {
                    angle: light.pos.angle_to(pos),
                    pos,
                })
                .collect();

            vertices.sort_by(|a, b| a.angle.total_cmp(&b.angle));

            vertices.iter().for_each(|v| {
                vertices_buffer.push(v.clone());
            });

            vertices_buffer.write_buffer(&render_device, &render_queue);

            occluder_set.vertices.push(vertices_buffer);
        }
    }
    occluder_set
        .meta
        .write_buffer(&render_device, &render_queue);
}
