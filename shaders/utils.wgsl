#define_import_path firefly::utils

#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;

fn world_to_ndc(world_position: vec2<f32>) -> vec2<f32> {
    return (view.clip_from_world * vec4(world_position, 0.0, 1.0)).xy;
}

fn ndc_to_world(ndc_position: vec2<f32>) -> vec2<f32> {
    return (view.world_from_clip * vec4(ndc_position, 0.0, 1.0)).xy;
}

fn frag_coord_to_uv(frag_coord: vec2<f32>) -> vec2<f32> {
    return (frag_coord - view.viewport.xy) / view.viewport.zw;
}

fn frag_coord_to_ndc(frag_coord: vec2<f32>) -> vec2<f32> {
    return uv_to_ndc(frag_coord_to_uv(frag_coord.xy));
}

fn uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
}

fn ndc_to_uv(ndc: vec2<f32>) -> vec2<f32> {
    return ndc * vec2(0.5, -0.5) + vec2(0.5);
}

// checks if p and q are on the same side of the [a, b] segment
fn same_orientation(a: vec2f, b: vec2f, p: vec2f, q: vec2f) -> bool {
    let ori_p = orientation(a, b, p);
    let ori_q = orientation(a, b, q);

    return (ori_p == 0. && ori_q == 0.) || ori_p * ori_q > 0.; 
}

// = 0 - on
// > 0 - left
// < 0 - right  
fn orientation(a: vec2f, b: vec2f, p: vec2f) -> f32 {
    return (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
} 

fn ccw(a: vec2f, b: vec2f, c: vec2f) -> bool {
    return (c.y-a.y) * (b.x-a.x) > (b.y-a.y) * (c.x-a.x);
}

// Return true if line segments AB and CD intersect
fn intersect(a: vec2f, b: vec2f, c: vec2f, d: vec2f) -> bool {
    return ccw(a, c, d) != ccw(b, c, d) && ccw(a, b, c) != ccw(a, b, d);
}