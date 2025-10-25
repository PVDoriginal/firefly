#define_import_path firefly::types

struct LightingData {
    n_occluders: u32,
}

struct PointLight {
    pos: vec2f,
}

struct OccluderMeta {
    n_vertices: u32,
    seam: f32,
    concave: u32,
    closed: u32,
}

struct Vertex {
    angle: f32,
    pos: vec2f
}