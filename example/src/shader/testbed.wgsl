#import types

@group(2) @binding(1)
var<storage> rts: types::RtsStruct;

@group(2) @binding(2)
var<storage> a: types::Scalars;

@group(2) @binding(3)
var<storage> b: types::VectorsU32;

@group(2) @binding(4)
var<storage> c: types::VectorsI32;

@group(2) @binding(5)
var<storage> d: types::VectorsF32;

@group(2) @binding(6)
var<storage> f: types::MatricesF32;

@group(2) @binding(8)
var<storage> h: types::StaticArrays;

@group(2) @binding(9)
var<storage> i: types::Nested;

@group(0) @binding(0)
var color_texture: texture_2d<f32>;
@group(0) @binding(1)
var color_sampler: sampler;

struct Uniforms {
  color_rgb: vec4<f32>,
  scalars: types::Scalars
}

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
  @location(0) position: vec3<f32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) tex_coords: vec2<f32>
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    //A fullscreen triangle.
  var out: VertexOutput;
  out.clip_position = vec4(in.position.xyz, 1.0);
  out.tex_coords = in.position.xy * 0.5 + 0.5;
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let color = textureSample(color_texture, color_sampler, in.tex_coords).rgb;
  return vec4(color * uniforms.color_rgb.rgb, 1.0);
}
