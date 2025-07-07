// Test shader with conditional compilation using shader_defs

struct Uniforms {
    color: vec4<f32>,
#ifdef USE_TIME
    time: f32,
#endif
#ifdef USE_SCALE
    scale: f32,
#endif
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

#ifdef USE_TEXTURE
@group(0) @binding(1) var test_texture: texture_2d<f32>;
@group(0) @binding(2) var test_sampler: sampler;
#endif

@vertex
fn vs_main() -> @builtin(position) vec4<f32> {
    var pos = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    
#ifdef USE_SCALE
    pos = pos * uniforms.scale;
#endif

#ifdef USE_TIME
    pos.x = pos.x + sin(uniforms.time);
#endif

    return pos;
}

@fragment  
fn fs_main() -> @location(0) vec4<f32> {
    var color = uniforms.color;
    
#ifdef USE_TEXTURE
    let tex_color = textureSample(test_texture, test_sampler, vec2<f32>(0.5, 0.5));
    color = color * tex_color;
#endif

#ifdef DEBUG_MODE
    // Debug: make everything red
    color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
#endif

    return color;
}