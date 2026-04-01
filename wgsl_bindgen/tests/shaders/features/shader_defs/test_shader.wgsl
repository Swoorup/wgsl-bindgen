// Test shader with WESL conditional translation using @if feature attributes

@if(!USE_TIME && !USE_SCALE)
struct UniformsBase {
    color: vec4<f32>,
}

@if(!USE_TIME && USE_SCALE)
struct UniformsScaleOnly {
    color: vec4<f32>,
    scale: f32,
}

@if(USE_TIME && !USE_SCALE)
struct UniformsTimeOnly {
    color: vec4<f32>,
    time: f32,
}

@if(USE_TIME && USE_SCALE)
struct UniformsFull {
    color: vec4<f32>,
    time: f32,
    scale: f32,
}

@if(!USE_TIME && !USE_SCALE)
@group(0) @binding(0) var<uniform> uniforms: UniformsBase;

@if(!USE_TIME && USE_SCALE)
@group(0) @binding(0) var<uniform> uniforms: UniformsScaleOnly;

@if(USE_TIME && !USE_SCALE)
@group(0) @binding(0) var<uniform> uniforms: UniformsTimeOnly;

@if(USE_TIME && USE_SCALE)
@group(0) @binding(0) var<uniform> uniforms: UniformsFull;

@if(USE_TEXTURE)
@group(0) @binding(1) var test_texture: texture_2d<f32>;

@if(USE_TEXTURE)
@group(0) @binding(2) var test_sampler: sampler;

@vertex
fn vs_main() -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return uniforms.color;
}
