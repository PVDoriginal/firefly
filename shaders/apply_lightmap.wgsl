#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import firefly::types::FireflyConfig

#import firefly::utils::blend

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;

@group(0) @binding(1)
var light_map_texture: texture_2d<f32>;

@group(0) @binding(2)
var texture_sampler: sampler;

@group(0) @binding(3)
var<uniform> config: FireflyConfig;

@fragment
fn fragment(vo: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var light_frag = blend(textureSample(light_map_texture, texture_sampler, vo.uv), vec4f(config.ambient_color, 0), config.ambient_brightness);

    if config.light_bands > 0 {
        light_frag = floor(light_frag / vec4f(config.light_bands)) * config.light_bands + config.light_bands * 0.5;
    }

    // return light_frag;
    let scene_frag = textureSample(screen_texture, texture_sampler, vo.uv);
    return scene_frag * light_frag;
}
