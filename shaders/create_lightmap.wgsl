#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

#import firefly::types::{PointLight, LightingData}
#import firefly::utils::{ndc_to_world, frag_coord_to_ndc}

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

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pos = ndc_to_world(frag_coord_to_ndc(in.position.xy));
    var res = vec4<f32>(0, 0, 0, 1);

    let light = light.pos; 
    let dist = distance(pos, light) + 0.01; 

    if (dist < 100.) {
        let x = dist / 100.;
        res += vec4<f32>(vec3<f32>(1. - x * x), 0);
    }

    res += textureSample(lightmap_texture, texture_sampler, in.uv);

    return min(res, vec4<f32>(1, 1, 1, 1));
}