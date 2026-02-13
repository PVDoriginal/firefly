#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

#import firefly::types::{
    view, PointLight, LightingData, PolyOccluder, RoundOccluder, OccluderPointer, 
    FireflyConfig, Bin, BinCounts, N_OCCLUDERS, N_BINS,
}

#import firefly::utils::{
    ndc_to_world, frag_coord_to_ndc, orientation, same_orientation, intersect, blend, 
    shadow_blend, intersects_arc, rotate, rotate_arctan, between_arctan, distance_point_to_line,
    intersection_point, rect_intersection, rect_line_intersection, intersects_axis_edge, intersects_corner_arc,
    rotate_90, rotate_90_cc,
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
var<storage> bins: array<array<Bin, N_BINS>>;

@group(1) @binding(7)
var<storage> bin_counts: BinCounts;

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
    // let stencil = textureLoad(sprite_stencil, vec2<i32>(in.uv * vec2<f32>(textureDimensions(sprite_stencil))), 0);
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

        for (var bin_set = 0u; bin_set <= bin_counts.counts[bin]; bin_set += 1) {
            if bins[bin_set][bin].n_occluders == 0 {
                // return vec4f(1, 0, 0, 1);
                break;
            }

            for (var i = 0u; i < bins[bin_set][bin].n_occluders; i += 1) {

                if bins[bin_set][bin].occluders[i].distance > dist { break; }

                let occluder_type = bins[bin_set][bin].occluders[i].index & 2147483648u;

                // round occluder
                if occluder_type == 0 {
                    let index = bins[bin_set][bin].occluders[i].index;
                    
                    if stencil.a > 0.1 {
                        if config.z_sorting == 1 && round_occluders[index].z_sorting == 1 && stencil.g >= round_occluders[index].z {
                            continue;
                        }
                    }

                    // let result = round_check(pos, index); 
    
                    // if result.occluded == true {
                    //     shadow = shadow_blend(shadow, round_occluders[index].color, round_occluders[index].opacity);
                    // }                    
                    // else if config.softness > 0 && result.extreme_angle < soft_angle {
                    //     shadow = shadow_blend(shadow, round_occluders[index].color, round_occluders[index].opacity * (1f - (result.extreme_angle / soft_angle)));
                    // }
                }
                // poly occluder
                else {
                    let index = (bins[bin_set][bin].occluders[i].index << 4) >> 4; 

                    if stencil.a > 0.1 {
                        if config.z_sorting == 1 && poly_occluders[index].z_sorting == 1 && stencil.g >= poly_occluders[index].z {
                            continue;
                        }
                    }

                    let result = is_occluded(pos, bins[bin_set][bin].occluders[i], index); 

                    if result > 0.0 {
                        shadow = shadow_blend(shadow, poly_occluders[index].color, poly_occluders[index].opacity * result);
                    }
                }

                if dot(shadow, shadow) < 0.0001 {
                    break;
                }
            }

            if dot(shadow, shadow) < 0.0001 {
                break;
            }
        }
        res *= vec4f(shadow, 1);
    }

#ifdef TONEMAP_IN_SHADER
    res = tonemapping::tone_mapping(res, view.color_grading);
#endif

    return res;
}

fn get_extreme_angle(pos: vec2f, extreme: vec2f) -> f32 {
    let light = lights[light_index];

    let light_proj = (extreme - light.pos) + extreme;  
    
    let a = vec2f(extreme.x - pos.x, extreme.y - pos.y);
    let b = vec2f(extreme.x - light_proj.x, extreme.y - light_proj.y);
    let angle = acos(dot(a, b) / (length(a) * length(b)));
    
    return angle;
}

fn is_occluded(pos: vec2f, pointer: OccluderPointer, index: u32) -> f32 {
    let light = lights[light_index];
    let occluder = poly_occluders[index];

    let term = (pointer.index & 1610612736u) >> 29;
    let rev = (pointer.index & 268435456u) >> 28;

    let angle = atan2(pos.y - light.pos.y, pos.x - light.pos.x);

    var maybe_prev = 0; 

    if rev == 0 {
        maybe_prev = bs_vertex_forward(angle, pointer.min_v, pointer.length, term);
    }
    else {
        maybe_prev = bs_vertex_reverse(angle, pointer.min_v, pointer.length, term);
    }

    var is_occluded = false;

    // bounds
    // 0 - in bounds
    // 1 - left out of bounds
    // 2 - right out of bounds 

    var bounds = 0u;

    if maybe_prev < 0 {
        bounds = 1u; 
    }
    else if maybe_prev + 1 >= i32(pointer.length) {
        bounds = 2u;
    }

    let out_of_bounds = bounds != 0u;

    // if 1 > 0 {
    //     if out_of_bounds {
    //         return 1.0;
    //     }
    //     else {
    //         return 0.0;
    //     }
    // }

    if !out_of_bounds {
        if rev == 0 {
            is_occluded = !same_orientation(vertices[pointer.min_v + u32(maybe_prev)], vertices[pointer.min_v + u32(maybe_prev) + 1], pos, light.pos);
        }
        else {
            is_occluded = !same_orientation(vertices[pointer.min_v - u32(maybe_prev)], vertices[pointer.min_v - u32(maybe_prev) - 1], pos, light.pos);
        }
    }

    if config.softness > 0 && (is_occluded || out_of_bounds){
        if rev == 0 {
            var term2 = 0u;
            if term == 3 && pointer.min_v == occluder.vertex_start {
                term2 = 1u;
            }
            if term == 3 && pointer.min_v + pointer.length - 1 == occluder.vertex_start + occluder.n_vertices - 1 {
                term2 = 2u;
            }

            var prev = vec2<f32>(0.0);
            var next = vec2<f32>(0.0);

            if !out_of_bounds {

                if pointer.min_v == occluder.vertex_start {
                    prev = vertices[occluder.vertex_start + occluder.n_vertices - 2];
                }
                else {
                    prev = vertices[pointer.min_v - 1];
                }

                if pointer.min_v + pointer.length - 1 == occluder.vertex_start + occluder.n_vertices - 1 {
                    next = vertices[occluder.vertex_start + 1];
                }
                else {
                    next = vertices[pointer.min_v + pointer.length];
                }
                
                // if orientation(vertices[pointer.min_v], prev, pos) > 0 && orientation(vertices[pointer.min_v + pointer.length - 1], next, pos) < 0 {
                //     return 1.0;
                // }

                // if orientation(vertices[pointer.min_v + pointer.length - 1], next, pos) < 0 {
                //     return 1.0;
                // }
            }
            return get_softness_multi(pos, vertices[pointer.min_v], vertices[pointer.min_v + pointer.length - 1], bounds, term, term2);
        }
        else {
            return get_softness_multi(pos, vertices[pointer.min_v], vertices[pointer.min_v - pointer.length + 1], bounds, term, 0u);
        }
    }
    
    if is_occluded {
        return 1.0;
    }
    else {
        return 0.0;
    }
}

fn get_softness_multi(pos: vec2<f32>, extreme_left: vec2<f32>, extreme_right: vec2<f32>, bounds: u32, term: u32, term2: u32) -> f32 {
    let light = lights[light_index];

    let left_range = min(light.inner_range, distance(extreme_left, light.pos)); 

    let left1 = normalize(extreme_left - light.pos + rotate_90(normalize(extreme_left - light.pos)) * left_range); 
    let left2 = normalize(extreme_left - light.pos + rotate_90_cc(normalize(extreme_left - light.pos)) * left_range); 

    let right_range = min(light.inner_range, distance(extreme_right, light.pos));

    let right1 = normalize(extreme_right - light.pos + rotate_90(normalize(extreme_right - light.pos)) * right_range); 
    let right2 = normalize(extreme_right - light.pos + rotate_90_cc(normalize(extreme_right - light.pos)) * right_range); 

    var left_multi = 1.0; 
    var right_multi = 1.0;

    let left_middle = normalize(pos - extreme_left);
    let right_middle = normalize(pos - extreme_right);

    var ok = false;

    if bounds != 2 && term2 != 1 && term != 2 && acos(dot(left_middle, left1)) < acos(dot(left1, left2)) && acos(dot(left_middle, left2)) < acos(dot(left1, left2)) {
        left_multi = acos(dot(left_middle, left2)) / acos(dot(left1, left2));

        ok = ok || true;
        // return 1.0;
    }

    if bounds != 1 && term2 != 2 && term != 1 && acos(dot(right_middle, right1)) < acos(dot(right1, right2)) && acos(dot(right_middle, right2)) < acos(dot(right1, right2)) {
        right_multi = acos(dot(right_middle, right1)) / acos(dot(right1, right2));

        ok = ok || true; 
        // return 1.0;
    }

    let final_res = min(left_multi, right_multi);

    if ok {
        return final_res;
    }

    if bounds != 0u {
        return 0.0;
    }

    if acos(dot(left_middle, left1)) < acos(dot(left_middle, left2)) && acos(dot(right_middle, right2)) < acos(dot(right_middle, right1)) {
        return 1.0;
    }

    return 0.0;
}

fn angle(p: vec2f) -> f32 {
    let light = lights[light_index];

    return atan2(p.y - light.pos.y, p.x - light.pos.x);
}

fn angle_term(p: vec2f, i: u32, length: u32, term: u32) -> f32 {
    let light = lights[light_index];
    
    if i == length - 1 {
        if term == 1 {
            return atan2(p.y - light.pos.y, p.x - light.pos.x) + PI2;
        }
        else {
            return atan2(p.y - light.pos.y, p.x - light.pos.x);
        }
    }
    else if i == 0 {
        if term == 2 {
            return atan2(p.y - light.pos.y, p.x - light.pos.x) - PI2; 
        }
        else {
            return atan2(p.y - light.pos.y, p.x - light.pos.x);
        }
    }

    return atan2(p.y - light.pos.y, p.x - light.pos.x);
}

fn bs_vertex_forward(angle: f32, start: u32, length: u32, term: u32) -> i32 {
    let light = lights[light_index];

    var ans = -1;
    
    var low = 0i; 
    var high = i32(length) - 1;

    if angle < angle_term(vertices[start + u32(low)], u32(low), length, term) {
        return -1;
    }

    if angle >= angle_term(vertices[start + u32(high)], u32(high), length, term) {
        return high + 1;
    }

    while (low <= high) {
        let mid = low + (high - low + 1) / 2;
        let val = angle_term(vertices[start + u32(mid)], u32(mid), length, term);

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

fn bs_vertex_reverse(angle: f32, start: u32, length: u32, term: u32) -> i32 {
    let light = lights[light_index];

    var ans = -1;
    
    var low = 0i; 
    var high = i32(length) - 1;

    if angle < angle_term(vertices[start - u32(low)], u32(low), length, term) {
        return -1;
    }

    if angle >= angle_term(vertices[start - u32(high)], u32(high), length, term) {
        return high + 1;
    }

    while (low <= high) {
        let mid = low + (high - low + 1) / 2;
        let val = angle_term(vertices[start - u32(mid)], u32(mid), length, term);

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

// checks if pixel is blocked by round occluder
// fn round_check(pos: vec2f, occluder: u32) -> OcclusionResult {
//     let light = lights[light_index];

//     let occ = round_occluders[occluder];
//     let half_w = occ.width * 0.5;
//     let half_h = occ.height * 0.5;
//     let radius = occ.radius;

//     let relative_pos = pos - occ.pos; 
//     let relative_light = light.pos - occ.pos; 

//     var rot = round_occluders[occluder].rot;
    
//     if (rot > PI2) {
//         rot = rot - PI2 * floor(rot / PI2);
//     }

//     let c = cos(occ.rot);
//     let s = sin(occ.rot);

//     let p_local = vec2f(relative_pos.x * c + relative_pos.y * s, -relative_pos.x * s + relative_pos.y * c);
//     let l_local = vec2f(relative_light.x * c + relative_light.y * s, -relative_light.x * s + relative_light.y * c);

//     var rect = vec4f(-(half_w + radius), -(half_h + radius), half_w + radius, half_h + radius);

//     var extreme_angle = 10.0;

//     if !rect_line_intersection(p_local, l_local, rect) {

//         if config.softness > 0 {
//             extreme_angle = get_round_extreme_angle(half_w, half_h, p_local, l_local, radius);
//         }

//         return OcclusionResult(false, extreme_angle);
//     }

//     if (occ.width > 0) {
//         // top edge
//         if intersects_axis_edge(p_local, l_local, half_h + radius, -half_w, half_w, false) {
//             return OcclusionResult(true, 0.0);
//         }

//         // bottom edge
//         if intersects_axis_edge(p_local, l_local, -(half_h + radius), -half_w, half_w, false) {
//             return OcclusionResult(true, 0.0);
//         }
//     }

//     if (occ.height > 0) {
//         // right edge
//         if intersects_axis_edge(p_local, l_local, half_w + radius, -half_h, half_h, true) {
//             return OcclusionResult(true, 0.0);
//         }

//         // left edge
//         if intersects_axis_edge(p_local, l_local, -(half_w + radius), -half_h, half_h, true) {
//             return OcclusionResult(true, 0.0);
//         }
//     }

//     if (radius > 0) {
//         let quadrants = array<vec2f, 4>(vec2f(1,1), vec2f(-1,1), vec2f(1,-1), vec2f(-1,-1));
//         let centers = array<vec2f, 4>(vec2f(half_w, half_h), vec2f(-half_w, half_h), vec2f(half_w, -half_h), vec2f(-half_w, -half_h));
//         for(var i = 0u; i < 4u; i++) {
//             if intersects_corner_arc(p_local, l_local, centers[i], radius, quadrants[i]) { 
//                 return OcclusionResult(true, 0.0); 
//             }
//         }
//     }

//     if config.softness > 0 {
//         extreme_angle = get_round_extreme_angle(half_w, half_h, p_local, l_local, radius);
//     }

//     return OcclusionResult(false, extreme_angle);
// }

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
