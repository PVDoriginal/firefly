# firefly

[Firefly](https://crates.io/crates/bevy_firefly) is an open-source, **2d lighting** crate for the [bevy game engine](https://bevy.org/).

https://github.com/user-attachments/assets/1984ef2a-0edd-4a40-93cb-a901057a9b74

> Credit for the characters and assets to [Kimberly](https://github.com/Kaircha) and her upcoming game! 

## Goals 
The goal of firefly is to have a **fully 2d** lighting crate with features akin to the ones of more mature game engines, while 
keeping performance as high as possible and the minimizing the end-user API. 

## State 
Firefly is still pretty early in development, with many features to be added still. However, there are already lot of cool stuff for you to play with 
or use for your games, if you wish to. 

The crate is currently only available for Bevy 0.16. I'm planning to update it but am not currently working on it. You can open an issue if you want me to prioritize it.

## Features 
Some of the existing features are: 
  - Dynamic lights and occluders
  - Point lights
  - Round and polygonal occluders
  - Soft shadows
  - Occlusion z-sorting
  - Occlusion sprite masking
  - Transparent & colored occluders
  - Light banding

Some of the currently planned features are: 
  - Normal maps
  - Occluders casting sprite-based shadows
  - Mulitple lightmaps
  - Light textures

Feel free to open an issue if you want to request any specific features or report any bugs!  

## Usage 
To use this crate, simply run `cargo run bevy_firefly` or add firefly to your Cargo.toml file. 
Here is a basis example of integrating firefly into a bevy app: 

```Rs
use bevy::prelude::*;
use bevy_firefly::prelude::*;

fn main() {
  App:new()
    // add FireflyPlugin to your app
    .add_plugins((DefaultPlugins, FireflyPlugin))
    .add_systems(Startup, setup)
    .run();
}

fn setup(mut commands: Commands) {
  commands.spawn((
    Camera2d,
    // make sure to also have the FireflyConfig component on your camera
    FireflyConfig::default()
  ));
     
  // spawn a simple red light
  commands.spawn((
    PointLight2d {
      color: Color::srgb(1.0, 0.0, 0.0),
      range: 100.0,
      ..default()
    },
    Transform::default()
  ));
     
  // spawn a circle occluder
  commands.spawn((
    Occluder2d::circle(10.0),
    Transform::from_translation(vec3(0.0, 50.0, 0.0)),
  ));
}
```
Check out the [examples](examples/) and the [crate documentation](docs.rs/bevy_firefly/0.1.0) to learn more about using it.

## Bevy Compatibility 
| bevy | firefly |
|------|---------------|
| 0.16 | 0.1     |

## Alternatives
You can check out [bevy_light_2d](https://github.com/jgayfer/bevy_light_2d) and [bevy_lit](https://github.com/malbernaz/bevy_lit). They were both a big inspiration when starting out with this crate! 
