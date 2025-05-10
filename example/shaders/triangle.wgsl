#import global_bindings::{
  color_texture, 
  color_sampler, 
  time
}

struct Uniforms {
  color_rgb: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
  @location(0) position: vec3<f32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) tex_coords: vec2<f32>,
  #ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
  #endif
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
  //A fullscreen triangle.
  var out: VertexOutput;
  out.clip_position = vec4(in.position.xyz, 1.0);
  out.tex_coords = in.position.xy * 0.5 + 0.5;
  return out;
}

struct PushConstants {
    color_matrix: mat4x4<f32>
}

var<push_constant> constants: PushConstants;

// wgsl outputs with pipeline constants are not supported.
// https://github.com/gfx-rs/wgpu/blob/abba12ae4e5488b08d9e189fc37dab5e1755b443/naga/src/back/wgsl/writer.rs#L108-L113
// override force_black: bool;
// override scale: f32 = 1.0;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let uv = in.tex_coords;
  let color = textureSample(color_texture, color_sampler, uv).rgb;
  
  // Simple time variable
  let t = time * 0.5;
  
  // Create a simple ripple effect from the center
  let center = vec2<f32>(0.5, 0.5);
  let dist = distance(uv, center);
  let ripple = sin(dist * 15.0 - t * 2.0) * 0.5 + 0.5;
  
  // Simple color cycling
  let color_shift = vec3<f32>(
    0.5 + 0.5 * sin(t),
    0.5 + 0.5 * sin(t + 2.0),
    0.5 + 0.5 * sin(t + 4.0)
  );
  
  // Simple vignette effect
  let vignette = smoothstep(0.0, 0.7, 1.0 - dist * 1.5);
  
  // Combine effects
  let final_color = color * uniforms.color_rgb.rgb * color_shift * (0.7 + 0.3 * ripple) * vignette;
  
  // Apply the color matrix from push constants
  return constants.color_matrix * vec4(final_color, 1.0);
}