#import utils::types::{
  Scalars, 
  VectorsU32, 
  VectorsI32, 
  VectorsF32, 
  MatricesF32, 
  StaticArrays, 
  Nested,
  VertexIn
}

@group(2) @binding(2)
var<storage> a: Scalars;

@group(2) @binding(3)
var<storage> b: VectorsU32;

@group(2) @binding(4)
var<storage> c: VectorsI32;

@group(2) @binding(5)
var<storage> d: VectorsF32;

@group(2) @binding(6)
var<storage> f: MatricesF32;

@group(2) @binding(8)
var<storage> h: StaticArrays;

@group(2) @binding(9)
var<storage> i: Nested;

@group(0) @binding(0)
var color_texture: texture_2d<f32>;
@group(0) @binding(1)
var color_sampler: sampler;

struct Uniforms {
  color_rgb: vec4<f32>,
  scalars: Scalars
}

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

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