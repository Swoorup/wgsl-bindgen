struct Scalars {
  a: u32,
  b: i32,
  c: f32,
  @builtin(vertex_index) d: u32,
};

struct VectorsU32 {
  a: vec2<u32>,
  b: vec3<u32>,
  c: vec4<u32>,
  _padding: f32,
};

struct VectorsI32 {
  a: vec2<i32>,
  b: vec3<i32>,
  c: vec4<i32>,
};

struct VectorsF32 {
  a: vec2<f32>,
  b: vec3<f32>,
  c: vec4<f32>,
};

struct MatricesF32 {
  a: mat4x4<f32>,
  b: mat4x3<f32>,
  c: mat4x2<f32>,
  d: mat3x4<f32>,
  e: mat3x3<f32>,
  f: mat3x2<f32>,
  g: mat2x4<f32>,
  h: mat2x3<f32>,
  i: mat2x2<f32>,
};

struct StaticArrays {
  a: array<u32, 5>,
  b: array<f32, 3>,
  c: array<mat4x4<f32>, 512>,
  d: array<vec3<f32>, 4>
};

struct Nested {
  a: MatricesF32,
  b: VectorsF32
};

struct Uniforms {
  color_rgb: vec4<f32>,
  scalars: Scalars
}

@group(0) @binding(0) var color_texture: texture_2d<f32>;
@group(0) @binding(1) var color_sampler: sampler;

@group(1) @binding(0) var<uniform> uniforms: Uniforms;

@group(2) @binding(2) var<storage> a: Scalars;
@group(2) @binding(3) var<storage> b: VectorsU32;
@group(2) @binding(4) var<storage> c: VectorsI32;
@group(2) @binding(5) var<storage> d: VectorsF32;
@group(2) @binding(6) var<storage> f: MatricesF32;
@group(2) @binding(8) var<storage> h: StaticArrays;
@group(2) @binding(9) var<storage> i: Nested;

struct VertexIn {
    @location(0) position: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex_main(input: VertexIn) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 1.0, 1.0);
}