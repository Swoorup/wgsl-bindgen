// Test shader using WESL import syntax and conditional translation

import package::shared::apply_scale;

struct UniformsBase {
    color: vec4<f32>,
    scale: f32,
}

@group(0) @binding(0) var<uniform> uniforms: UniformsBase;

@if(USE_TEXTURE)
@group(0) @binding(1) var test_texture: texture_2d<f32>;
@if(USE_TEXTURE)
@group(0) @binding(2) var test_sampler: sampler;

@vertex
fn vs_main() -> @builtin(position) vec4<f32> {
    var pos = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    pos = apply_scale(pos, uniforms.scale);
    return pos;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return uniforms.color;
}
