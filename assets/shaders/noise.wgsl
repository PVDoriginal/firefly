// Shader that receives the lightmap and applies noise over it. 
// The actual noise function is taken from: https://www.shadertoy.com/view/tldSRj. 

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var lightmap: texture_2d<f32>;

@group(0) @binding(1)
var texture_sampler: sampler;

@group(0) @binding(2)
var<uniform> settings: NoiseSettings;

struct NoiseSettings {
    time: f32, 
    kf: f32, 
}

@fragment
fn fragment(vo: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let light_frag = textureSample(lightmap, texture_sampler, vo.uv);

    var f = noise(24.0 * vo.uv + settings.time * 4.0);
    f = 0.5 + 0.5*f;

    // lightmap is multiplied by the noise
    return vec4<f32>(light_frag.xyz * f, 1.0);
}

fn noise(p: vec2<f32>) -> f32 {
    let kf = settings.kf;
    
    let i = floor(p);
	var f = fract(p);
    f = f*f*(3.0-2.0*f);
    return mix(mix(sin(kf*dot(p,g(i+vec2<f32>(0,0)))),
               	   sin(kf*dot(p,g(i+vec2<f32>(1,0)))),f.x),
               mix(sin(kf*dot(p,g(i+vec2<f32>(0,1)))),
               	   sin(kf*dot(p,g(i+vec2<f32>(1,1)))),f.x),f.y);
}
fn g(n: vec2<f32>) -> vec2<f32> { 
    return sin(n.x*n.y*vec2<f32>(12,17)+vec2<f32>(1,2)); 
}