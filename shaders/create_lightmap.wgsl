#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

#import firefly::types::{PointLight, LightingData, Occluder, Vertex, RoundOccluder}
#import firefly::utils::{ndc_to_world, frag_coord_to_ndc, orientation, same_orientation, intersect, blend, intersects_arc}

@group(0) @binding(0)
var<uniform> view: View;

@group(0) @binding(1)
var lightmap_texture: texture_2d<f32>;

@group(0) @binding(2)
var texture_sampler: sampler;

@group(0) @binding(3) 
var<uniform> data: LightingData;

@group(0) @binding(4)
var<uniform> light: PointLight;

@group(0) @binding(5)
var<storage> occluders: array<Occluder>;

@group(0) @binding(6)
var<storage> vertices: array<Vertex>;

@group(0) @binding(7)
var<storage> round_occluders: array<RoundOccluder>;

@group(0) @binding(8)
var sprite_stencil: texture_2d<f32>;

const PI2: f32 = 6.28318530718;
const PI: f32 = 3.14159265359;
const PIDIV2: f32 = 1.57079632679; 

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let pos = ndc_to_world(frag_coord_to_ndc(in.position.xy));
    var res = max(textureSample(lightmap_texture, texture_sampler, in.uv), vec4f(0, 0, 0, 1));
    let stencil = textureLoad(sprite_stencil, vec2<i32>(in.uv * vec2<f32>(textureDimensions(sprite_stencil))), 0);
    
    let dist = distance(pos, light.pos);
    if (dist < light.range) {
        let x = dist / light.range;
        res = blend(res, vec4f(light.color, 0), light.intensity * (1. - x * x));

        var round_index = 0u;

        var start_vertex = 0u;
        for (var i = 0u; i < data.n_occluders; i++) {
            var shadow = vec4f(0, 0, 0, 0); 

            if (occluders[i].round == 1) {
                if (round_check(pos, round_index)) {
                    shadow = vec4f(1, 1, 1, 0);
                }
                round_index += 1;
            }
            else if (occluders[i].concave == 1) {
                let intersections = concave_check(pos, i, start_vertex);
                if (intersections > 0) {
                    shadow = vec4f(1, 1, 1, 0);
                }
            }
            else if (is_occluded(pos, i, start_vertex)) {
                shadow = vec4f(1, 1, 1, 0);
            }
            start_vertex += occluders[i].n_vertices;
            
            if (stencil.a > 0.1) {
                if (stencil.g >= occluders[i].z) {
                    continue;
                }

                if (stencil.r == occluders[i].sprite_id) {
                    continue;
                }
            }

            res -= shadow;
        }
    }
    return res;
}

fn is_occluded(pos: vec2f, occluder: u32, start_vertex: u32) -> bool {
    let raw_angle = atan2(pos.y - light.pos.y, pos.x - light.pos.x);

    let angle = (raw_angle - occluders[occluder].seam) + PI2 * floor((occluders[occluder].seam - raw_angle) / PI2);

    let maybe_prev = bs_vertex(angle, start_vertex, occluders[occluder].n_vertices);

    if maybe_prev == -1 {
        return false;
    }

    let prev = u32(maybe_prev);

    if prev + 1 >= occluders[occluder].n_vertices  {
        return false;
    }
    
    return !same_orientation(vertices[start_vertex + prev].pos, vertices[start_vertex + prev + 1].pos, pos, light.pos);
}

fn bs_vertex(angle: f32, offset: u32, size: u32) -> i32 {
    var ans = -1;
    
    var low = 0i; 
    var high = i32(size) - 1;

    while (low <= high) {
        let mid = low + (high - low + 1) / 2;
        let val = vertices[u32(mid) + offset].angle;

        if (val  < angle) {
            ans = i32(mid);
            low = mid + 1;
        }
        else {
            high = mid - 1;
        }
    }

    return ans;
}

// returns number of times pixel was blocked by occluder
fn concave_check(pos: vec2f, occluder: u32, start_vertex: u32) -> u32 {
    var intersections = 0u;

    for (var i = start_vertex; i < start_vertex + occluders[occluder].n_vertices - 1; i++) {
        if (intersect(vertices[i].pos, vertices[i+1].pos, pos, light.pos)) {
            intersections += 1;
        }
    }

    if (occluders[occluder].line == 0 && intersect(vertices[start_vertex].pos, vertices[start_vertex + occluders[occluder].n_vertices - 1].pos, pos, light.pos)) {
        intersections += 1;
    }

    if (occluders[occluder].line == 1) {
        return intersections;
    }
    
    return (intersections + 1) / 2;
}

// checks if pixel is blocked by round occluder
fn round_check(pos: vec2f, occluder: u32) -> bool {
    let center = round_occluders[occluder].pos;
    let width = round_occluders[occluder].width / 2; 
    let height = round_occluders[occluder].height / 2; 
    let radius = round_occluders[occluder].radius;

    // top edge
    if (intersect(center + vec2f(-width, height + radius), center + vec2f(width, height + radius), pos, light.pos)) {
        return true;
    }

    // right edge
    if (intersect(center + vec2f(width + radius, height), center + vec2f(width + radius, -height), pos, light.pos)) {
        return true;
    }
    
    // bottom edge
    if (intersect(center + vec2f(-width, -height - radius), center + vec2f(width, -height - radius), pos, light.pos)) {
        return true;
    }
    
    // left edge
    if (intersect(center + vec2f(-width - radius, height), center + vec2f(-width - radius, -height), pos, light.pos)) {
        return true;
    }

    // top-left arc
    if (intersects_arc(pos, light.pos, center + vec2f(-width, height), radius, PIDIV2, PI)) {
        return true;
    }
    
    // top-right arc
    if (intersects_arc(pos, light.pos, center + vec2f(width, height), radius, 0, PIDIV2)) {
        return true;
    }
    
    // bottom-right arc
    if (intersects_arc(pos, light.pos, center + vec2f(width, -height), radius, -PIDIV2, 0)) {
        return true;
    }
    
    // bottom-left arc
    if (intersects_arc(pos, light.pos, center + vec2f(-width, -height), radius, -PI, -PIDIV2)) {
        return true;
    }

    return false;
}