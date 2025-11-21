use std::{f32::consts::PI, slice::Iter};

use crate::{data::ExtractedWorldData, sprites::ExtractedSprites};

use bevy::{
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
    data::{FireflyConfig, UniformFireflyConfig},
    lights::{ExtractedPointLight, LightSet, UniformPointLight},
    occluders::{
        ExtractedOccluder, Occluder2dShape, OccluderSet, UniformOccluder, UniformRoundOccluder,
        UniformVertex,
    },
    sprites::SpriteStencilTexture,
};

#[derive(Component)]
pub(crate) struct BufferedFireflyConfig(pub UniformBuffer<UniformFireflyConfig>);

pub(crate) struct PreparePlugin;

impl Plugin for PreparePlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

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
            softness: match config.softness {
                None => 0.,
                Some(x) => x.min(1.).max(0.),
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
    mut occluders: Query<&ExtractedOccluder>,
    sprites: Res<ExtractedSprites>,
    camera: Single<(&ExtractedWorldData, &Projection)>,
    mut light_set: ResMut<LightSet>,
    mut occluder_set: ResMut<OccluderSet>,
) {
    let Projection::Orthographic(projection) = camera.1 else {
        return;
    };

    let camera_rect = Rect {
        min: projection.area.min + camera.0.camera_pos,
        max: projection.area.max + camera.0.camera_pos,
    };

    *light_set = default();

    let lights = lights.iter().filter(|light| {
        !Rect {
            min: light.pos - light.range,
            max: light.pos + light.range,
        }
        .intersect(camera_rect)
        .is_empty()
    });

    *occluder_set = default();
    for light in lights {
        let mut buffer = UniformBuffer::<UniformPointLight>::default();
        buffer.set(UniformPointLight {
            pos: light.pos,
            color: light.color.to_linear().to_vec3(),
            intensity: light.intensity,
            range: light.range,
            z: light.z,
        });
        buffer.write_buffer(&render_device, &render_queue);

        light_set.0.push(buffer);

        let light_rect = camera_rect.union_point(light.pos).intersect(Rect {
            min: light.pos - light.range,
            max: light.pos + light.range,
        });

        let mut meta_buffer = GpuArrayBuffer::<UniformOccluder>::new(&render_device);
        let mut sequence_buffer = GpuArrayBuffer::<u32>::new(&render_device);
        let mut vertices_buffer = GpuArrayBuffer::<UniformVertex>::new(&render_device);
        let mut round_buffer = GpuArrayBuffer::<UniformRoundOccluder>::new(&render_device);
        let mut id_buffer = GpuArrayBuffer::<f32>::new(&render_device);

        for occluder in &mut occluders {
            if occluder.rect.intersect(light_rect).is_empty() {
                continue;
            }

            let mut meta: UniformOccluder = default();

            let ids: Vec<_> = sprites
                .sprites
                .iter()
                .filter(|x| occluder.ignored_sprites.contains(&x.main_entity))
                .collect();

            meta.n_sprites = ids.len() as u32;
            meta.z = occluder.z;

            meta.round = match occluder.shape.is_round() {
                false => 0,
                true => 1,
            };

            for id in &ids {
                info!("pushing id: {}", id.id);
                id_buffer.push(id.id);
            }

            meta.color = occluder.color.to_linear().to_vec3();
            meta.opacity = occluder.opacity;

            if let Occluder2dShape::RoundRectangle {
                width,
                height,
                radius,
            } = occluder.shape
            {
                round_buffer.push(UniformRoundOccluder {
                    pos: occluder.pos,
                    rot: occluder.rot,
                    width,
                    height,
                    radius,
                });
                meta_buffer.push(meta);
                continue;
            }

            let angle = |a: Vec2, b: Vec2| (a.y - b.y).atan2(a.x - b.x);

            let mut vertices: Vec<_> = occluder
                .vertices()
                .iter()
                .map(|&pos| UniformVertex {
                    angle: angle(pos, light.pos),
                    pos,
                })
                .collect();

            let mut push_slice = |slice: &Vec<UniformVertex>| {
                sequence_buffer.push(slice.len() as u32);
                for vertex in slice {
                    vertices_buffer.push(vertex.clone());
                }
                meta.n_vertices += slice.len() as u32;
                meta.n_sequences += 1;
            };

            let mut process_vertices = |vertices: Iter<UniformVertex>| {
                let mut slice: Vec<UniformVertex> = default();

                for vertex in vertices {
                    if let Some(last) = slice.last() {
                        let loops = (vertex.angle - last.angle).abs() > PI;

                        // if the next vertex is decreasing
                        if (!loops && vertex.angle < last.angle)
                            || (loops && vertex.angle > last.angle)
                        {
                            if slice.len() > 1 {
                                push_slice(&slice);
                            }
                            slice = vec![vertex.clone()];
                        }
                        // if the next vertex is increasing, simple case
                        else if !loops && vertex.angle > last.angle {
                            slice.push(vertex.clone());
                        }
                        // if the next vertex is increasing and loops over
                        else {
                            let mut old_vertex = last.clone();
                            let mut new_vertex = vertex.clone();
                            new_vertex.angle += 2. * PI;
                            slice.push(new_vertex.clone());

                            push_slice(&slice);

                            old_vertex.angle -= 2. * PI;
                            slice = vec![old_vertex, vertex.clone()];
                        }
                    } else {
                        slice.push(vertex.clone());
                    }
                }

                if slice.len() > 1 {
                    push_slice(&slice);
                }
            };

            process_vertices(vertices.iter());
            vertices.reverse();
            process_vertices(vertices.iter());

            meta_buffer.push(meta);
        }
        meta_buffer.push(default());
        sequence_buffer.push(default());
        vertices_buffer.push(default());
        round_buffer.push(default());
        id_buffer.push(default());

        meta_buffer.write_buffer(&render_device, &render_queue);
        sequence_buffer.write_buffer(&render_device, &render_queue);
        vertices_buffer.write_buffer(&render_device, &render_queue);
        round_buffer.write_buffer(&render_device, &render_queue);
        id_buffer.write_buffer(&render_device, &render_queue);

        occluder_set.0.push((
            meta_buffer,
            sequence_buffer,
            vertices_buffer,
            round_buffer,
            id_buffer,
        ));
    }
}
