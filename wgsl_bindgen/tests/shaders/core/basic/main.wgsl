#import bindings;
#import types::{Fp64};

struct Style {
    color: vec4f,
    width: f32,
}

@group(0) @binding(0)
var<storage, read_write> buffer: array<f32>;

@group(0) @binding(1)
var texture_float: texture_2d<f32>;

@group(0) @binding(2)
var texture_sint: texture_2d<i32>;

@group(0) @binding(3)
var texture_uint: texture_2d<u32>;

@group(0) @binding(4)
var texture_array_float: texture_2d_array<f32>;

@group(0) @binding(5)
var texture_array_sint: texture_2d_array<i32>;

@group(0) @binding(6)
var texture_array_uint: texture_2d_array<u32>;

var<push_constant> const_style: Style;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    buffer[id.x] *= 2 * bindings::ONE * const_style.color.a * const_style.width;
}
