#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

#import firefly::types::{PointLight, LightingData, Occluder, Vertex, RoundOccluder, FireflyConfig}
#import firefly::utils::{
    ndc_to_world, frag_coord_to_ndc, orientation, same_orientation, intersect, blend, 
    shadow_blend, intersects_arc, rotate, rotate_arctan, between_arctan, distance_point_to_line,
    intersection_point
}

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
var normal_map: texture_2d<f32>;

@group(0) @binding(10)
var<storage> ids: array<f32>;

@group(0) @binding(11)
var<uniform> config: FireflyConfig;

const PI2: f32 = 6.28318530718;
const PI: f32 = 3.14159265359;
const PIDIV2: f32 = 1.57079632679; 

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    return vec4f(1);
    // let pos = ndc_to_world(frag_coord_to_ndc(in.position.xy));
    // var prev = max(textureSample(lightmap_texture, texture_sampler, in.uv), vec4f(0, 0, 0, 1));
    // // let normal = textureSample(normal_map, texture_sampler, in.uv);
    // let stencil = textureLoad(sprite_stencil, vec2<i32>(in.uv * vec2<f32>(textureDimensions(sprite_stencil))), 0);
    // let normal = textureLoad(normal_map, vec2<i32>(in.uv * vec2<f32>(textureDimensions(normal_map))), 0);


    // // if normal.a > 0 {
    // //     return vec4f(0);
    // // }

    // // var light_dir = vec3f(0f, 1f, 0f);


    // // if normal.a > 0 && normal.r < 0.4 {
    // //     return vec4f(0, 1, 0, 1);
    // // }

    // // if normal.a > 0 {
    // //     if (normal * 2f - 1f).r != 0 {
    // //         return vec4f(1, 0, 0, 1);
    // //     }
    // // }

    // // if normal.a > 0 {
    // //     return normal;
    // //     // return vec4f(normalize(light_dir) * 0.5 + 0.5, 1f);
    // // }

    // let soft_angle = config.softness; 

    // let dist = distance(pos, light.pos);
    
    // let a = pos - light.pos;
    // let b = light.dir;
    // let angle = acos(dot(a, b) / (length(a) * length(b)));
    
    // if (dist < light.range && angle <= light.angle / 2.) {
    //     var res = vec4f(0);

    //     var normal_multi = 1f;

    //     // var light_dir = vec3(light.pos - ((in.position.xy - view.viewport.xy) / view.viewport.zw), 1f);
    //     // light_dir.x *= view.viewport.z / view.viewport.w;

    //     if normal.a > 0 {
    //         if normal.b == 0f {
    //             normal_multi = 0f;
    //         }
    //         else {
    //             // let normal_red = mix(normal, vec4f(0), 0.8);
    //             let light_dir = normalize(vec3f(light.pos.x - pos.x, pos.y - light.pos.y, light.height - 0.0)).xzy;
    //             let normal_dir = mix(normalize(normal.xyz * 2f - 1f), vec3f(0f), 0.5);

    //             normal_multi = max(0f, dot(normal_dir, light_dir));
    //         }
    //     };

    //     res = vec4f(light.color, 0f) * normal_multi;

    //     if dist <= light.inner_range {
    //         res = min(vec4f(1), vec4f(light.color, 0) * light.intensity * normal_multi);
    //     }
    //     else {
    //         let x = (dist - light.inner_range) / (light.range - light.inner_range);

    //         if light.falloff == 0 {
    //             res = min(vec4f(1), vec4f(light.color, 0) * light.intensity * (1. - x * x) * normal_multi);
    //         }
    //         else if light.falloff == 1 { 
    //             res = min(vec4f(1), vec4f(light.color, 0) * light.intensity * (1. - x) * normal_multi);
    //         }
    //     }

    //     var round_index = 0u;
    //     var start_vertex = 0u;
    //     var sequence_index = 0u;
    //     var id_index = 0u;

    //     var shadow = vec3f(1); 
    //     var i = 0u; 

    //     loop {
    //         if (i >= arrayLength(&occluders)) {
    //             break;
    //         }
            
    //         if (stencil.a > 0.1) {
    //             if (config.z_sorting == 1 && occluders[i].z_sorting == 1 && stencil.g >= occluders[i].z) {
    //                 continue;
    //             }

    //             if (is_excluded(i, id_index, stencil.r)) {
    //                 continue;
    //             }
    //         }

    //         if (occluders[i].round == 1) {
    //             let result = round_check(pos, round_index); 

    //             if result.occluded == true {
    //                 shadow = shadow_blend(shadow, occluders[i].color, occluders[i].opacity);
    //             }
    //             else if config.softness > 0 && result.extreme_angle < soft_angle {
    //                 shadow = shadow_blend(shadow, occluders[i].color, occluders[i].opacity * (1f - (result.extreme_angle / soft_angle)));
    //             }
    //         }

    //         else {
    //             for (var s = sequence_index; s < sequence_index + occluders[i].n_sequences; s++) {
    //                 let result = is_occluded(pos, s, start_vertex); 

    //                 if result.occluded == true {    
    //                     shadow = shadow_blend(shadow, occluders[i].color, occluders[i].opacity);
    //                 }
    //                 else if config.softness > 0 && result.extreme_angle < soft_angle {
    //                     shadow = shadow_blend(shadow, occluders[i].color, occluders[i].opacity * (1f - (result.extreme_angle / soft_angle)));
    //                 }

    //                 start_vertex += sequences[s];
    //             }
    //             start_vertex -= occluders[i].n_vertices;
    //         }

    //         continuing {
    //             sequence_index += occluders[i].n_sequences;
    //             start_vertex += occluders[i].n_vertices;
    //             id_index += occluders[i].n_sprites;

    //             if (occluders[i].round == 1) {
    //                 round_index += 1;
    //             }
                
    //             i += 1;
    //         }
    //     }
    //     res *= vec4f(shadow, 1);
    //     prev = max(prev, res);
    // }
    // return prev;
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

    var extreme_angle = 0f;
    if config.softness > 0 {
        extreme_angle = min(
            get_extreme_angle(pos, vertices[start_vertex].pos),  
            get_extreme_angle(pos, vertices[start_vertex + sequences[sequence] - 1].pos)
        );
    }

    if maybe_prev == -1 {
        return OcclusionResult(
            false, 
            extreme_angle,
        );
    }

    let prev = u32(maybe_prev);

    if prev + 1 >= sequences[sequence]  {
        return OcclusionResult(
            false,
            extreme_angle,
        );
    }

    if same_orientation(vertices[start_vertex + prev].pos, vertices[start_vertex + prev + 1].pos, pos, light.pos) {
        return OcclusionResult(
            false,
            extreme_angle,
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

// checks if pixel is blocked by round occluder
fn round_check(pos: vec2f, occluder: u32) -> OcclusionResult {
    // let center = round_occluders[occluder].pos;
    // let width = round_occluders[occluder].width / 2; 
    // let height = round_occluders[occluder].height / 2; 
    // let radius = round_occluders[occluder].radius;

    // var rot = round_occluders[occluder].rot;
    
    // if (rot > PI2) {
    //     rot = rot - PI2 * floor(rot / PI2);
    // }

    // let cos_sin = vec2f(cos(rot), sin(rot));

    var extreme_angle = 10f;

    // if (width > 0) {
    //     let top_edge = vec4f(
    //         center + rotate(vec2f(-width, height + radius), cos_sin), 
    //         center + rotate(vec2f(width, height + radius), cos_sin)
    //     );

    //     if intersect(top_edge.xy, top_edge.zw, pos, light.pos) {
    //         return OcclusionResult(true, 0f);
    //     }

    //     let bottom_edge = vec4f(
    //         center + rotate(vec2f(-width, -height - radius), cos_sin), 
    //         center + rotate(vec2f(width, -height - radius), cos_sin)
    //     );

    //     if intersect(bottom_edge.xy, bottom_edge.zw, pos, light.pos) {
    //         return OcclusionResult(true, 0f);
    //     }

    //     if config.softness > 0 {
    //         extreme_angle = min(
    //             extreme_angle, 
    //             min(
    //                 min(get_extreme_angle(pos, top_edge.xy),  get_extreme_angle(pos, top_edge.zw)),
    //                 min(get_extreme_angle(pos, bottom_edge.xy), get_extreme_angle(pos, bottom_edge.zw))
    //             )
    //         );
    //     }
    // }

    // if (height > 0) {
    //     let right_edge = vec4f(
    //         center + rotate(vec2f(width + radius, height), cos_sin),
    //         center + rotate(vec2f(width + radius, -height), cos_sin)
    //     );

    //     if intersect(right_edge.xy, right_edge.zw, pos, light.pos) {
    //         return OcclusionResult(true, 0f);
    //     }
        
    //     let left_edge = vec4f(
    //         center + rotate(vec2f(-width - radius, height), cos_sin), 
    //         center + rotate(vec2f(-width - radius, -height), cos_sin)
    //     );

    //     if intersect(left_edge.xy, left_edge.zw, pos, light.pos) {
    //         return OcclusionResult(true, 0f);
    //     }

    //     if config.softness > 0 {
    //         extreme_angle = min(
    //             extreme_angle, 
    //             min(
    //                 min(get_extreme_angle(pos, right_edge.xy),  get_extreme_angle(pos, right_edge.zw)),
    //                 min(get_extreme_angle(pos, left_edge.xy), get_extreme_angle(pos, left_edge.zw)),
    //             )
    //         );
    //     }
    // }

    // if (radius > 0) {
    //     let top_left = center + rotate(vec2f(-width, height), cos_sin);
    //     if intersects_arc(pos, light.pos, top_left, radius, rotate_arctan(PIDIV2, rot), rotate_arctan(PI, rot)) {
    //         return OcclusionResult(true, 0f);
    //     }

    //     let top_right = center + rotate(vec2f(width, height), cos_sin);
    //     if intersects_arc(pos, light.pos, top_right, radius, rotate_arctan(0, rot), rotate_arctan(PIDIV2, rot)) {
    //         return OcclusionResult(true, 0f);
    //     }
        
    //     let bottom_right = center + rotate(vec2f(width, -height), cos_sin);
    //     if intersects_arc(pos, light.pos, bottom_right, radius, rotate_arctan(-PIDIV2, rot), rotate_arctan(0, rot)) {
    //         return OcclusionResult(true, 0f);
    //     }
        
    //     let bottom_left = center + rotate(vec2f(-width, -height), cos_sin);
    //     if intersects_arc(pos, light.pos, bottom_left, radius, rotate_arctan(-PI, rot), rotate_arctan(-PIDIV2, rot)) {
    //         return OcclusionResult(true, 0f);
    //     }

    //     if config.softness > 0 {
    //         extreme_angle = min(
    //             extreme_angle, 
    //             min(
    //                 min(
    //                     get_arc_extremes(pos, light.pos, top_left, radius, rotate_arctan(PIDIV2, rot), rotate_arctan(PI, rot)),  
    //                     get_arc_extremes(pos, light.pos, top_right, radius, rotate_arctan(0, rot), rotate_arctan(PIDIV2, rot)),  
    //                 ),
    //                 min(
    //                     get_arc_extremes(pos, light.pos, bottom_right, radius, rotate_arctan(-PIDIV2, rot), rotate_arctan(0, rot)), 
    //                     get_arc_extremes(pos, light.pos, bottom_left, radius, rotate_arctan(-PI, rot), rotate_arctan(-PIDIV2, rot)),  
    //                 )
    //             )
    //         );
    //     }
    // }

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