#define_import_path firefly::types


#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;

@group(0) @binding(1) var dt_lut_texture: texture_3d<f32>;

@group(0) @binding(2) var dt_lut_sampler: sampler;

struct PointLight {
    pos: vec2f,
    intensity: f32,
    range: f32,

    color: vec4f, 

    inner_range: f32,
    // 0 - inverse square, 1 - linear
    falloff: u32,
    falloff_intensity: f32,
    angle: f32,
    dir: vec2f, 
    z: f32,
    height: f32,
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

struct BinCounts {
    counts: array<u32, N_BINS>,
}