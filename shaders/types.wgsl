#define_import_path firefly::types

struct PointLight {
    pos: vec2f,
    color: vec3f, 
    intensity: f32,
    range: f32,
    z: f32,
}

struct Occluder {
    n_vertices: u32,
    seam: f32,
    concave: u32,
    line: u32,
    round: u32,
    n_sprites: u32,
    z: f32,
    color: vec3f, 
    opacity: f32,
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
}

struct SpriteId {
    id: f32
}