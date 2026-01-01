use std::f32::consts::PI;

use crate::{
    LightmapPhase, NormalMapTexture, SpriteStencilTexture,
    buffers::{BinBuffer, BufferManager, OccluderPointer, VertexBuffer},
    data::{ExtractedWorldData, NormalMode},
    lights::{Falloff, LightBatch, LightBatches, LightBindGroups, LightBuffers},
    occluders::{PolyOccluderIndex, RoundOccluderIndex, point_inside_poly, translate_vertices},
    phases::SpritePhase,
    pipelines::{LightmapCreationPipeline, SpritePipeline},
    sprites::{
        ExtractedSlices, ExtractedSpriteKind, ExtractedSprites, ImageBindGroups, SpriteAssetEvents,
        SpriteBatch, SpriteBatches, SpriteInstance, SpriteMeta, SpriteViewBindGroup,
    },
    utils::apply_scaling,
};

use bevy::{
    core_pipeline::tonemapping::{Tonemapping, TonemappingLuts, get_lut_bindings},
    math::{
        Affine3A,
        bounding::{Aabb2d, IntersectsVolume},
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_asset::RenderAssets,
        render_phase::{PhaseItem, ViewBinnedRenderPhases, ViewSortedRenderPhases},
        render_resource::{
            BindGroup, BindGroupEntries, BufferUsages, BufferVec, TextureDescriptor,
            TextureDimension, TextureFormat, TextureUsages, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{FallbackImage, GpuImage, TextureCache},
        view::{ExtractedView, ViewTarget, ViewUniforms},
    },
    tasks::{ComputeTaskPool, ParallelSlice, ParallelSliceMut},
};

use crate::{
    LightMapTexture,
    data::{FireflyConfig, UniformFireflyConfig},
    lights::{ExtractedPointLight, UniformPointLight},
    occluders::{ExtractedOccluder, Occluder2dShape, UniformOccluder, UniformRoundOccluder},
};

#[derive(Component)]
pub(crate) struct BufferedFireflyConfig(pub UniformBuffer<UniformFireflyConfig>);

pub(crate) struct PreparePlugin;

impl Plugin for PreparePlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            Render,
            (insert_light_buffers, prepare_data)
                .chain()
                .in_set(RenderSystems::Prepare),
        );
        render_app.add_systems(Render, prepare_config.in_set(RenderSystems::Prepare));
        render_app.add_systems(Render, prepare_lightmap.in_set(RenderSystems::Prepare));

        render_app.add_systems(
            Render,
            (
                prepare_sprite_view_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                (prepare_sprite_image_bind_groups.in_set(RenderSystems::PrepareBindGroups),)
                    .chain(),
            ),
        );
    }
}

fn prepare_config(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    configs: Query<(Entity, &FireflyConfig)>,
    mut commands: Commands,
) {
    for (entity, config) in &configs {
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

            z_sorting: match config.z_sorting {
                false => 0,
                true => 1,
            },

            normal_mode: match config.normal_mode {
                NormalMode::None => 0,
                NormalMode::Simple => 1,
                NormalMode::TopDown => 2,
            },

            normal_attenuation: config.normal_attenuation,
        };
        let mut buffer = UniformBuffer::<UniformFireflyConfig>::from(uniform);
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

        let normal_map_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("normal map"),
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
            SpriteStencilTexture(sprite_stencil_texture),
            NormalMapTexture(normal_map_texture),
        ));
    }
}

fn insert_light_buffers(
    lights: Query<(Entity, Has<LightBuffers>), With<ExtractedPointLight>>,
    mut commands: Commands,
) {
    for (light, has_buffers) in lights {
        if !has_buffers {
            commands.entity(light).insert(LightBuffers {
                light: UniformBuffer::<UniformPointLight>::from(UniformPointLight::default()),
            });
        }
    }
}

pub(crate) fn prepare_data(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut lights: Query<(
        Entity,
        &ExtractedPointLight,
        &mut LightBuffers,
        &mut BinBuffer,
    )>,
    occluders: Query<(&ExtractedOccluder, &RoundOccluderIndex, &PolyOccluderIndex)>,
    camera: Single<(
        &ExtractedWorldData,
        &Projection,
        &SpriteStencilTexture,
        &NormalMapTexture,
        &BufferedFireflyConfig,
        &FireflyConfig,
    )>,
    phases: Res<ViewBinnedRenderPhases<LightmapPhase>>,
    lightmap_pipeline: Res<LightmapCreationPipeline>,
    mut light_bind_groups: ResMut<LightBindGroups>,
    mut batches: ResMut<LightBatches>,
    view_uniforms: Res<ViewUniforms>,
    round_occluders: Res<BufferManager<UniformRoundOccluder>>,
    poly_occluders: Res<BufferManager<UniformOccluder>>,
    vertices: Res<VertexBuffer>,
) {
    let Projection::Orthographic(projection) = camera.1 else {
        return;
    };

    let camera_rect = Rect {
        min: projection.area.min + camera.0.camera_pos,
        max: projection.area.max + camera.0.camera_pos,
    };

    batches.clear();

    let light_bind_groups = &mut *light_bind_groups;

    let mut lights: Vec<_> = lights.iter_mut().collect();

    for (retained_view, _) in phases.iter() {
        lights
            .par_splat_map_mut(ComputeTaskPool::get(), None, |_, lights| {
                let mut bind_groups: Vec<(Entity, BindGroup)> = vec![];

                for (entity, light, buffers, bins) in lights {
                    let uniform_light = UniformPointLight {
                        pos: light.pos,
                        color: light.color.to_linear().to_vec3(),
                        intensity: light.intensity,
                        range: light.range,
                        z: light.z,
                        inner_range: light.inner_range.min(light.range),
                        falloff: match light.falloff {
                            Falloff::InverseSquare => 0,
                            Falloff::Linear => 1,
                        },
                        angle: light.angle / 180. * PI,
                        dir: light.dir,
                        height: light.height,
                        n_rounds: 0,
                        n_poly: 0,
                    };
                    let light_rect = camera_rect.union_point(light.pos).intersect(Rect {
                        min: light.pos - light.range,
                        max: light.pos + light.range,
                    });

                    let light_aabb = Aabb2d {
                        min: light_rect.min,
                        max: light_rect.max,
                    };

                    bins.reset();

                    let softness = match camera.5.softness {
                        None => 0.0,
                        Some(x) => x.clamp(0.0, 1.0),
                    };

                    for (occluder, round_index, poly_index) in &occluders {
                        if !light.cast_shadows || !occluder.aabb.intersects(&light_aabb) {
                            continue;
                        }

                        if let Occluder2dShape::RoundRectangle {
                            width,
                            height,
                            radius,
                        } = occluder.shape
                        {
                            let Some(occluder_index) = round_index.0 else {
                                continue;
                            };

                            let vertices = vec![
                                vec2(-width / 2.0 - radius, -height / 2.0 - radius),
                                vec2(-width / 2.0 - radius, height / 2.0 + radius),
                                vec2(width / 2.0 + radius, height / 2.0 + radius),
                                vec2(width / 2.0 + radius, -height / 2.0 - radius),
                                vec2(-width / 2.0 - radius, -height / 2.0 - radius),
                            ];

                            let isometry = Isometry2d {
                                translation: occluder.pos,
                                rotation: Rot2::radians(occluder.rot),
                            };
                            let aabb = Aabb2d::from_point_cloud(isometry, &vertices);

                            let vertices = translate_vertices(
                                vertices,
                                isometry.translation,
                                isometry.rotation,
                            );

                            let light_inside_occluder =
                                point_inside_poly(light.pos, vertices.clone(), aabb);

                            let closest = aabb.closest_point(light.pos);

                            push_vertices(
                                bins,
                                vertices,
                                light.pos,
                                0,
                                occluder_index.index as u32,
                                softness,
                                closest.distance(light.pos),
                                light_inside_occluder,
                                false,
                            );
                        } else {
                            let Some(occluder_index) = poly_index.occluder else {
                                continue;
                            };

                            let Some(vertex_index) = poly_index.vertices else {
                                continue;
                            };

                            let light_inside_occluder =
                                matches!(occluder.shape, Occluder2dShape::Polygon { .. })
                                    && point_inside_poly(
                                        light.pos,
                                        occluder.vertices(),
                                        occluder.aabb,
                                    );

                            let closest = occluder.aabb.closest_point(light.pos);

                            push_vertices(
                                bins,
                                occluder.vertices(),
                                light.pos,
                                vertex_index.index as u32,
                                occluder_index.index as u32,
                                softness,
                                closest.distance(light.pos),
                                light_inside_occluder,
                                true,
                            );
                        }
                    }

                    buffers.light.set(uniform_light);
                    buffers.light.write_buffer(&render_device, &render_queue);

                    bins.write(&render_device, &render_queue);

                    bind_groups.push((
                        *entity,
                        render_device.create_bind_group(
                            "light bind group",
                            &lightmap_pipeline.layout,
                            &BindGroupEntries::sequential((
                                view_uniforms.uniforms.binding().unwrap(),
                                &lightmap_pipeline.sampler,
                                buffers.light.binding().unwrap(),
                                round_occluders.binding(),
                                poly_occluders.binding(),
                                vertices.binding(),
                                bins.binding(),
                                &camera.2.0.default_view,
                                &camera.3.0.default_view,
                                camera.4.0.binding().unwrap(),
                            )),
                        ),
                    ));
                }
                bind_groups
            })
            .iter()
            .for_each(|bind_groups| {
                for (entity, bind_group) in bind_groups {
                    light_bind_groups
                        .values
                        .entry(*entity)
                        .insert(bind_group.clone());

                    batches
                        .entry((*retained_view, *entity))
                        .insert(LightBatch { id: *entity });
                }
            });
    }
}

#[derive(Default)]
struct OccluderSlice {
    pub start: u32,
    pub length: u32,
    pub term: u32,

    pub start_angle: f32,
    pub end_angle: f32,
}

struct Vertex {
    pub index: u32,
    pub angle: f32,
}

fn push_vertices(
    bins: &mut BinBuffer,
    occluder_vertices: Vec<Vec2>,
    light_pos: Vec2,
    start_vertex: u32,
    index: u32,
    softness: f32,
    distance: f32,
    rev: bool,
    poly: bool,
) {
    let mut push_slice = |slice: &OccluderSlice| {
        if slice.length > 1 {
            let rev: u32 = match rev {
                true => 1,
                false => 0,
            };

            let index = match poly {
                true => (1 << 31) | (slice.term << 29) | (rev << 28) | index as u32,
                false => (0 << 31) | index as u32,
            };

            let occluder = OccluderPointer {
                index,
                min_v: slice.start + start_vertex,
                length: slice.length,
                distance,
            };

            match slice.term {
                0 => bins.add_occluder(
                    occluder,
                    slice.start_angle - softness,
                    slice.end_angle + softness,
                ),
                1 => bins.add_occluder(
                    occluder,
                    slice.start_angle - softness,
                    slice.end_angle + (2.0 * PI),
                ),
                2 => bins.add_occluder(
                    occluder,
                    slice.start_angle - (2.0 * PI),
                    slice.end_angle + softness,
                ),
                _ => {}
            }
        }
    };

    let vertices = occluder_vertices.iter().enumerate().map(|(i, v)| Vertex {
        index: i as u32,
        angle: (v.y - light_pos.y).atan2(v.x - light_pos.x),
    });

    let vertices: Vec<_> = if !rev {
        vertices.collect()
    } else {
        vertices.rev().collect()
    };

    let mut last: Option<&Vertex> = None;
    let mut slice: OccluderSlice = default();

    for vertex in &vertices {
        if let Some(last) = last {
            let loops = (vertex.angle - last.angle).abs() > PI;

            // if the next vertex is decreasing
            if (!loops && vertex.angle < last.angle) || (loops && vertex.angle > last.angle) {
                push_slice(&slice);
                slice = OccluderSlice {
                    start: vertex.index,
                    length: 1,
                    term: 0,
                    start_angle: vertex.angle,
                    end_angle: vertex.angle,
                };
            }
            // if the next vertex is increasing, simple case
            else if !loops && vertex.angle > last.angle {
                if slice.length == 0 {
                    slice.start = vertex.index;
                }
                slice.length += 1;
                slice.end_angle = vertex.angle;
            }
            // if the next vertex is increasing and loops over
            else {
                if slice.length == 0 {
                    slice.start = vertex.index;
                }
                slice.length += 1;
                slice.term = 1;

                push_slice(&slice);

                slice = OccluderSlice {
                    start: last.index,
                    length: 2,
                    term: 2,
                    start_angle: last.angle,
                    end_angle: vertex.angle,
                };
            }
        } else {
            slice = OccluderSlice {
                start: vertex.index,
                length: 1,
                term: 0,
                start_angle: vertex.angle,
                end_angle: vertex.angle,
            };
        }

        last = Some(vertex);
    }

    push_slice(&slice);
}

fn prepare_sprite_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    sprite_pipeline: Res<SpritePipeline>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &Tonemapping), With<ExtractedView>>,
    tonemapping_luts: Res<TonemappingLuts>,
    images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
) {
    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };

    for (entity, tonemapping) in &views {
        let lut_bindings =
            get_lut_bindings(&images, &tonemapping_luts, tonemapping, &fallback_image);
        let view_bind_group = render_device.create_bind_group(
            "mesh2d_view_bind_group",
            &sprite_pipeline.view_layout,
            &BindGroupEntries::with_indices((
                (0, view_binding.clone()),
                (1, lut_bindings.0),
                (2, lut_bindings.1),
            )),
        );

        commands.entity(entity).insert(SpriteViewBindGroup {
            value: view_bind_group,
        });
    }
}

fn prepare_sprite_image_bind_groups(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut sprite_meta: ResMut<SpriteMeta>,
    sprite_pipeline: Res<SpritePipeline>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    extracted_sprites: Res<ExtractedSprites>,
    extracted_slices: Res<ExtractedSlices>,
    mut phases: ResMut<ViewSortedRenderPhases<SpritePhase>>,
    events: Res<SpriteAssetEvents>,
    mut batches: ResMut<SpriteBatches>,
) {
    let mut is_dummy = UniformBuffer::<u32>::from(0);
    is_dummy.write_buffer(&render_device, &render_queue);

    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added { .. } |
            // Images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Unused { id } | AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.retain(|k, _| k.0 != *id && k.1 != *id);
            }
        };
    }

    batches.clear();

    // Clear the sprite instances
    sprite_meta.sprite_instance_buffer.clear();

    // Index buffer indices
    let mut index = 0;

    let image_bind_groups = &mut *image_bind_groups;

    for (retained_view, transparent_phase) in phases.iter_mut() {
        let mut current_batch = None;
        let mut batch_item_index = 0;
        let mut batch_image_size = Vec2::ZERO;
        let mut batch_image_handle = AssetId::invalid();
        let mut batch_normal_handle;
        let mut is_dummy;

        // Iterate through the phase items and detect when successive sprites that can be batched.
        // Spawn an entity with a `SpriteBatch` component for each possible batch.
        // Compatible items share the same entity.
        for item_index in 0..transparent_phase.items.len() {
            let item = &transparent_phase.items[item_index];

            let Some(extracted_sprite) = extracted_sprites
                .sprites
                .get(item.extracted_index)
                .filter(|extracted_sprite| extracted_sprite.render_entity == item.entity())
            else {
                // If there is a phase item that is not a sprite, then we must start a new
                // batch to draw the other phase item(s) and to respect draw order. This can be
                // done by invalidating the batch_image_handle
                batch_image_handle = AssetId::invalid();
                continue;
            };

            if batch_image_handle != extracted_sprite.image_handle_id {
                let Some(gpu_image) = gpu_images.get(extracted_sprite.image_handle_id) else {
                    continue;
                };

                batch_image_size = gpu_image.size_2d().as_vec2();
                batch_image_handle = extracted_sprite.image_handle_id;

                (batch_normal_handle, is_dummy) = match extracted_sprite.normal_handle_id {
                    None => (batch_image_handle, true),
                    Some(x) => (x, false),
                };

                let Some(normal_image) = (if is_dummy {
                    Some(gpu_image)
                } else {
                    gpu_images.get(batch_normal_handle)
                }) else {
                    continue;
                };

                let mut dummy_buffer = UniformBuffer::<u32>::from(if is_dummy { 1 } else { 0 });
                dummy_buffer.write_buffer(&render_device, &render_queue);

                image_bind_groups
                    .values
                    .entry((batch_image_handle, batch_normal_handle, is_dummy))
                    .or_insert_with(|| {
                        render_device.create_bind_group(
                            "sprite_material_bind_group",
                            &sprite_pipeline.material_layout,
                            &BindGroupEntries::sequential((
                                &gpu_image.texture_view,
                                &normal_image.texture_view,
                                &gpu_image.sampler,
                                dummy_buffer.binding().unwrap(),
                            )),
                        )
                    });

                batch_item_index = item_index;
                current_batch = Some(batches.entry((*retained_view, item.entity())).insert(
                    SpriteBatch {
                        image_handle_id: batch_image_handle,
                        normal_handle_id: batch_normal_handle,
                        normal_dummy: is_dummy,
                        range: index..index,
                    },
                ));
            }
            match extracted_sprite.kind {
                ExtractedSpriteKind::Single {
                    anchor,
                    rect,
                    scaling_mode,
                    custom_size,
                } => {
                    // By default, the size of the quad is the size of the texture
                    let mut quad_size = batch_image_size;
                    let mut texture_size = batch_image_size;

                    // Calculate vertex data for this item
                    // If a rect is specified, adjust UVs and the size of the quad
                    let mut uv_offset_scale = if let Some(rect) = rect {
                        let rect_size = rect.size();
                        quad_size = rect_size;
                        // Update texture size to the rect size
                        // It will help scale properly only portion of the image
                        texture_size = rect_size;
                        Vec4::new(
                            rect.min.x / batch_image_size.x,
                            rect.max.y / batch_image_size.y,
                            rect_size.x / batch_image_size.x,
                            -rect_size.y / batch_image_size.y,
                        )
                    } else {
                        Vec4::new(0.0, 1.0, 1.0, -1.0)
                    };

                    if extracted_sprite.flip_x {
                        uv_offset_scale.x += uv_offset_scale.z;
                        uv_offset_scale.z *= -1.0;
                    }
                    if extracted_sprite.flip_y {
                        uv_offset_scale.y += uv_offset_scale.w;
                        uv_offset_scale.w *= -1.0;
                    }

                    // Override the size if a custom one is specified
                    quad_size = custom_size.unwrap_or(quad_size);

                    // Used for translation of the quad if `TextureScale::Fit...` is specified.
                    let mut quad_translation = Vec2::ZERO;

                    // Scales the texture based on the `texture_scale` field.
                    if let Some(scaling_mode) = scaling_mode {
                        apply_scaling(
                            scaling_mode,
                            texture_size,
                            &mut quad_size,
                            &mut quad_translation,
                            &mut uv_offset_scale,
                        );
                    }

                    let transform = extracted_sprite.transform.affine()
                        * Affine3A::from_scale_rotation_translation(
                            quad_size.extend(1.0),
                            Quat::IDENTITY,
                            ((quad_size + quad_translation) * (-anchor - Vec2::splat(0.5)))
                                .extend(0.0),
                        );

                    // Store the vertex data and add the item to the render phase
                    sprite_meta
                        .sprite_instance_buffer
                        .push(SpriteInstance::from(
                            &transform,
                            &uv_offset_scale,
                            extracted_sprite.transform.translation().z,
                            extracted_sprite.height,
                        ));

                    current_batch.as_mut().unwrap().get_mut().range.end += 1;
                    index += 1;
                }
                ExtractedSpriteKind::Slices { ref indices } => {
                    for i in indices.clone() {
                        let slice = &extracted_slices.slices[i];
                        let rect = slice.rect;
                        let rect_size = rect.size();

                        // Calculate vertex data for this item
                        let mut uv_offset_scale: Vec4;

                        // If a rect is specified, adjust UVs and the size of the quad
                        uv_offset_scale = Vec4::new(
                            rect.min.x / batch_image_size.x,
                            rect.max.y / batch_image_size.y,
                            rect_size.x / batch_image_size.x,
                            -rect_size.y / batch_image_size.y,
                        );

                        if extracted_sprite.flip_x {
                            uv_offset_scale.x += uv_offset_scale.z;
                            uv_offset_scale.z *= -1.0;
                        }
                        if extracted_sprite.flip_y {
                            uv_offset_scale.y += uv_offset_scale.w;
                            uv_offset_scale.w *= -1.0;
                        }

                        let transform = extracted_sprite.transform.affine()
                            * Affine3A::from_scale_rotation_translation(
                                slice.size.extend(1.0),
                                Quat::IDENTITY,
                                (slice.size * -Vec2::splat(0.5) + slice.offset).extend(0.0),
                            );

                        // Store the vertex data and add the item to the render phase
                        sprite_meta
                            .sprite_instance_buffer
                            .push(SpriteInstance::from(
                                &transform,
                                &uv_offset_scale,
                                extracted_sprite.transform.translation().z,
                                extracted_sprite.height,
                            ));

                        current_batch.as_mut().unwrap().get_mut().range.end += 1;
                        index += 1;
                    }
                }
            }
            transparent_phase.items[batch_item_index]
                .batch_range_mut()
                .end += 1;
        }
        sprite_meta
            .sprite_instance_buffer
            .write_buffer(&render_device, &render_queue);

        if sprite_meta.sprite_index_buffer.len() != 6 {
            sprite_meta.sprite_index_buffer.clear();

            // NOTE: This code is creating 6 indices pointing to 4 vertices.
            // The vertices form the corners of a quad based on their two least significant bits.
            // 10   11
            //
            // 00   01
            // The sprite shader can then use the two least significant bits as the vertex index.
            // The rest of the properties to transform the vertex positions and UVs (which are
            // implicit) are baked into the instance transform, and UV offset and scale.
            // See bevy_sprite/src/render/sprite.wgsl for the details.
            sprite_meta.sprite_index_buffer.push(2);
            sprite_meta.sprite_index_buffer.push(0);
            sprite_meta.sprite_index_buffer.push(1);
            sprite_meta.sprite_index_buffer.push(1);
            sprite_meta.sprite_index_buffer.push(3);
            sprite_meta.sprite_index_buffer.push(2);

            sprite_meta
                .sprite_index_buffer
                .write_buffer(&render_device, &render_queue);
        }
    }
}
