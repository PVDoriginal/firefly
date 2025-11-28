use bevy::{
    platform::collections::HashSet,
    prelude::*,
    render::{
        Extract, RenderApp, extract_component::ExtractComponentPlugin,
        render_phase::ViewSortedRenderPhases, sync_world::RenderEntity, view::RetainedViewEntity,
    },
    sprite::SpriteSystem,
};

use crate::{
    data::{ExtractedWorldData, FireflyConfig},
    lights::{ExtractedPointLight, PointLight2d},
    occluders::ExtractedOccluder,
    phases::{NormalPhase, Stencil2d},
    prelude::Occluder2d,
    sprites::{
        ExtractedSlices, ExtractedSprite, ExtractedSpriteKind, ExtractedSprites, NormalMap,
        SpriteAssetEvents,
    },
};

pub(crate) struct ExtractPlugin;
impl Plugin for ExtractPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<FireflyConfig>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            ExtractSchedule,
            (
                extract_camera_phases,
                extract_sprites.in_set(SpriteSystem::ExtractSprites),
                extract_sprite_events,
                extract_world_data,
                extract_lights_occluders,
            ),
        );
    }
}

fn extract_camera_phases(
    mut stencil_phases: ResMut<ViewSortedRenderPhases<Stencil2d>>,
    mut normal_phases: ResMut<ViewSortedRenderPhases<NormalPhase>>,
    cameras: Extract<Query<(Entity, &Camera), With<Camera2d>>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();
    for (main_entity, camera) in &cameras {
        if !camera.is_active {
            continue;
        }
        // This is the main camera, so we use the first subview index (0)
        let retained_view_entity = RetainedViewEntity::new(main_entity.into(), None, 0);

        stencil_phases.insert_or_clear(retained_view_entity);
        normal_phases.insert_or_clear(retained_view_entity);

        live_entities.insert(retained_view_entity);
    }

    // Clear out all dead views.
    stencil_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
    normal_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

pub fn extract_sprite_events(
    mut events: ResMut<SpriteAssetEvents>,
    mut image_events: Extract<EventReader<AssetEvent<Image>>>,
) {
    let SpriteAssetEvents { ref mut images } = *events;
    images.clear();

    for event in image_events.read() {
        images.push(*event);
    }
}

pub fn extract_sprites(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    mut extracted_slices: ResMut<ExtractedSlices>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    sprite_query: Extract<
        Query<(
            Entity,
            RenderEntity,
            &ViewVisibility,
            &Sprite,
            Option<&NormalMap>,
            &GlobalTransform,
            Option<&super::utils::ComputedTextureSlices>,
        )>,
    >,
) {
    let mut id_counter = 0.;
    extracted_sprites.sprites.clear();
    extracted_slices.slices.clear();
    for (main_entity, render_entity, view_visibility, sprite, normal_map, transform, slices) in
        sprite_query.iter()
    {
        if !view_visibility.get() {
            continue;
        }

        id_counter += f32::EPSILON;

        if let Some(slices) = slices {
            let start = extracted_slices.slices.len();
            extracted_slices
                .slices
                .extend(slices.extract_slices(sprite));
            let end = extracted_slices.slices.len();
            extracted_sprites.sprites.push(ExtractedSprite {
                main_entity,
                render_entity,

                transform: *transform,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                image_handle_id: sprite.image.id(),
                normal_handle_id: normal_map.and_then(|x| Some(x.handle().id())),
                kind: ExtractedSpriteKind::Slices {
                    indices: start..end,
                },
                id: id_counter,
            });
        } else {
            let atlas_rect = sprite
                .texture_atlas
                .as_ref()
                .and_then(|s| s.texture_rect(&texture_atlases).map(|r| r.as_rect()));
            let rect = match (atlas_rect, sprite.rect) {
                (None, None) => None,
                (None, Some(sprite_rect)) => Some(sprite_rect),
                (Some(atlas_rect), None) => Some(atlas_rect),
                (Some(atlas_rect), Some(mut sprite_rect)) => {
                    sprite_rect.min += atlas_rect.min;
                    sprite_rect.max += atlas_rect.min;
                    Some(sprite_rect)
                }
            };

            // PERF: we don't check in this function that the `Image` asset is ready, since it should be in most cases and hashing the handle is expensive
            extracted_sprites.sprites.push(ExtractedSprite {
                main_entity,
                render_entity,
                transform: *transform,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                image_handle_id: sprite.image.id(),
                normal_handle_id: normal_map.and_then(|x| Some(x.handle().id())),
                kind: ExtractedSpriteKind::Single {
                    anchor: sprite.anchor.as_vec(),
                    rect,
                    scaling_mode: sprite.image_mode.scale(),
                    // Pass the custom size
                    custom_size: sprite.custom_size,
                },
                id: id_counter,
            });
        }
    }
}

fn extract_world_data(
    mut commands: Commands,
    camera: Extract<Query<(&RenderEntity, &GlobalTransform, &FireflyConfig, &Camera2d)>>,
) {
    for (entity, transform, _, _) in &camera {
        commands.entity(entity.id()).insert(ExtractedWorldData {
            camera_pos: transform.translation().truncate(),
        });
    }
}

fn extract_lights_occluders(
    mut commands: Commands,
    camera: Extract<Single<(&GlobalTransform, &Projection), With<FireflyConfig>>>,
    lights: Extract<Query<(&RenderEntity, &GlobalTransform, &PointLight2d)>>,
    occluders: Extract<Query<(&RenderEntity, &Occluder2d, &GlobalTransform)>>,
) {
    let Projection::Orthographic(projection) = camera.1 else {
        return;
    };

    let camera_rect = Rect {
        min: projection.area.min + camera.0.translation().truncate(),
        max: projection.area.max + camera.0.translation().truncate(),
    };

    let mut light_rect = Rect::default();
    for (entity, transform, light) in &lights {
        let pos = transform.translation().truncate() - vec2(0.0, light.height);

        if (Rect {
            min: pos - light.range,
            max: pos + light.range,
        })
        .intersect(camera_rect)
        .is_empty()
        {
            continue;
        }

        commands.entity(entity.id()).insert(ExtractedPointLight {
            pos: pos,
            color: light.color,
            intensity: light.intensity,
            range: light.range,
            z: transform.translation().z,
            inner_range: light.inner_range,
            falloff: light.falloff,
            angle: light.angle,
            cast_shadows: light.cast_shadows,
            dir: (transform.rotation() * Vec3::Y).xy(),
            height: light.height,
        });

        light_rect = light_rect.union(camera_rect.union_point(pos).intersect(Rect {
            min: pos - light.range,
            max: pos + light.range,
        }));
    }

    for (render_entity, occluder, global_transform) in &occluders {
        let mut rect = occluder.rect();
        rect.min += global_transform.translation().truncate();
        rect.max += global_transform.translation().truncate();

        if rect.intersect(light_rect).is_empty() {
            continue;
        }

        commands
            .entity(render_entity.id())
            .insert(ExtractedOccluder {
                pos: global_transform.translation().truncate(),
                rot: global_transform.rotation().to_euler(EulerRot::XYZ).2,
                shape: occluder.shape().clone(),
                rect,
                z: global_transform.translation().z,
                color: occluder.color,
                opacity: occluder.opacity,
                ignored_sprites: occluder.ignored_sprites.clone(),
                z_sorting: occluder.z_sorting,
                height: occluder.height,
            });
    }
}
