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

fn blend(bg: vec4f, fg: vec4f, intensity: f32) -> vec4f {
    return max(fg * intensity, bg);
}


fn shadow_blend(bg: vec3f, fg: vec3f, opacity: f32) -> vec3f {
    return bg * min(vec3f(1), (vec3f(2) - (vec3f(1) - fg)) * (1 - opacity));
}

// check if the [a, b] segment intersects the circle (c, r) between the 2 angles 
fn intersects_arc(a: vec2f, b: vec2f, c: vec2f, r: f32, start_angle: f32, end_angle: f32) -> bool {
    let a2 = a - c;
    let b2 = b - c;

    let dx = b2.x - a2.x; 
    let dy = b2.y - a2.y; 
    let dr_sqr = dx * dx + dy * dy;

    let det = a2.x * b2.y - a2.y * b2.x;
    let delta = r * r * dr_sqr - det * det;

    if (delta < 0) {
        return false;
    }

    let new_delta = sqrt(delta);

    let x1 = (det * dy + sgn(dy) * dx * new_delta) / dr_sqr;
    let y1 = (-det * dx + abs(dy) * new_delta) / dr_sqr;

    let dt1 = dot(b2 - a2, vec2f(x1, y1) - a2);
    if dt1 >= 0 && dt1 < dr_sqr { 
        let angle = atan2(y1, x1);
        if (between_arctan(angle, start_angle, end_angle)) {
            return true;
        }
    }  

    if (delta == 0) {
        return false;
    }

    let x2 = (det * dy - sgn(dy) * dx * new_delta) / dr_sqr;
    let y2 = (-det * dx - abs(dy) * new_delta) / dr_sqr;


    let dt2 = dot(b2 - a2, vec2f(x2, y2) - a2);
    if dt2 >= 0 && dt2 < dr_sqr { 
        let angle = atan2(y2, x2);
        if (between_arctan(angle, start_angle, end_angle)) {
            return true;
        }  
    }
    
    return false;
}

// sign of float
fn sgn(x: f32) -> f32 {
    if (x < 0) {
        return -1.0;
    } 
    return 1.0;
}

fn rotate(p: vec2f, r: vec2f) -> vec2f {
    return vec2f(p.x * r.x - p.y * r.y, p.x * r.y + p.y * r.x);
}

const PI2: f32 = 6.28318530718;
const PI: f32 = 3.14159265359;
const PIDIV2: f32 = 1.57079632679; 

// checks if x is between a and b, all being arctan angles 
fn between_arctan(x: f32, a: f32, b: f32) -> bool {
    return (x > a && x < b) || (x > PIDIV2 && b < -PIDIV2 && x > a && x - PI2 < b) || (x < -PIDIV2 && a > PIDIV2 && x + PI2 > a && x < b);
}

// rotate an arctan angle 
fn rotate_arctan(x: f32, r: f32) -> f32 {
    var res = x + r; 
    if (res > PI) {
        return res - PI2; 
    }
    if (res < -PI) {
        return res + PI2; 
    }
    return res;
}

// distance from p to line [a, b]
fn distance_point_to_line(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    return abs((b.y - a.y) * p.x - (b.x - a.x) * p.y + b.x * a.y - b.y * a.x) / distance(a, b);
}

// get intersection point of [a, b] and [c, d]
fn intersection_point(a: vec2f, b: vec2f, c: vec2f, d: vec2f) -> vec2f {
    let denom = (a.x - b.x) * (c.y - d.y) - (a.y - b.y) * (c.x - d.x);
    if denom == 0. {
        return vec2f(0);
    }

    let r1 = a.x * b.y - a.y * b.x;
    let r2 = c.x * d.y - c.y * d.x;

    return vec2f(
        (r1 * (c.x - d.x) - (a.x - b.x) * r2) / denom, 
        (r1 * (c.y - d.y) - (a.y - b.y) * r2) / denom
    );
}

fn rect_intersection(r1: vec4f, r2: vec4f) -> bool {
    return !(
        r2.x > r1.z ||
        r2.z < r1.x || 
        r2.w > r1.y || 
        r2.y < r1.w
    );
}

// Liang-Barsky algorithm for line clipping
fn rect_line_intersection(a: vec2f, b: vec2f, rect: vec4f) -> bool {
    let d = b - a;
    
    var t_min = 0.0f;
    var t_max = 1.0f;

    if (abs(d.x) < 1e-6) {
        if (a.x < rect.x || a.x > rect.z) { return false; }
    } else {
        let inv_dx = 1.0 / d.x;
        var t1 = (rect.x - a.x) * inv_dx;
        var t2 = (rect.z - a.x) * inv_dx;
        
        t_min = max(t_min, min(t1, t2));
        t_max = min(t_max, max(t1, t2));
    }

    if (abs(d.y) < 1e-6) {
        if (a.y < rect.y || a.y > rect.w) { return false; }
    } else {
        let inv_dy = 1.0 / d.y;
        var t1 = (rect.y - a.y) * inv_dy;
        var t2 = (rect.w - a.y) * inv_dy;
        
        t_min = max(t_min, min(t1, t2));
        t_max = min(t_max, max(t1, t2));
    }

    return t_min <= t_max;
}