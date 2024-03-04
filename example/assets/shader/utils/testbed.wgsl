#import "../../more-shader-files/reachme" as reachme 
#import types::{Scalars, VectorsU32, VectorsI32, VectorsF32, MatricesF32, StaticArrays, Nested}

// The following also works
// #import "../more-shader-files/reachme.wgsl" as reachme

@group(2) @binding(1)
// TODO: Fix this, I think the bug is in naga_oil.
// var<storage> rts: array<reachme::rtsStruct>;
var<storage> rts: reachme::rtsStruct;

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

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) { }

