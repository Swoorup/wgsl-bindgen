struct RtsStruct {
  other_data : i32,
  the_array : array<u32>,
};

@group(0) @binding(3)
var<storage, read_write> rts : RtsStruct;

struct Scalars {
  a : u32,
  b : i32,
  c : f32,
};
var<uniform> a : Scalars;

struct VectorsU32 {
  a : vec2 < u32>,
  b : vec3 < u32>,
  c : vec4 < u32>,
};
var<uniform> b : VectorsU32;

struct VectorsI32 {
  a : vec2 < i32>,
  b : vec3 < i32>,
  c : vec4 < i32>,
};
var<uniform> c : VectorsI32;

struct VectorsF32 {
  a : vec2 < f32>,
  b : vec3 < f32>,
  c : vec4 < f32>,
};
var<uniform> d : VectorsF32;

struct MatricesF32 {
  a : mat4x4 < f32>,
  b : mat4x3 < f32>,
  c : mat4x2 < f32>,
  d : mat3x4 < f32>,
  e : mat3x3 < f32>,
  f : mat3x2 < f32>,
  g : mat2x4 < f32>,
  h : mat2x3 < f32>,
  i : mat2x2 < f32>,
};
var<uniform> f : MatricesF32;

struct VertexInput {
  @location(0) position : vec3 < f32>,
};

struct VertexOutput {
  @builtin(position) clip_position : vec4 < f32>,
  @location(0) tex_coords : vec2 < f32>
};

@vertex
fn vs_main(in : VertexInput) -> VertexOutput {
    //A fullscreen triangle.
  var out : VertexOutput;
  out.clip_position = vec4(in.position.xyz, 1.0);
  out.tex_coords = in.position.xy * 0.5 + 0.5;
  return out;
}

@group(0) @binding(0)
var color_texture : texture_2d<f32>;
@group(0) @binding(1)
var color_sampler : sampler;

struct Uniforms {
  color_rgb : vec4 < f32>,
}

@group(1) @binding(0)
var<uniform> uniforms : Uniforms;

struct ScalingModeData {
  kind : i32,
  //log base 10
  logical_offset : f32,
  coord_offset : f32,
  //percentage
  base_value : f32,
}

struct UniformsData {
  x_scaling_mode : ScalingModeData,
  y_scaling_mode : ScalingModeData,
  logical_view_matrix : mat2x2 < f32>,
  logical_space_center_point : vec2 < f32>,
}

var<uniform> g : UniformsData;

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4 < f32> {
  let color = textureSample(color_texture, color_sampler, in.tex_coords).rgb;
  return vec4(color * uniforms.color_rgb.rgb, 1.0);
}
