#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

#import firefly::types::{PointLight, LightingData, OccluderMeta, Vertex}
#import firefly::utils::{ndc_to_world, frag_coord_to_ndc, orientation, same_orientation, intersect}

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
var<storage> occluders: array<OccluderMeta>;

@group(0) @binding(6)
var<storage> vertices: array<Vertex>;

const PI2: f32 = 6.28318530718;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let pos = ndc_to_world(frag_coord_to_ndc(in.position.xy));
    var res = vec4f(0, 0, 0, 1);

    let light = light.pos; 
    let dist = distance(pos, light) + 0.01; 

    if (dist < 200.) {
        let x = dist / 200.;
        res += vec4f(vec3f(1. - x * x), 0);
        
        var start_vertex = 0u;
        for (var i = 0u; i < data.n_occluders; i++) {

            if (occluders[i].concave == 1) {
                let intersections = concave_check(pos, i, start_vertex);
                if (intersections > 0) {
                    res = vec4f(0, 0, 0, 1);
                    break;
                }
            }
            else if (is_occluded(pos, i, start_vertex)) {
                res = vec4f(0, 0, 0, 1);
                break;
            }
            start_vertex += occluders[i].n_vertices;
        }
    }
    res = max(res, vec4f(0, 0, 0, 1));

    res += textureSample(lightmap_texture, texture_sampler, in.uv);
    return min(res, vec4f(1, 1, 1, 1));
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
fn concave_check(pos: vec2f, occluder: u32, start_vertex: u32) -> i32 {
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
        return i32(intersections);
    }

    if (intersections % 2 == 0) {
        return i32(intersections) / 2;
    }
    
    return 0;
}