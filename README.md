# Firefly

[![Discord](https://img.shields.io/discord/805147867924267018?logo=discord&color=7289DA)](https://discord.com/channels/691052431525675048/1447681362722033816)
[![crates.io](https://img.shields.io/crates/v/bevy_firefly)](https://crates.io/crates/bevy_firefly)
[![docs](https://docs.rs/bevy_firefly/badge.svg)](https://docs.rs/bevy_firefly/)
[![downloads](https://img.shields.io/crates/d/bevy_firefly)](https://crates.io/crates/bevy_firefly)

[Firefly](https://crates.io/crates/bevy_firefly) is an open-source, **2d lighting** crate for the [bevy game engine](https://bevy.org/).

I am working on it as part of my college thesis. It uses certain geometrical and computational algorithms to take advantage of Bevy's ECS and put less strain on the GPU. 

My final goal is to have a lighting crate with features akin to the ones of more mature game engines, while 
keeping performance as high as possible and minimizing the end-user API.

Firefly is still pretty early in development, with many features to be added still. However, there is already lot of cool stuff for you to play with 
or use for your games, if you wish to. 

Short video showing off soft shadows and z-sorting: 

https://github.com/user-attachments/assets/1984ef2a-0edd-4a40-93cb-a901057a9b74

> Credit for the characters and assets to [Kimberly](https://github.com/Kaircha) and her upcoming game!

Here's the same game but with light banding and hard shadows:

https://github.com/user-attachments/assets/6118f75e-b797-41bb-998e-381dc9d84cb9

> Credit for the characters and assets to [Kimberly](https://github.com/Kaircha) and her upcoming game!

Here's a video of the [crates example](https://github.com/PVDoriginal/firefly/blob/main/examples/shapes.rs), showcasing normal maps and z-sorting:

https://github.com/user-attachments/assets/fd9453ba-e42a-4155-b96b-889bfdceea48

And here is a video of the [stress example](https://github.com/PVDoriginal/firefly/blob/main/examples/stress.rs).

https://github.com/user-attachments/assets/c9b8c716-a0c4-4604-8fbb-50d6bbbe8aad

## Usage 
To use this crate, simply run `cargo add bevy_firefly` or add firefly to your Cargo.toml file. 

You can see all the firely versions [here](https://crates.io/crates/bevy_firefly/versions). 

Here is a basic example of integrating firefly into a bevy app: 

```Rs
use bevy::prelude::*;
use bevy_firefly::prelude::*;

fn main() {
  App:new()
    .add_plugins((DefaultPlugins, FireflyPlugin))
    .add_systems(Startup, setup)
    .run();
}

fn setup(mut commands: Commands) {
  commands.spawn((
    Camera2d,
    FireflyConfig::default()
  ));
     
  commands.spawn((
    PointLight2d {
      color: Color::srgb(1.0, 0.0, 0.0),
      range: 100.0,
      ..default()
    },
    Transform::default()
  ));
     
  commands.spawn((
    Occluder2d::circle(10.0),
    Transform::from_translation(vec3(0.0, 50.0, 0.0)),
  ));
}
```
Check out the [examples](examples/) and the [crate documentation](https://docs.rs/bevy_firefly/) to learn more about using it.

## Features 
Some of the existing features are: 
  - Dynamic lights and occluders
  - Point lights
  - Round and polygonal occluders
  - Soft shadows
  - Occlusion z-sorting
  - Normal maps
  - Occlusion sprite masking
  - Transparent & colored occluders
  - Light banding

Some of the currently planned features are: 
  - Occluders casting sprite-based shadows
  - Mulitple lightmaps
  - Light textures

Feel free to open an issue if you want to request any specific features or report any bugs!

Also you can ask any questions over on [discord](https://discord.com/channels/691052431525675048/1447681362722033816)! 

## Bevy Compatibility 
| bevy | bevy_firefly  |
|------|---------------|
| 0.16 | 0.16          |
| 0.17 | 0.17          |

## Alternatives
You can check out [bevy_light_2d](https://github.com/jgayfer/bevy_light_2d) and [bevy_lit](https://github.com/malbernaz/bevy_lit). They were both a big inspiration when starting out with this crate! 
