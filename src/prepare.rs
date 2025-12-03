use std::{
    f32::consts::{FRAC_PI_2, PI},
    slice::Iter,
    sync::{Arc, Mutex},
};

use crate::{
    LightmapPhase, NormalMapTexture, SpriteStencilTexture,
    data::ExtractedWorldData,
    lights::{Falloff, LightBatch, LightBatches, LightBindGroups, LightBufferMeta},
    occluders::point_inside_poly,
    phases::{NormalPhase, Stencil2d},
    pipelines::{LightmapCreationPipeline, SpriteNormalMapsPipeline, SpriteStencilPipeline},
    sprites::{
        ExtractedSlices, ExtractedSpriteKind, ExtractedSprites, ImageBindGroups, SpriteAssetEvents,
        SpriteBatch, SpriteInstance, SpriteNormalBatches, SpriteNormalMeta, SpriteStencilBatches,
        SpriteStencilMeta, SpriteViewBindGroup,
    },
    utils::apply_scaling,
};

use bevy::{
    core_pipeline::tonemapping::{Tonemapping, TonemappingLuts, get_lut_bindings},
    math::Affine3A,
    pbr::LightMeta,
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        render_asset::RenderAssets,
        render_phase::{
            PhaseItem, SortedPhaseItem, ViewBinnedRenderPhases, ViewSortedRenderPhases,
        },
        render_resource::{
            BindGroupEntries, GpuArrayBuffer, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{FallbackImage, GpuImage, TextureCache},
        view::{ExtractedView, ViewTarget, ViewUniforms},
    },
    tasks::{ComputeTaskPool, ParallelSlice},
};

use crate::{
    EmptyLightMapTexture, IntermediaryLightMapTexture, LightMapTexture,
    data::{FireflyConfig, UniformFireflyConfig},
    lights::{ExtractedPointLight, LightSet, UniformPointLight},
    occluders::{
        ExtractedOccluder, Occluder2dShape, OccluderSet, UniformOccluder, UniformRoundOccluder,
        UniformVertex,
    },
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

        render_app.add_systems(
            Render,
            (
                prepare_sprite_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                (
                    prepare_sprite_image_bind_groups_stencil.in_set(RenderSet::PrepareBindGroups),
                    prepare_sprite_image_bind_groups_normal.in_set(RenderSet::PrepareBindGroups),
                )
                    .chain(),
            ),
        );
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
            z_sorting: match config.z_sorting {
                false => 0,
                true => 1,
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
            IntermediaryLightMapTexture(inter_light_map_texture),
            EmptyLightMapTexture(empty_light_map_texture),
            SpriteStencilTexture(sprite_stencil_texture),
            NormalMapTexture(normal_map_texture),
        ));
    }
}

fn prepare_data(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    lights: Query<(Entity, &ExtractedPointLight)>,
    occluders: Query<&ExtractedOccluder>,
    sprites: Res<ExtractedSprites>,
    camera: Single<(
        &ExtractedWorldData,
        &Projection,
        &SpriteStencilTexture,
        &NormalMapTexture,
        &BufferedFireflyConfig,
    )>,
    mut light_set: ResMut<LightSet>,
    mut occluder_set: ResMut<OccluderSet>,
    mut phases: ResMut<ViewBinnedRenderPhases<LightmapPhase>>,
    mut light_buffer_meta: ResMut<LightBufferMeta>,
    lightmap_pipeline: Res<LightmapCreationPipeline>,
    mut light_bind_groups: ResMut<LightBindGroups>,
    mut batches: ResMut<LightBatches>,
    view_uniforms: Res<ViewUniforms>,
) {
    let Projection::Orthographic(projection) = camera.1 else {
        return;
    };

    let camera_rect = Rect {
        min: projection.area.min + camera.0.camera_pos,
        max: projection.area.max + camera.0.camera_pos,
    };

    batches.clear();

    // Clear the sprite instances
    light_buffer_meta.light_index_buffer.clear();

    // Index buffer indices
    let mut index = 0;

    let light_bind_groups = &mut *light_bind_groups;

    for (retained_view, transparent_phase) in phases.iter_mut() {
        let mut batch_item_index = index;

        let mut index = 0;

        for item_index in 0..transparent_phase.non_mesh_items.len() {
            let item = &transparent_phase.non_mesh_items[item_index];

            for (_, entity) in &item.entities {
                let Ok((_, light)) = lights.get(*entity) else {
                    continue;
                };

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
                };

                let mut light_buffer = UniformBuffer::<UniformPointLight>::from(uniform_light);
                light_buffer.write_buffer(&render_device, &render_queue);

                let light_rect = camera_rect.union_point(light.pos).intersect(Rect {
                    min: light.pos - light.range,
                    max: light.pos + light.range,
                });

                let mut meta_buffer = GpuArrayBuffer::<UniformOccluder>::new(&render_device);
                let mut sequence_buffer = GpuArrayBuffer::<u32>::new(&render_device);
                let mut vertices_buffer = GpuArrayBuffer::<UniformVertex>::new(&render_device);
                let mut round_buffer = GpuArrayBuffer::<UniformRoundOccluder>::new(&render_device);
                let mut id_buffer = GpuArrayBuffer::<f32>::new(&render_device);

                for occluder in &occluders {
                    if !light.cast_shadows {
                        break;
                    }

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
                    meta.height = occluder.height;

                    meta.z_sorting = match occluder.z_sorting {
                        false => 0,
                        true => 1,
                    };

                    for id in &ids {
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
                        meta.round = 1;
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
                    let vertices_iter = || {
                        Box::new(occluder.vertices_iter().map(|pos| UniformVertex {
                            angle: angle(pos, light.pos),
                            pos,
                        }))
                    };

                    let light_inside_occluder =
                        matches!(occluder.shape, Occluder2dShape::Polygon { .. })
                            && point_inside_poly(light.pos, occluder.vertices(), occluder.rect);

                    let mut push_slice = |slice: &Vec<UniformVertex>| {
                        sequence_buffer.push(slice.len() as u32);
                        for vertex in slice {
                            vertices_buffer.push(vertex.clone());
                        }
                        meta.n_vertices += slice.len() as u32;
                        meta.n_sequences += 1;
                    };

                    let mut push_vertices =
                        |vertices: Box<dyn DoubleEndedIterator<Item = UniformVertex>>| {
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

                    if !light_inside_occluder {
                        push_vertices(vertices_iter());
                    } else {
                        push_vertices(Box::new(vertices_iter().rev()));
                    }

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

                light_bind_groups.values.entry(*entity).insert({
                    render_device.create_bind_group(
                        "light bind group",
                        &lightmap_pipeline.layout,
                        &BindGroupEntries::sequential((
                            view_uniforms.uniforms.binding().unwrap(),
                            &lightmap_pipeline.sampler,
                            light_buffer.binding().unwrap(),
                            meta_buffer.binding().unwrap(),
                            sequence_buffer.binding().unwrap(),
                            vertices_buffer.binding().unwrap(),
                            round_buffer.binding().unwrap(),
                            &camera.2.0.default_view,
                            &camera.3.0.default_view,
                            id_buffer.binding().unwrap(),
                            camera.4.0.binding().unwrap(),
                        )),
                    )
                });

                batches.entry((*retained_view, *entity)).insert(LightBatch {
                    id: *entity,
                    range: index..index,
                });

                index += 1;

                light_buffer_meta.light_index_buffer.push(3);
            }
        }

        light_buffer_meta
            .light_index_buffer
            .write_buffer(&render_device, &render_queue);
    }
}

fn prepare_sprite_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    sprite_pipeline: Res<SpriteStencilPipeline>,
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

fn prepare_sprite_image_bind_groups_stencil(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut sprite_meta: ResMut<SpriteStencilMeta>,
    sprite_pipeline: Res<SpriteStencilPipeline>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    extracted_sprites: Res<ExtractedSprites>,
    extracted_slices: Res<ExtractedSlices>,
    mut phases: ResMut<ViewSortedRenderPhases<Stencil2d>>,
    events: Res<SpriteAssetEvents>,
    mut batches: ResMut<SpriteStencilBatches>,
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
                image_bind_groups.values.remove(&(*id, false));
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

                image_bind_groups
                    .values
                    .entry((batch_image_handle, false))
                    .or_insert_with(|| {
                        render_device.create_bind_group(
                            "sprite_material_bind_group",
                            &sprite_pipeline.material_layout,
                            &BindGroupEntries::sequential((
                                &gpu_image.texture_view,
                                &gpu_image.sampler,
                                is_dummy.binding().unwrap(),
                            )),
                        )
                    });

                batch_item_index = item_index;
                current_batch = Some(batches.entry((*retained_view, item.entity())).insert(
                    SpriteBatch {
                        image_handle_id: batch_image_handle,
                        normal_dummy: false,
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
                            extracted_sprite.id,
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
                                extracted_sprite.id,
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

fn prepare_sprite_image_bind_groups_normal(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut sprite_meta: ResMut<SpriteNormalMeta>,
    sprite_pipeline: Res<SpriteNormalMapsPipeline>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    extracted_sprites: Res<ExtractedSprites>,
    extracted_slices: Res<ExtractedSlices>,
    mut phases: ResMut<ViewSortedRenderPhases<NormalPhase>>,
    events: Res<SpriteAssetEvents>,
    mut batches: ResMut<SpriteNormalBatches>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added { .. } |
            // Images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Unused { id } | AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.remove(&(*id, false));
                image_bind_groups.values.remove(&(*id, true));
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

            let (normal_handle_id, is_dummy) = match extracted_sprite.normal_handle_id {
                None => (extracted_sprite.image_handle_id, 1),
                Some(id) => (id, 0),
            };

            let normal_dummy = match is_dummy {
                1 => true,
                _ => false,
            };

            let mut is_dummy = UniformBuffer::<u32>::from(is_dummy);
            is_dummy.write_buffer(&render_device, &render_queue);

            if batch_image_handle != normal_handle_id {
                let Some(gpu_image) = gpu_images.get(normal_handle_id) else {
                    continue;
                };
                batch_image_size = gpu_image.size_2d().as_vec2();
                batch_image_handle = normal_handle_id;

                image_bind_groups
                    .values
                    .entry((batch_image_handle, normal_dummy))
                    .or_insert_with(|| {
                        render_device.create_bind_group(
                            "sprite_material_bind_group",
                            &sprite_pipeline.material_layout,
                            &BindGroupEntries::sequential((
                                &gpu_image.texture_view,
                                &gpu_image.sampler,
                                is_dummy.binding().unwrap(),
                            )),
                        )
                    });

                batch_item_index = item_index;
                current_batch = Some(batches.entry((*retained_view, item.entity())).insert(
                    SpriteBatch {
                        image_handle_id: batch_image_handle,
                        normal_dummy,
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
                            extracted_sprite.id,
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
                                extracted_sprite.id,
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
