#define_import_path firefly::types

struct PointLight {
    pos: vec2f,
    intensity: f32,
    range: f32,

    @size(16)
    color: vec3f, 

    inner_range: f32,
    // 0 - inverse square, 1 - linear
    falloff: u32,
    angle: f32,
    dir: vec2f, 
    z: f32,
    height: f32,
    n_rounds: u32,
    n_poly: u32,
}

struct PolyOccluder {
    vertex_start: u32,
    n_vertices: u32,
    z: f32,
    @size(16)
    color: vec3f, 
    opacity: f32,
    z_sorting: u32,
}

struct OccluderPointer {
    index: u32, 
    min_v: u32,
    length: u32, 
    distance: f32,
}

struct RoundOccluder {
    pos: vec2f,
    rot: f32,
    width: f32,
    height: f32, 
    radius: f32,
    z: f32, 
    @size(16)
    color: vec3f,
    opacity: f32, 
    z_sorting: u32, 
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

// Should correspond to the values in buffers.rs!
const N_BINS: u32 = 128;
const N_OCCLUDERS: u32 = 32; 

struct Bin {
    occluders: array<OccluderPointer, N_OCCLUDERS>,
    n_occluders: u32,
}
