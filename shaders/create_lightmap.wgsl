#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

#import firefly::types::{PointLight, LightingData, Occluder, Vertex, RoundOccluder}
#import firefly::utils::{ndc_to_world, frag_coord_to_ndc, orientation, same_orientation, intersect, blend, shadow_blend, intersects_arc, rotate, rotate_arctan, between_arctan}

@group(0) @binding(0)
var<uniform> view: View;

@group(0) @binding(1)
var lightmap_texture: texture_2d<f32>;

@group(0) @binding(2)
var texture_sampler: sampler;

@group(0) @binding(3)
var<uniform> light: PointLight;

@group(0) @binding(4)
var<storage> occluders: array<Occluder>;

@group(0) @binding(5)
var<storage> sequences: array<u32>;

@group(0) @binding(6)
var<storage> vertices: array<Vertex>;

@group(0) @binding(7)
var<storage> round_occluders: array<RoundOccluder>;

@group(0) @binding(8)
var sprite_stencil: texture_2d<f32>;

@group(0) @binding(9)
var<storage> ids: array<f32>;

const PI2: f32 = 6.28318530718;
const PI: f32 = 3.14159265359;
const PIDIV2: f32 = 1.57079632679; 
const MAXANGLE: f32 = 0.7;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let pos = ndc_to_world(frag_coord_to_ndc(in.position.xy));
    var prev = max(textureSample(lightmap_texture, texture_sampler, in.uv), vec4f(0, 0, 0, 1));
    let stencil = textureLoad(sprite_stencil, vec2<i32>(in.uv * vec2<f32>(textureDimensions(sprite_stencil))), 0);

    let dist = distance(pos, light.pos);
    if (dist < light.range) {
        let x = dist / light.range;
        var res = min(vec4f(1), vec4f(light.color, 0) * light.intensity * (1. - x * x));

        var round_index = 0u;
        var start_vertex = 0u;
        var sequence_index = 0u;
        var id_index = 0u;

        var shadow = vec3f(1); 
        var i = 0u; 

        loop {
            if (i >= arrayLength(&occluders)) {
                break;
            }
            
            if (stencil.a > 0.1) {
                if (stencil.g >= occluders[i].z) {
                    continue;
                }

                if (is_excluded(i, id_index, stencil.r)) {
                    continue;
                }
            }

            if (occluders[i].round == 1) {
                let result = round_check(pos, round_index); 

                if result.occluded == true {
                    shadow = shadow_blend(shadow, occluders[i].color, occluders[i].opacity);
                }
                else {
                    if result.extreme_angle < MAXANGLE {
                        shadow = shadow_blend(shadow, occluders[i].color, occluders[i].opacity * (1f - (result.extreme_angle / MAXANGLE)));
                    }
                }
            }

            else {
                var start_vertex_2 = start_vertex;
                for (var s = sequence_index; s < sequence_index + occluders[i].n_sequences; s++) {
                    let result = is_occluded(pos, s, start_vertex_2); 

                    if result.occluded == true {    
                        shadow = shadow_blend(shadow, occluders[i].color, occluders[i].opacity);
                    }
                    else {
                        if result.extreme_angle < MAXANGLE {
                            shadow = shadow_blend(shadow, occluders[i].color, occluders[i].opacity * (1f - (result.extreme_angle / MAXANGLE)));
                        }
                    }

                    start_vertex_2 += sequences[s];
                }
            }

            continuing {
                for (var s = sequence_index; s < sequence_index + occluders[i].n_sequences; s++) {
                    start_vertex += sequences[s];
                }
                sequence_index += occluders[i].n_sequences;
                id_index += occluders[i].n_sprites;

                if (occluders[i].round == 1) {
                    round_index += 1;
                }
                i += 1;
            }
        }
        res *= vec4f(shadow, 1);
        prev = max(prev, res);
    }
    return prev;
}

fn is_excluded(occluder: u32, start_id: u32, id: f32) -> bool {
    for (var i = start_id; i < start_id + occluders[occluder].n_sprites; i++) {
        if (ids[i] == id) {
            return true;
        }
    }
    return false;
}

struct OcclusionResult {
    occluded: bool, 
    extreme_angle: f32,
}

fn get_extreme_angle(pos: vec2f, extreme: vec2f) -> f32 {
    let light_proj = (extreme - light.pos) + extreme;  
    
    let a = vec2f(extreme.x - pos.x, extreme.y - pos.y);
    let b = vec2f(extreme.x - light_proj.x, extreme.y - light_proj.y);
    let angle = acos(dot(a, b) / (length(a) * length(b)));
    
    return angle;
}

fn is_occluded(pos: vec2f, sequence: u32, start_vertex: u32) -> OcclusionResult {
    let angle = atan2(pos.y - light.pos.y, pos.x - light.pos.x);

    let maybe_prev = bs_vertex(angle, start_vertex, sequences[sequence]);

    if maybe_prev == -1 {
        return OcclusionResult(
            false, 
            min(
                get_extreme_angle(pos, vertices[start_vertex].pos),  
                get_extreme_angle(pos, vertices[start_vertex + sequences[sequence] - 1].pos)
            )
        );
    }

    let prev = u32(maybe_prev);

    if prev + 1 >= sequences[sequence]  {
        return OcclusionResult(
            false, 
            min(
                get_extreme_angle(pos, vertices[start_vertex].pos),  
                get_extreme_angle(pos, vertices[start_vertex + sequences[sequence] - 1].pos)
            )
        );
    }

    if same_orientation(vertices[start_vertex + prev].pos, vertices[start_vertex + prev + 1].pos, pos, light.pos) {
        return OcclusionResult(
            false, 
            min(
                get_extreme_angle(pos, vertices[start_vertex].pos),  
                get_extreme_angle(pos, vertices[start_vertex + sequences[sequence] - 1].pos)
            )
        );
    }

    return OcclusionResult(
        true, 
        0f,
    );
}

fn bs_vertex(angle: f32, offset: u32, size: u32) -> i32 {
    var ans = -1;
    
    var low = 0i; 
    var high = i32(size) - 1;

    if angle < vertices[u32(low) + offset].angle {
        return -1;
    }

    if angle >= vertices[u32(high) + offset].angle {
        return high + 1;
    }

    while (low <= high) {
        let mid = low + (high - low + 1) / 2;
        let val = vertices[u32(mid) + offset].angle;

        if (val < angle) {
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
// fn concave_check(pos: vec2f, occluder: u32, start_vertex: u32) -> u32 {
//     var intersections = 0u;

//     for (var i = start_vertex; i < start_vertex + occluders[occluder].n_vertices - 1; i++) {
//         if (intersect(vertices[i].pos, vertices[i+1].pos, pos, light.pos)) {
//             intersections += 1;
//         }
//     }

//     if (occluders[occluder].line == 0 && intersect(vertices[start_vertex].pos, vertices[start_vertex + occluders[occluder].n_vertices - 1].pos, pos, light.pos)) {
//         intersections += 1;
//     }

//     if (occluders[occluder].line == 1) {
//         return intersections;
//     }
    
//     return (intersections + 1) / 2;
// }

// checks if pixel is blocked by round occluder
fn round_check(pos: vec2f, occluder: u32) -> OcclusionResult {
    let center = round_occluders[occluder].pos;
    let width = round_occluders[occluder].width / 2; 
    let height = round_occluders[occluder].height / 2; 
    let radius = round_occluders[occluder].radius;

    var rot = round_occluders[occluder].rot;
    
    if (rot > PI2) {
        rot = rot - PI2 * floor(rot / PI2);
    }

    let cos_sin = vec2f(cos(rot), sin(rot));

    var extreme_angle = 10f;

    if (width > 0) {
        // top edge
        if (intersect(center + rotate(vec2f(-width, height + radius), cos_sin), center + rotate(vec2f(width, height + radius), cos_sin), pos, light.pos)) {
            return OcclusionResult(true, extreme_angle);
        }

        // bottom edge
        if (intersect(center + rotate(vec2f(-width, -height - radius), cos_sin), center + rotate(vec2f(width, -height - radius), cos_sin), pos, light.pos)) {
            return OcclusionResult(true, extreme_angle);
        }

        extreme_angle = min(
            extreme_angle, 
            min(
                min(get_extreme_angle(pos, center + rotate(vec2f(-width, height + radius), cos_sin)),  get_extreme_angle(pos, center + rotate(vec2f(width, height + radius), cos_sin))),
                min(get_extreme_angle(pos, center + rotate(vec2f(-width, -height - radius), cos_sin)), get_extreme_angle(pos, center + rotate(vec2f(width, -height - radius), cos_sin)))
            )
        );
    }

    if (height > 0) {
        // right edge
        if (intersect(center + rotate(vec2f(width + radius, height), cos_sin), center + rotate(vec2f(width + radius, -height), cos_sin), pos, light.pos)) {
            return OcclusionResult(true, extreme_angle);
        }
        
        // left edge
        if (intersect(center + rotate(vec2f(-width - radius, height), cos_sin), center + rotate(vec2f(-width - radius, -height), cos_sin), pos, light.pos)) {
            return OcclusionResult(true, extreme_angle);
        }

        
        extreme_angle = min(
            extreme_angle, 
            min(
                min(get_extreme_angle(pos, center + rotate(vec2f(width + radius, height), cos_sin)),  get_extreme_angle(pos, center + rotate(vec2f(width + radius, -height), cos_sin))),
                min(get_extreme_angle(pos, center + rotate(vec2f(-width - radius, height), cos_sin)), get_extreme_angle(pos, center + rotate(vec2f(-width - radius, -height), cos_sin)))
            )
        );
    }

    if (radius > 0) {
        // top-left arc
        if (intersects_arc(pos, light.pos, center + rotate(vec2f(-width, height), cos_sin), radius, rotate_arctan(PIDIV2, rot), rotate_arctan(PI, rot))) {
            return OcclusionResult(true, extreme_angle);
        }
        
        // top-right arc
        if (intersects_arc(pos, light.pos, center + rotate(vec2f(width, height), cos_sin), radius, rotate_arctan(0, rot), rotate_arctan(PIDIV2, rot))) {
            return OcclusionResult(true, extreme_angle);
        }
        
        // bottom-right arc
        if (intersects_arc(pos, light.pos, center + rotate(vec2f(width, -height), cos_sin), radius, rotate_arctan(-PIDIV2, rot), rotate_arctan(0, rot))) {
            return OcclusionResult(true, extreme_angle);
        }
        
        // bottom-left arc
        if (intersects_arc(pos, light.pos, center + rotate(vec2f(-width, -height), cos_sin), radius, rotate_arctan(-PI, rot), rotate_arctan(-PIDIV2, rot))) {
            return OcclusionResult(true, extreme_angle);
        }

        extreme_angle = min(
            extreme_angle, 
            min(
                min(
                    get_arc_extremes(pos, light.pos, center + rotate(vec2f(-width, height), cos_sin), radius, rotate_arctan(PIDIV2, rot), rotate_arctan(PI, rot)),  
                    get_arc_extremes(pos, light.pos, center + rotate(vec2f(width, height), cos_sin), radius, rotate_arctan(0, rot), rotate_arctan(PIDIV2, rot)),  
                ),
                min(
                    get_arc_extremes(pos, light.pos, center + rotate(vec2f(width, -height), cos_sin), radius, rotate_arctan(-PIDIV2, rot), rotate_arctan(0, rot)), 
                    get_arc_extremes(pos, light.pos, center + rotate(vec2f(-width, -height), cos_sin), radius, rotate_arctan(-PI, rot), rotate_arctan(-PIDIV2, rot)),  
                )
            )
        );

    }

    return OcclusionResult(false, extreme_angle);
}

fn get_arc_extremes(pos: vec2f, p: vec2f, c: vec2f, r: f32, start_angle: f32, end_angle: f32) -> f32 {
    let b = sqrt((p.x - c.x) * (p.x - c.x) + (p.y - c.y) * (p.y - c.y));
    let th = acos(r / b);
    let d = atan2(p.y - c.y, p.x - c.x);
    let d1 = d + th;
    let d2 = d - th;

    let t1 = vec2f(c.x + r * cos(d1), c.y + r * sin(d1));
    let t2 = vec2f(c.x + r * cos(d2), c.y + r * sin(d2));

    let a1 = atan2(t1.y - c.y, t1.x - c.x);
    let a2 = atan2(t2.y - c.y, t2.x - c.x);

    var res = 10f;

    if (between_arctan(a1, start_angle, end_angle)) {
        res = min(res, get_extreme_angle(pos, t1));
    }

    if (between_arctan(a2, start_angle, end_angle)) {
        res = min(res, get_extreme_angle(pos, t2));
    }

    return res;
}