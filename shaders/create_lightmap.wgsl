#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

#import firefly::types::{PointLight, LightingData}
#import firefly::utils::{ndc_to_world, frag_coord_to_ndc}

@group(0) @binding(0)
var<uniform> view: View;

@group(0) @binding(1) 
var<uniform> data: LightingData;

@group(0) @binding(2)
var<storage> lights: array<PointLight>;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pos = ndc_to_world(frag_coord_to_ndc(in.position.xy));
    var res = vec4<f32>(0, 0, 0, 1);

    for (var i = 0u; i < data.n_lights; i++) {
        let light = lights[i].pos; 
        let dist = distance(pos, light) + 0.01; 

        if (dist < 100.) {
            let x = dist / 100.;
            res += vec4<f32>(vec3<f32>(1. - x * x), 0);
        }
    }
    
    return min(res, vec4<f32>(1, 1, 1, 1));
}
