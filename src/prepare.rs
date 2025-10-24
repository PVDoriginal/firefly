use std::cmp::Ordering;

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

#[derive(Resource, Default)]
pub(crate) struct OccluderSet(pub Vec<(GpuArrayBuffer<OccluderMeta>, GpuArrayBuffer<Vertex>)>);

#[derive(Resource, Default)]
pub(crate) struct LightingDataBuffer(pub UniformBuffer<LightingData>);

#[repr(C, align(16))]
#[derive(ShaderType, Clone, Default)]
pub(crate) struct OccluderMeta {
    pub n_vertices: u32,
    pub offset_angle: f32,
}

#[repr(C, align(16))]
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

    *occluder_set = default();

    for light in lights {
        let mut meta_buffer = GpuArrayBuffer::<OccluderMeta>::new(&render_device);
        let mut vertices_buffer = GpuArrayBuffer::<Vertex>::new(&render_device);

        for occluder in occluders {
            let mut meta: OccluderMeta = default();

            meta.offset_angle =
                (occluder.vertices[0].y - light.pos.y).atan2(occluder.vertices[0].x - light.pos.x);

            let vertices: Vec<_> = occluder
                .vertices
                .iter()
                .map(|&pos| Vertex {
                    angle: (pos.y - light.pos.y).atan2(pos.x - light.pos.x),
                    pos,
                })
                .collect();

            vertices
                .iter()
                .enumerate()
                .for_each(|(i, v)| info!("vertex {i}: {}", v.pos));

            let cmp = |a: &&Vertex, b: &&Vertex| a.angle.total_cmp(&b.angle);

            let mut min_vertex = vertices
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| cmp(a, b))
                .map(|(i, _)| i)
                .unwrap();

            info!("min vertex: {}", vertices[min_vertex].pos);

            let max_vertex = vertices
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| cmp(a, b))
                .map(|(i, _)| i)
                .unwrap();

            info!("max vertex: {}", vertices[max_vertex].pos);
            // info!("Looking at vertices!");

            loop {
                vertices_buffer.push(vertices[min_vertex].clone());
                meta.n_vertices += 1;

                info!("Adding vertex with angle: {}", vertices[min_vertex].angle);

                if min_vertex == max_vertex {
                    break;
                }

                // TODO: increment by 1 instead for non-hollow ocluders

                if min_vertex == 0 {
                    min_vertex = vertices.len() - 1;
                } else {
                    min_vertex -= 1
                };
            }
            info!("n_vertices: {}", meta.n_vertices);
            meta_buffer.push(meta);
            // info!("Done!\n\n");
        }
        meta_buffer.write_buffer(&render_device, &render_queue);
        vertices_buffer.write_buffer(&render_device, &render_queue);

        occluder_set.0.push((meta_buffer, vertices_buffer));
    }
}
