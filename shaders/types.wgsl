#define_import_path firefly::types

struct LightingData {
    n_occluders: u32,
}

struct PointLight {
    pos: vec2f,
    light: LightColor,
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

struct LightColor {
    color: vec4f,
    intensity: f32,
}

struct FireflyConfig {
    global_light: LightColor,
    light_bands: u32,
}