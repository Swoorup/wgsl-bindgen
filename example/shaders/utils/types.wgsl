struct Scalars {
  a: u32,
  b: i32,
  c: f32,
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

struct VertexIn {
    @location(0) position: vec4<f32>,
}