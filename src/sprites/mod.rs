use crate::sprites::pipeline::{SpritePipeline, SpritesPipelinePlugin};
use crate::{CreateLightmapLabel, RenderLabel};

use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::{
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    ecs::{
        prelude::*,
        query::QueryItem,
        system::lifetimeless::{Read, Write},
    },
    prelude::*,
    render::{
        RenderApp,
        camera::ExtractedCamera,
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner,
        },
        render_phase::ViewSortedRenderPhases,
        render_resource::*,
        renderer::RenderContext,
        sync_world::SyncToRenderWorld,
        texture::CachedTexture,
        view::{ExtractedView, ViewTarget},
    },
};

mod phase;
mod pipeline;
mod texture_slice;

use phase::*;

#[derive(Component, Clone, Copy, ExtractComponent)]
#[require(SyncToRenderWorld)]
pub(crate) struct SpriteId(pub f32);

#[derive(Component, Clone, Copy, ShaderType)]
pub(crate) struct UniformSpriteId {
    id: f32,
}

impl SpriteId {
    pub(crate) fn float(&self) -> f32 {
        self.0
    }

    pub(crate) fn uniform(&self) -> UniformSpriteId {
        UniformSpriteId { id: self.float() }
    }
}

#[derive(Resource, Clone, Default)]
struct SpriteIdCounter(f32);

impl SpriteIdCounter {
    fn next(&mut self) -> f32 {
        self.0 += f32::EPSILON;
        return self.0;
    }
}

#[derive(Component)]
pub(crate) struct SpriteStencilTexture(pub CachedTexture);

pub(crate) struct SpritesPlugin;
impl Plugin for SpritesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SpritesPipelinePlugin);
        app.init_resource::<SpriteIdCounter>();

        app.add_systems(Update, increment_sprites);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_render_graph_node::<ViewNodeRunner<SpriteStencilNode>>(
                Core2d,
                SpriteStencilLabel,
            );
            // .add_render_graph_edges(Core2d, (Node2d::MainTransparentPass, SpriteStencilLabel));
        }
    }
}

// TODO: FIX THIS!
fn increment_sprites(
    sprites: Query<(Entity, &Name), (With<Sprite>, Without<SpriteId>)>,
    mut counter: ResMut<SpriteIdCounter>,
    mut commands: Commands,
) {
    for (sprite, name) in &sprites {
        let id = counter.next();
        info!("giving id {id} to {name}");
        commands.entity(sprite).insert(SpriteId(id));
    }
}

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct SpriteStencilLabel;

#[derive(Default)]
struct SpriteStencilNode;
impl ViewNode for SpriteStencilNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
        Read<SpriteStencilTexture>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, view, target, stencil_texture): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // First, we need to get our phases resource
        let Some(stencil_phases) = world.get_resource::<ViewSortedRenderPhases<Stencil2d>>() else {
            return Ok(());
        };

        // Get the view entity from the graph
        let view_entity = graph.view_entity();

        // Get the phase for the current view running our node
        let Some(stencil_phase) = stencil_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        // Render pass setup
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("stencil pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &stencil_texture.0.default_view,
                resolve_target: None,
                ops: default(),
            })],
            // For the purpose of the example, we will write directly to the view target. A real
            // stencil pass would write to a custom texture and that texture would be used in later
            // passes to render custom effects using it.
            // color_attachments: &[Some(target.get_color_attachment())],
            // We don't bind any depth buffer for this pass
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // if let Some(viewport) = camera.viewport.as_ref() {
        //     render_pass.set_camera_viewport(viewport);
        // }

        // Render the phase
        // This will execute each draw functions of each phase items queued in this phase
        if let Err(err) = stencil_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the stencil phase {err:?}");
        }

        // render_pass.draw(0..3, 0..1);
        Ok(())
    }
}
