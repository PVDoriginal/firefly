#define_import_path firefly::types

struct PointLight {
    pos: vec2f,
    color: vec3f, 
    intensity: f32,
    range: f32,
    inner_range: f32,
    // 0 - inverse square, 1 - linear
    falloff: u32,
    angle: f32,
    dir: vec2f,
    z: f32,
    height: f32,
}

struct Occluder {
    n_sequences: u32,
    n_vertices: u32,
    round: u32,
    n_sprites: u32,
    z: f32,
    color: vec3f, 
    opacity: f32,
    z_sorting: u32,
}

struct RoundOccluder {
    pos: vec2f,
    rot: f32,
    width: f32,
    height: f32, 
    radius: f32,
}

struct Vertex {
    angle: f32,
    pos: vec2f
}

struct FireflyConfig {
    ambient_color: vec3f,
    ambient_brightness: f32, 
    light_bands: u32,
    softness: f32,
    z_sorting: u32,
    // 0 - none, 1 - simple, 2 - topdown
    normal_mode: u32, 
    normal_attenuation: f32,
}

struct SpriteId {
    id: f32
}