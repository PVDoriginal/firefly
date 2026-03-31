#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

#import firefly::types::{
    view, PointLight, LightingData, PolyOccluder, RoundOccluder, OccluderPointer, 
    FireflyConfig, BinIndices, N_BINS,
}

#import firefly::utils::{
    ndc_to_world, frag_coord_to_ndc, orientation, same_orientation, intersect, blend, 
    shadow_blend, intersects_arc, rotate, rotate_arctan, between_arctan, distance_point_to_line,
    intersection_point, rect_intersection, rect_line_intersection, intersects_axis_edge, intersects_corner_arc,
    rotate_90, rotate_90_cc, intersects_half
}

@group(1) @binding(0)
var texture_sampler: sampler;

@group(1) @binding(1)
var<storage> lights: array<PointLight>;

@group(1) @binding(2)
var<storage> light_index: u32; 

@group(1) @binding(3)
var<storage> round_occluders: array<RoundOccluder>;

@group(1) @binding(4)
var<storage> poly_occluders: array<PolyOccluder>;

@group(1) @binding(5)
var<storage> vertices: array<vec2f>;

@group(1) @binding(6)
var<storage> occluders: array<OccluderPointer>;

@group(1) @binding(7)
var<storage> bin_indices: BinIndices;

@group(1) @binding(8)
var sprite_stencil: texture_2d<f32>;

@group(1) @binding(9)
var normal_map: texture_2d<f32>;

@group(1) @binding(10)
var<uniform> config: FireflyConfig;

const PI2: f32 = 6.28318530718;
const PI: f32 = 3.14159265359;
const PIDIV2: f32 = 1.57079632679; 

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let light = lights[light_index];

    var res = vec4f(0);
    
    let pos = ndc_to_world(frag_coord_to_ndc(in.position.xy));
    let normal = textureSample(normal_map, texture_sampler, in.uv);
    let stencil = textureSample(sprite_stencil, texture_sampler, in.uv);
    let soft_angle = config.softness; 

    let dist = distance(pos, light.pos);
    
    let a = pos - light.pos;
    let b = light.dir;
    let angle = acos(dot(a, b) / (length(a) * length(b)));
    
    if (dist < light.range && angle <= light.angle / 2.) {
        var normal_multi = 1f;
    
        if config.normal_mode != 0 && normal.a > 0 {
            let normal_dir = mix(normalize(normal.xyz * 2f - 1f), vec3f(0f), config.normal_attenuation);

            if normal.b == 0.0 {
                normal_multi = 0.0;
            }
            else if normal.b == 0.1 {
                normal_multi = 1.0;
            }
            else if config.normal_mode == 1 {
                let light_dir = normalize(vec3f(light.pos.x - pos.x, light.pos.y - pos.y, light.z - stencil.g));
                normal_multi = max(0f, dot(normal_dir, light_dir));
            }
            else if config.normal_mode == 2 {
                let light_dir = normalize(vec3f(light.pos.x - pos.x, light.height - stencil.b, light.z - stencil.g));
                normal_multi = max(0f, dot(normal_dir, light_dir));
            }
        }; 

        if dist <= light.inner_range {
            res = vec4f(light.color.xyz, 0) * light.intensity * normal_multi;
        }
        else {
            let x = (dist - light.inner_range) / (light.range - light.inner_range);

            if light.falloff == 0 {
                let x2 = x * x; 
                res = vec4f(light.color.xyz, 0) * light.intensity * ((1.0 - x2) * (1.0 - x2) / (1.0 + light.falloff_intensity * x2)) * normal_multi;
            }
            else if light.falloff == 1 { 
                res = vec4f(light.color.xyz, 0) * light.intensity * ((1.0 - x) / (1.0 + light.falloff_intensity * x)) * normal_multi;
            }
        }

        var round_index = 0u;
        var start_vertex = 0u;
        var sequence_index = 0u;

        var shadow = vec3f(1); 

        let bin = u32(floor(((atan2(pos.y - light.pos.y, pos.x - light.pos.x) + PI) / PI2) * f32(N_BINS)));

        let left = bin_indices.indices[bin]; 
        let right = bin_indices.indices[bin + 1];

        // if left >= right {
        //     return vec4f(0, 0, 1, 1);
        // }

        var prev_index: u32;
        var prev_rev: bool;
        var acc_res: OccRes;

        for (var pointer_index = left; pointer_index < right; pointer_index += 1) {
            let pointer = occluders[pointer_index];
            
            if pointer.distance > dist { break; }
            
            let occluder_type = pointer.index & 2147483648u;
            let occluder_index = pointer.index & 2147483647u;

            // round occluder
            if occluder_type == 0 {
                if stencil.a > 0.1 {
                    if config.z_sorting == 1 && round_occluders[occluder_index].z_sorting == 1 && stencil.g >= round_occluders[occluder_index].z {
                        continue;
                    }
                }

                let result = round_check(pos, occluder_index); 

                if result.occluded == true {
                    shadow = shadow_blend(shadow, round_occluders[occluder_index].color, round_occluders[occluder_index].opacity);
                }                    
                // else if config.softness > 0 && result.extreme_angle < soft_angle {
                //     shadow = shadow_blend(shadow, round_occluders[index].color, round_occluders[index].opacity * (1f - (result.extreme_angle / soft_angle)));
                // }
            }
            // poly occluder
            else {
                // res.r = 1.0;

                if stencil.a > 0.1 {
                    if config.z_sorting == 1 && poly_occluders[occluder_index].z_sorting == 1 && stencil.g >= poly_occluders[occluder_index].z {
                        continue;
                    }
                }

                if prev_index != occluder_index {
                    shadow = apply_occlusion(shadow, prev_index, acc_res, pos, prev_rev);
                    
                    prev_index = occluder_index; 
                    acc_res = res_no_occlusion();
                    // acc_res = OccRes(false, false, 0.0, false, 0.0);
                }

                let term = (pointer.min_v & 3221225472u) >> 30u;

                let rev = (pointer.min_v & 536870912u) >> 29u;

                let min_v = pointer.min_v & 536870911u;
                let split = pointer.split;
                let length = pointer.length & 1073741823u;

                // if split == 0 {
                //     return vec4<f32>(0, 1, 0, 1);
                // }

                let result = poly_check(pos, occluder_index, term, rev, min_v, split, length); 
                acc_res = accumulate_occlusion(acc_res, result, pos);
                prev_rev = rev != 0u;

                // if result > 0.0 {
                //     shadow = shadow_blend(shadow, poly_occluders[occluder_index].color, poly_occluders[occluder_index].opacity * result);
                // }
            }
            
            if dot(shadow, shadow) < 0.0001 {
                break;
            }
        }


        shadow = apply_occlusion(shadow, prev_index, acc_res, pos, prev_rev);
        
        res *= vec4f(shadow, 1);
    }

#ifdef TONEMAP_IN_SHADER
    res = tonemapping::tone_mapping(res, view.color_grading);
#endif

    return res;
}

fn accumulate_occlusion(prev_result: OccRes, result: OccRes, pos: vec2<f32>) -> OccRes {
    var acc_res = prev_result;

    if result.occluded {
        acc_res.occluded = true;
    
        if result.occluded_left {

            if !acc_res.occluded_left || orientation(pos, acc_res.left_point, result.left_point) < 0 {
                acc_res.left_point = result.left_point;
            }
            
            acc_res.left = max(acc_res.left, result.left);
            acc_res.occluded_left = true;
        }

        if result.occluded_right {

            if !acc_res.occluded_right || orientation(pos, acc_res.right_point, result.right_point) > 0 {
                acc_res.right_point = result.right_point;
            }

            acc_res.right = max(acc_res.right, result.right);
            acc_res.occluded_right = true;
        }

        if result.behind_occluder {
            acc_res.behind_occluder = true;
        }

        if result.behind_left {
            acc_res.behind_left = true;
        }

        if result.behind_right {
            acc_res.behind_right = true;
        }
    }
    
    return acc_res;
}

fn apply_occlusion(shadow: vec3<f32>, index: u32, occ: OccRes, pos: vec2<f32>, prev_rev: bool) -> vec3<f32> {    
    let light = lights[light_index];

    if occ.occluded && index != 0u {
        var multi = 0.0;

        if occ.occluded_left || occ.occluded_right {
            multi = 1.0;
            if occ.occluded_left {
                multi = min(multi, occ.left);
            }
            if occ.occluded_right {
                multi = min(multi, occ.right);
            }
        }
        else if occ.behind_occluder {
            multi = 1.0;
        }
        

        return shadow_blend(shadow, poly_occluders[index].color, poly_occluders[index].opacity * multi);
    }
    else {
        return shadow;
    }
}

struct OccRes {
    occluded: bool, 

    occluded_left: bool,
    left: f32,
    left_point: vec2<f32>,
     
    occluded_right: bool,
    right: f32,
    right_point: vec2<f32>,

    behind_occluder: bool,

    behind_left: bool, 
    behind_right: bool,
}

fn res_full_occlusion() -> OccRes {
    return OccRes(true, false, 0.0, vec2<f32>(0.0), false, 0.0, vec2<f32>(0.0), true, true, true);
}

fn res_no_occlusion() -> OccRes {
    return OccRes(false, false, 0.0, vec2<f32>(0.0), false, 0.0, vec2<f32>(0.0), false, false, false);
}

fn get_extreme_angle(pos: vec2f, extreme: vec2f) -> f32 {
    let light = lights[light_index];

    let light_proj = (extreme - light.pos) + extreme;  
    
    let a = vec2f(extreme.x - pos.x, extreme.y - pos.y);
    let b = vec2f(extreme.x - light_proj.x, extreme.y - light_proj.y);
    let angle = acos(dot(a, b) / (length(a) * length(b)));
    
    return angle;
}

fn poly_check(pos: vec2f, index: u32, term: u32, rev: u32, min_v: u32, split: u32, length: u32) -> OccRes {
    let light = lights[light_index];
    let occluder = poly_occluders[index];

    let angle = atan2(pos.y - light.pos.y, pos.x - light.pos.x);

    var maybe_prev = 0; 

    var start = min_v; 
    var len = length; 

    if rev == 0 {

        if term == 1 {
            len = split + 1;
        }
        else if term == 2 {
            start = min_v + split - 1;
            len = length - split + 1;
        }

        maybe_prev = bs_vertex_forward(angle, start, len, term, occluder.start_vertex, occluder.n_vertices);
    }
    else {
        if term == 1 {
            len = split + 1;
        }
        else if term == 2 {
            start = min_v - split + 1;
            len = length - split + 1;
        }

        maybe_prev = bs_vertex_reverse(angle, start, len, term, occluder.start_vertex, occluder.n_vertices);
    }

    var is_occluded = false;

    let out_of_bounds = maybe_prev < 0 || maybe_prev + 1 >= i32(len);

    if !out_of_bounds {
        if rev == 0 {
            let v1 = vertices[start + u32(maybe_prev) - select(0, occluder.n_vertices, start + u32(maybe_prev) >= occluder.start_vertex + occluder.n_vertices)];
            let v2 = vertices[start + u32(maybe_prev) + 1 - select(0, occluder.n_vertices, start + u32(maybe_prev) + 1 >= occluder.start_vertex + occluder.n_vertices)];

            is_occluded = !same_orientation(v1, v2, pos, light.pos);
        }
        else {
            let v1 = vertices[i32(start) - maybe_prev + select(0, i32(occluder.n_vertices), i32(start) - maybe_prev < i32(occluder.start_vertex))];
            let v2 = vertices[i32(start) - maybe_prev - 1 + select(0, i32(occluder.n_vertices), i32(start) - maybe_prev - 1 < i32(occluder.start_vertex))];

            is_occluded = !same_orientation(v1, v2, pos, light.pos);
        }
    }

    if config.softness > 0 && (is_occluded || out_of_bounds) {//&& rev == 0 {
        if rev == 0 {
            let loops = min_v + length - 1 >= occluder.start_vertex + occluder.n_vertices;
            let last = min_v + length - 1 - select(0, occluder.n_vertices, loops);
            
            let prev = select(min_v - 1, occluder.start_vertex + occluder.n_vertices - 1, min_v - 1 < occluder.start_vertex); 
            let next = select(last + 1, last + 1 - occluder.n_vertices, last + 1 >= occluder.start_vertex + occluder.n_vertices);

            let prev_last  = select(last - 1, last - 1 + occluder.n_vertices, last - 1 < occluder.start_vertex);
            let prev_first = select(min_v + 1, min_v + 1 - occluder.n_vertices, min_v + 1 >= occluder.start_vertex + occluder.n_vertices);
            
            return get_softness_multi(pos, vertices[min_v], vertices[prev_first], vertices[last], vertices[prev_last], vertices[prev], vertices[next], out_of_bounds, term);
        }
        else {
            let loops = i32(min_v) - i32(length) + 1 < i32(occluder.start_vertex);
            let last = u32(i32(min_v) - i32(length) + 1 + select(0, i32(occluder.n_vertices), loops));
            
            let prev = select(min_v + 1, occluder.start_vertex, min_v + 1 >= occluder.start_vertex + occluder.n_vertices); 
            let next = select(last - 1, last - 1 + occluder.n_vertices, last - 1 < occluder.start_vertex);

            let prev_last  = select(last + 1, last + 1 - occluder.n_vertices, last + 1 >= occluder.start_vertex + occluder.n_vertices);
            let prev_first = select(min_v - 1, min_v - 1 + occluder.n_vertices, min_v - 1 < occluder.start_vertex);
            
            return get_softness_multi(pos, vertices[min_v], vertices[prev_first], vertices[last], vertices[prev_last], vertices[prev], vertices[next], out_of_bounds, term);
        }
    }
    
    if is_occluded {
        return res_full_occlusion();
    }
    return res_no_occlusion();
}

fn get_softness_multi(pos: vec2<f32>, extreme_left: vec2<f32>, prev_extreme_left: vec2<f32>, extreme_right: vec2<f32>, prev_extreme_right: vec2<f32>, prev: vec2<f32>, next: vec2<f32>, out_of_bounds: bool, term: u32) -> OccRes {
    let light = lights[light_index];

    let range = light.inner_range;

    let left_range = min(range, distance(extreme_left, light.pos)); 
 
    var left_t1 = light.pos + rotate_90(normalize(extreme_left - light.pos)) * left_range;
    var left_t2 = light.pos + rotate_90_cc(normalize(extreme_left - light.pos)) * left_range;

    let rev = orientation(light.pos, extreme_right, extreme_left) > 0;
    
    if orientation(left_t2, extreme_left, prev) < 0 {
        left_t2 = (extreme_left - prev) * 2.0 + extreme_left;
    }

    if rev && orientation(left_t1, extreme_left, extreme_right) > 0 {
        left_t1 = (extreme_left - extreme_right) * 2.0 + extreme_left;
    }

    let right_range = min(range, distance(extreme_right, light.pos));

    var right_t1 = light.pos + rotate_90(normalize(extreme_right - light.pos)) * right_range;
    var right_t2 = light.pos + rotate_90_cc(normalize(extreme_right - light.pos)) * right_range;

    if orientation(right_t1, extreme_right, next) > 0 {
        right_t1 = (extreme_right - next) * 2.0 + extreme_right;
    }

    if rev && orientation(right_t2, extreme_right, extreme_left) < 0 {
        right_t2 = (extreme_right - extreme_left) * 2.0 + extreme_right;
    }

    let left_is_valid = orientation(prev, extreme_left, prev_extreme_left) <= 0;
    let right_is_valid = orientation(next, extreme_right, prev_extreme_right) >= 0;

    var left = false;
    var right = false;

    var left_multi = 0.0; 
    var right_multi = 0.0;

    let above_left = orientation(left_t1, extreme_left, pos) < 0;
    let under_left = orientation(left_t2, extreme_left, pos) > 0;

    let inside_left = !above_left && !under_left;

    let under_right = orientation(right_t2, extreme_right, pos) > 0;
    let above_right = orientation(right_t1, extreme_right, pos) < 0;

    let inside_right = !above_right && !under_right;

    var behind_left = false;
    var behind_right = false;
    
    if inside_left {
        left = true;
        let left2 = normalize(extreme_left - left_t2);
        left_multi = 1.0 - acos(dot(normalize(pos - extreme_left), left2)) / acos(dot(normalize(extreme_left - left_t1), left2));
    }
    
    if inside_right {
        right = true;
        let right1 = normalize(extreme_right - right_t1);
        right_multi = 1.0 - acos(dot(normalize(pos - extreme_right), right1)) / acos(dot(normalize(extreme_right - right_t2), right1));
    }

    let behind_occluder = !out_of_bounds;

    // if !out_of_bounds && !left_is_valid && !right_is_valid && !inside_left && !inside_right {
    //     behind_left = true;
    //     behind_right = true;
    // }

    return OccRes(left || right || !out_of_bounds || behind_left || behind_right, left || behind_left, left_multi, extreme_left, right || behind_right, right_multi, extreme_right, behind_occluder, behind_left, behind_right);   
}

fn angle_term(p: vec2f, i: u32, length: u32, term: u32) -> f32 {
    let light = lights[light_index];
    let angle = atan2(p.y - light.pos.y, p.x - light.pos.x);
    
    if i == length - 1 {
        if term == 1 {
            return angle + PI2;
        }
        else {
            return angle;
        }
    }
    else if i == 0 {
        if term == 2 {
            return angle - PI2; 
        }
        else {
            return angle;
        }
    }

    return angle;
}

fn vertex_forward(start: u32, index: u32, start_vertex: u32, n_vertices: u32) -> vec2<f32> {
    if start + index >= start_vertex + n_vertices {
        return vertices[start + index - n_vertices];
    }
    return vertices[start + index];
}

fn vertex_reverse(start: u32, index: i32, start_vertex: u32, n_vertices: u32) -> vec2<f32> {
    if i32(start) - i32(index) < i32(start_vertex) {
        return vertices[u32(i32(start) - i32(index) + i32(n_vertices))];
    }
    return vertices[u32(i32(start) - i32(index))];
} 

fn bs_vertex_forward(angle: f32, start: u32, length: u32, term: u32, start_vertex: u32, n_vertices: u32) -> i32 {
    let light = lights[light_index];

    var ans = -1;
    
    var low = 0i; 
    var high = i32(length) - 1; 

    if angle < angle_term(vertex_forward(start, u32(low), start_vertex, n_vertices), u32(low), length, term) {
        return -1;
    }

    if angle >= angle_term(vertex_forward(start, u32(high), start_vertex, n_vertices), u32(high), length, term) {
        return high + 1;
    }

    while (low <= high) {
        let mid = low + (high - low + 1) / 2;
        let val = angle_term(vertex_forward(start, u32(mid), start_vertex, n_vertices), u32(mid), length, term);

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

fn bs_vertex_reverse(angle: f32, start: u32, length: u32, term: u32, start_vertex: u32, n_vertices: u32) -> i32 {
    let light = lights[light_index];

    var ans = -1;
    
    var low = 0i; 
    var high = i32(length) - 1;

    if angle < angle_term(vertex_reverse(start, low, start_vertex, n_vertices), u32(low), length, term) {
        return -1;
    }

    if angle >= angle_term(vertex_reverse(start, high, start_vertex, n_vertices), u32(high), length, term) {
        return high + 1;
    }

    while (low <= high) {
        let mid = low + (high - low + 1) / 2;
        let val = angle_term(vertex_reverse(start, mid, start_vertex, n_vertices), u32(mid), length, term);

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


// fn bs_vertex(angle: f32, offset: u32, size: u32) -> i32 {
//     var ans = -1;
    
//     var low = 0i; 
//     var high = i32(size) - 1;

//     if angle < vertices[u32(low) + offset].angle {
//         return -1;
//     }

//     if angle >= vertices[u32(high) + offset].angle {
//         return high + 1;
//     }

//     while (low <= high) {
//         let mid = low + (high - low + 1) / 2;
//         let val = vertices[u32(mid) + offset].angle;

//         if (val < angle) {
//             ans = i32(mid);
//             low = mid + 1;
//         }
//         else {
//             high = mid - 1;
//         }
//     }

//     return ans;
// }

struct OcclusionResult {
    occluded: bool, 
    extreme_angle: f32,
}

// checks if pixel is blocked by round occluder
fn round_check(pos: vec2f, occluder: u32) -> OcclusionResult {
    let light = lights[light_index];

    let occ = round_occluders[occluder];
    let half_w = occ.width * 0.5;
    let half_h = occ.height * 0.5;
    let radius = occ.radius;

    let relative_pos = pos - occ.pos; 
    let relative_light = light.pos - occ.pos; 

    var rot = round_occluders[occluder].rot;
    
    if (rot > PI2) {
        rot = rot - PI2 * floor(rot / PI2);
    }

    let c = cos(occ.rot);
    let s = sin(occ.rot);

    let p_local = vec2f(relative_pos.x * c + relative_pos.y * s, -relative_pos.x * s + relative_pos.y * c);
    let l_local = vec2f(relative_light.x * c + relative_light.y * s, -relative_light.x * s + relative_light.y * c);

    var rect = vec4f(-(half_w + radius), -(half_h + radius), half_w + radius, half_h + radius);

    var extreme_angle = 10.0;

    if !rect_line_intersection(p_local, l_local, rect) {

        if config.softness > 0 {
            extreme_angle = get_round_extreme_angle(half_w, half_h, p_local, l_local, radius);
        }

        return OcclusionResult(false, extreme_angle);
    }

    if (occ.width > 0) {
        // top edge
        if intersects_axis_edge(p_local, l_local, half_h + radius, -half_w, half_w, false) {
            return OcclusionResult(true, 0.0);
        }

        // bottom edge
        if intersects_axis_edge(p_local, l_local, -(half_h + radius), -half_w, half_w, false) {
            return OcclusionResult(true, 0.0);
        }
    }

    if (occ.height > 0) {
        // right edge
        if intersects_axis_edge(p_local, l_local, half_w + radius, -half_h, half_h, true) {
            return OcclusionResult(true, 0.0);
        }

        // left edge
        if intersects_axis_edge(p_local, l_local, -(half_w + radius), -half_h, half_h, true) {
            return OcclusionResult(true, 0.0);
        }
    }

    if (radius > 0) {
        let quadrants = array<vec2f, 4>(vec2f(1,1), vec2f(-1,1), vec2f(1,-1), vec2f(-1,-1));
        let centers = array<vec2f, 4>(vec2f(half_w, half_h), vec2f(-half_w, half_h), vec2f(half_w, -half_h), vec2f(-half_w, -half_h));
        for(var i = 0u; i < 4u; i++) {
            if intersects_corner_arc(p_local, l_local, centers[i], radius, quadrants[i]) { 
                return OcclusionResult(true, 0.0); 
            }
        }
    }

    if config.softness > 0 {
        extreme_angle = get_round_extreme_angle(half_w, half_h, p_local, l_local, radius);
    }

    return OcclusionResult(false, extreme_angle);
}

fn get_round_extreme_angle(half_w: f32, half_h: f32, p_local: vec2f, l_local: vec2f, radius: f32) -> f32 {
    var extreme_angle = 10.0;

    extreme_angle = min(
        extreme_angle, 
        min(
            min(
                get_extreme_angle_local(p_local, l_local, vec2f(half_w + radius, half_h)),
                get_extreme_angle_local(p_local, l_local, vec2f(half_w + radius, -half_h))
            ),
            min(
                get_extreme_angle_local(p_local, l_local, vec2f(-(half_w + radius), half_h)),
                get_extreme_angle_local(p_local, l_local, vec2f(-(half_w + radius), -half_h))
            )
        )
    );

    extreme_angle = min(
        extreme_angle, 
        min(
            min(
                get_extreme_angle_local(p_local, l_local, vec2f(half_w + radius, half_h)),
                get_extreme_angle_local(p_local, l_local, vec2f(half_w + radius, -half_h))
            ),
            min(
                get_extreme_angle_local(p_local, l_local, vec2f(-(half_w + radius), half_h)),
                get_extreme_angle_local(p_local, l_local, vec2f(-(half_w + radius), -half_h))
            )
        )
    );

    let centers = array<vec2f, 4>(vec2f(half_w, half_h), vec2f(-half_w, half_h), vec2f(half_w, -half_h), vec2f(-half_w, -half_h));
    extreme_angle = min(
        extreme_angle, 
        min(
            min(
                get_arc_extremes(p_local, l_local, centers[0], radius, 0.0, PIDIV2),
                get_arc_extremes(p_local, l_local, centers[1], radius, PIDIV2, PI),
            ),
            min(
                get_arc_extremes(p_local, l_local, centers[2], radius, -PIDIV2, 0.0),
                get_arc_extremes(p_local, l_local, centers[3], radius, -PI, -PIDIV2),
            )
        )
    );

    return extreme_angle;
}

fn round_rect_aabb(center: vec2f, width: f32, height: f32, radius: f32, cos_sin: vec2f) -> vec4f {
    let half_w = width * 0.5;
    let half_h = height * 0.5;

    let ex = half_w * cos_sin.x + half_h * cos_sin.y;
    let ey = half_w * cos_sin.y + half_h * cos_sin.x;

    let min_p = center - vec2f(ex + radius, ey + radius);
    let max_p = center + vec2f(ex + radius, ey + radius);

    return vec4f(min_p.x, min_p.y, max_p.x, max_p.y);
}

fn get_arc_extremes(p_local: vec2f, l_local: vec2f, c: vec2f, r: f32, start_angle: f32, end_angle: f32) -> f32 {
    let diff = p_local - c;
    let dist_sq = dot(diff, diff);
    
    // Pixel is inside the corner radius
    // if (dist_sq <= r * r) { return 10.0; } 

    let dist = sqrt(dist_sq);
    let th = acos(r / dist);
    let d = atan2(diff.y, diff.x);
    
    let d1 = d + th;
    let d2 = d - th;

    // Tangent points on the circle
    let t1 = vec2f(c.x + r * cos(d1), c.y + r * sin(d1));
    let t2 = vec2f(c.x + r * cos(d2), c.y + r * sin(d2));

    // Angles of tangent points relative to center 'c'
    let a1 = atan2(t1.y - c.y, t1.x - c.x);
    let a2 = atan2(t2.y - c.y, t2.x - c.x);

    var res = 10.0;

    if (between_arctan(a1, start_angle, end_angle)) {
        res = min(res, get_extreme_angle_local(p_local, l_local, t1));
    }

    if (between_arctan(a2, start_angle, end_angle)) {
        res = min(res, get_extreme_angle_local(p_local, l_local, t2));
    }

    return res;
}

fn get_extreme_angle_local(p: vec2f, l: vec2f, t: vec2f) -> f32 {
    let light_proj = (t - l) + t;  
    
    let a = t - p;
    let b = t - light_proj;
    // let angle = acos(dot(a, b) / (length(a) * length(b)));
    
    let angle = acos(dot(normalize(a), normalize(b)));

    return angle;
}
