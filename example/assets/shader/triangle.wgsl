@group(0) @binding(0)
var color_texture: texture_2d<f32>;
@group(0) @binding(1)
var color_sampler: sampler;

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

// wgsl outputs with pipeline constants are not supported.
// https://github.com/gfx-rs/wgpu/blob/abba12ae4e5488b08d9e189fc37dab5e1755b443/naga/src/back/wgsl/writer.rs#L108-L113
// override force_black: bool;
// override scale: f32 = 1.0;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let color = textureSample(color_texture, color_sampler, in.tex_coords).rgb;
  // if force_black {
  //   return vec4(0.0);
  // } else {
  //   return vec4(color * uniforms.color_rgb.rgb * scale, 1.0);
  // }
  return vec4(color * uniforms.color_rgb.rgb, 1.0);
}
