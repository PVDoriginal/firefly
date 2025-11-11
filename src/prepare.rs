use std::f32::{EPSILON, consts::PI};

use bevy::{
    math::ops::floor,
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        render_resource::{
            GpuArrayBuffer, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
            UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::TextureCache,
        view::ViewTarget,
    },
};

use crate::{
    EmptyLightMapTexture, IntermediaryLightMapTexture, LightMapTexture,
    data::{FireflyConfig, UniformFireflyConfig, UniformMeta},
    lights::{ExtractedPointLight, LightSet, UniformPointLight},
    occluders::{
        ExtractedOccluder, OccluderSet, UniformOccluder, UniformVertex, point_inside_poly,
    },
    sprites::SpriteStencilTexture,
};

#[derive(Resource, Default)]
pub(crate) struct LightingDataBuffer(pub UniformBuffer<UniformMeta>);

#[derive(Component)]
pub(crate) struct BufferedFireflyConfig(pub UniformBuffer<UniformFireflyConfig>);

pub(crate) struct PreparePlugin;

impl Plugin for PreparePlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<LightingDataBuffer>();
        render_app.init_resource::<LightSet>();

        render_app.add_systems(Render, prepare_data.in_set(RenderSet::Prepare));
        render_app.add_systems(Render, prepare_config.in_set(RenderSet::Prepare));
        render_app.add_systems(Render, prepare_lightmap.in_set(RenderSet::Prepare));
    }
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<OccluderSet>();
    }
}

fn prepare_config(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    configs: Query<(Entity, &FireflyConfig)>,
    mut commands: Commands,
) {
    for (entity, config) in &configs {
        let mut buffer = UniformBuffer::<UniformFireflyConfig>::default();
        let uniform = UniformFireflyConfig {
            ambient_color: config.ambient_color.to_linear().to_vec3(),
            ambient_brightness: config.ambient_brightness,
            light_bands: match config.light_bands {
                None => 0,
                Some(x) => x,
            },
        };
        buffer.set(uniform);
        buffer.write_buffer(&render_device, &render_queue);
        commands
            .entity(entity)
            .insert(BufferedFireflyConfig(buffer));
    }
}

fn prepare_lightmap(
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

        let sprite_stencil_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("sprite stencil"),
                size: view_target.main_texture().size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        commands.entity(entity).insert((
            LightMapTexture(light_map_texture),
            IntermediaryLightMapTexture(inter_light_map_texture),
            EmptyLightMapTexture(empty_light_map_texture),
            SpriteStencilTexture(sprite_stencil_texture),
        ));
    }
}

fn prepare_data(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    lights: Query<&ExtractedPointLight>,
    occluders: Query<&ExtractedOccluder>,
    mut data_buffer: ResMut<LightingDataBuffer>,
    mut light_set: ResMut<LightSet>,
    mut occluder_set: ResMut<OccluderSet>,
) {
    let data = UniformMeta {
        n_occluders: occluders.iter().len() as u32,
    };

    data_buffer.0.set(data);
    data_buffer.0.write_buffer(&render_device, &render_queue);

    *light_set = default();
    for light in lights {
        let mut buffer = UniformBuffer::<UniformPointLight>::default();
        buffer.set(UniformPointLight {
            pos: light.pos,
            color: light.color.to_linear().to_vec3(),
            intensity: light.intensity,
            range: light.range,
        });
        buffer.write_buffer(&render_device, &render_queue);

        light_set.0.push(buffer);
    }
    *occluder_set = default();

    for light in lights {
        let light_pos = light.pos;

        let mut meta_buffer = GpuArrayBuffer::<UniformOccluder>::new(&render_device);
        let mut vertices_buffer = GpuArrayBuffer::<UniformVertex>::new(&render_device);

        if occluders.is_empty() {
            meta_buffer.push(default());
            vertices_buffer.push(default());
        }

        for occluder in occluders {
            let mut meta: UniformOccluder = default();
            meta.sprite_id = occluder.sprite_id;
            meta.z = occluder.z;

            // warn!("setting occluder sprite id: {}", meta.sprite_id);

            meta.line = match occluder.shape.is_line() {
                false => 0,
                true => 1,
            };

            meta.concave = match occluder.shape.is_concave() {
                false => 0,
                true => 1,
            };

            if occluder.shape.is_concave() {
                meta.n_vertices = occluder.vertices().len() as u32;

                let mut vertices = occluder.vertices().clone();

                // TODO: temp
                if meta.line == 0 && point_inside_poly(light_pos, occluder.vertices()) {
                    meta.n_vertices += 1;
                    vertices.push(*vertices.first().unwrap());
                    meta.line = 1;
                }

                meta_buffer.push(meta);

                vertices.iter().for_each(|&pos| {
                    vertices_buffer.push(UniformVertex { angle: 0., pos });
                });

                continue;
            }

            let angle = |a: Vec2, b: Vec2| (a.y - b.y).atan2(a.x - b.x);

            if point_inside_poly(light_pos, occluder.vertices()) {
                let ref_angle = angle(*occluder.vertices().last().unwrap(), light_pos) - 0.001;
                meta.seam = ref_angle;

                meta.n_vertices = occluder.vertices().len() as u32 + 1;

                for &pos in occluder.vertices().iter().rev() {
                    vertices_buffer.push(UniformVertex {
                        angle: (angle(pos, light_pos) - meta.seam)
                            + 2. * PI * floor((meta.seam - angle(pos, light_pos)) / (2. * PI)),
                        pos,
                    });
                }

                let pos = *occluder.vertices().last().unwrap();

                vertices_buffer.push(UniformVertex {
                    angle: (angle(pos, light_pos) - meta.seam)
                        + 2. * PI * floor((meta.seam - angle(pos, light_pos)) / (2. * PI))
                        + 2. * PI,
                    pos,
                });
                meta_buffer.push(meta);
                continue;
            }

            let ref_angle = angle(occluder.vertices()[0], light_pos);

            if ref_angle > 0. {
                meta.seam = ref_angle - PI;
            } else {
                meta.seam = ref_angle + PI;
            }

            let mut vertices: Vec<_> = occluder
                .vertices()
                .iter()
                .map(|&pos| UniformVertex {
                    angle: (angle(pos, light_pos) - meta.seam)
                        + 2. * PI * floor((meta.seam - angle(pos, light_pos)) / (2. * PI)),
                    pos,
                })
                .collect();

            let cmp = |a: &&UniformVertex, b: &&UniformVertex| a.angle.total_cmp(&b.angle);

            let mut min_vertex = vertices
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| cmp(a, b))
                .map(|(i, _)| i)
                .unwrap();

            let max_vertex = vertices
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| cmp(a, b))
                .map(|(i, _)| i)
                .unwrap();

            loop {
                vertices_buffer.push(vertices[min_vertex].clone());
                meta.n_vertices += 1;

                if min_vertex == max_vertex {
                    break;
                }
                min_vertex = (min_vertex + 1) % vertices.len();
            }

            meta_buffer.push(meta);
        }
        meta_buffer.write_buffer(&render_device, &render_queue);
        vertices_buffer.write_buffer(&render_device, &render_queue);

        occluder_set.0.push((meta_buffer, vertices_buffer));
    }
}
