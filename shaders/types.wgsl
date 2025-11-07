#define_import_path firefly::types

struct LightingData {
    n_occluders: u32,
}

struct PointLight {
    pos: vec2f,
    color: vec3f, 
    intensity: f32,
    range: f32,
}

struct UniformOccluder {
    n_vertices: u32,
    seam: f32,
    concave: u32,
    line: u32,
    hollow: u32,
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