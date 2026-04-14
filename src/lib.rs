//! **Firefly** is an open-source **2d lighting** crate made for the [Bevy](https://bevy.org/) game engine.
//!
//! Feel free to [create an issue](https://github.com/PVDoriginal/firefly/issues) if you want to request a specific feature or report a bug.
//!
//! # Example
//! Here is a basic example of implementing Firefly into a Bevy game.
//!
//! ```
//! use bevy::prelude::*;
//! use bevy_firefly::prelude::*;
//!
//! fn main() {
//!     App:new()
//!         // add FireflyPlugin to your app
//!         .add_plugins((DefaultPlugins, FireflyPlugin))
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn((
//!         Camera2d,
//!         // make sure to also have the FireflyConfig component on your camera
//!         FireflyConfig::default()
//!     ));
//!     
//!     // spawn a simple red light
//!     commands.spawn((
//!         PointLight2d {
//!             color: Color::srgb(1.0, 0.0, 0.0),
//!             range: 100.0,
//!             ..default()
//!         },
//!     ));
//!     
//!     // spawn a circle occluder
//!     commands.spawn((
//!         Occluder2d::circle(10.0),
//!         Transform::from_translation(vec3(0.0, 50.0, 0.0)),
//!     ));
//! }
//! ```
//!
//! # Occluders
//!
//! [Occluders](crate::occluders::Occluder2d) are shapes that block light and cast shadows.
//!
//! Current supported shapes include:
//! - [Polylines](crate::occluders::Occluder2d::polyline).
//! - [Polygons](crate::occluders::Occluder2d::polygon) (concave and convex).
//! - Round shapes such as [circles](crate::occluders::Occluder2d::circle), [capsules](crate::occluders::Occluder2d::capsule), [round rectangles](crate::occluders::Occluder2d::round_rectangle).
//!
//! Occluders have an [opacity](crate::occluders::Occluder2d::opacity), ranging from transprent to fully opaque, and can cast [colored shadows](crate::occluders::Occluder2d::opacity).   
//!
//! Occluders can be moved and rotated via the [Transform] component.   
//!
//! # Lights
//!
//! You can create lights by spawning entities with the [PointLight2d](crate::prelude::PointLight2d) component.
//!
//! Lights have adjustable [range](crate::prelude::PointLight2d::range), [falloff mode](crate::prelude::PointLight2d::falloff) and a variety of other features.
//!
//! # Features
//!
//! Here are some of the main features currently implemented :
//!
//! - **Soft Shadows**:
//! [FireflyConfig](crate::prelude::FireflyConfig) has a [Softness](crate::prelude::FireflyConfig::softness) field
//! that can be adjusted to disable / enable soft shadows, as well as give it a value (0 to 1) to set how soft the shadows should be.
//!
//! - **Occlusion Z-Sorting**: You can enable [z-sorting](crate::prelude::FireflyConfig::z_sorting) on [FireflyConfig](crate::prelude::FireflyConfig) to have shadows
//! only render over sprites with a lower z position than the occluder that cast them. This is extremely useful for certain 2d games, such as top-down games.
//!
//! - **Normal maps**: You can enable normal maps by changing the [normal mode](crate::prelude::FireflyConfig::normal_mode) field. You can then
//! add the [NormalMap](crate::prelude::NormalMap) component to sprites. Normal maps need to have the same exact layout as their entity's sprite image.
//! If [normal mode](crate::prelude::FireflyConfig::normal_mode) is set to [top down](crate::prelude::NormalMode::TopDown),
//! you can use [LightHeight](crate::prelude::LightHeight) and [SpriteHeight](crate::prelude::SpriteHeight) to emulate 3d dimensions for the normal maps.  
//!
//! - **Light Banding**: You can enable [light bands](crate::prelude::FireflyConfig::light_bands) on [FireflyConfig](crate::prelude::FireflyConfig) to
//! reduce the lightmap to a certain number of 'bands', creating a stylized look.
//!
//! # Upcoming Features
//!
//! Here are some of the features that are currently planned:
//! - Multiple lightmaps.
//! - Sprite-based shadows.
//! - Light textures.

use bevy::{
    prelude::*,
    render::{render_graph::RenderLabel, texture::CachedTexture},
};

pub mod app;
pub mod buffers;
pub mod change;
pub mod data;
pub mod lights;
pub mod occluders;
pub mod visibility;

pub mod extract;
pub mod nodes;
pub mod phases;
pub mod pipelines;
pub mod prepare;
pub mod sprites;

mod utils;

pub(crate) use phases::*;

pub mod prelude {
    pub use crate::app::{FireflyGizmosPlugin, FireflyPlugin};
    pub use crate::data::{FireflyConfig, NormalMode};
    pub use crate::lights::{Falloff, LightHeight, PointLight2d};
    pub use crate::occluders::Occluder2d;
    pub use crate::sprites::{NormalMap, SpriteHeight};
    pub use crate::{ApplyLightmapLabel, CreateLightmapLabel};
}

/// Camera component that stores the texture of the lightmap.
#[derive(Component)]
pub struct LightMapTexture(pub CachedTexture);

/// Camera component that stores the sprite stencil.
#[derive(Component)]
pub struct SpriteStencilTexture(pub CachedTexture);

/// Camera component that stores the normal map texture.  
#[derive(Component)]
pub struct NormalMapTexture(pub CachedTexture);

/// Render graph label for creating the lightmap.
///
/// Useful if you want to add your own render passes before / after it.   
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct CreateLightmapLabel;

/// Render graph label for when the lightmap is applied over the view texture and fed to the camera.
///
/// Useful if you want to add your own render passes before / after it.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ApplyLightmapLabel;

/// Render graph label for when the normal maps and sprite stencils are created.
///
/// Useful if you want to add your own render passes before / after it.
#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct SpriteLabel;
