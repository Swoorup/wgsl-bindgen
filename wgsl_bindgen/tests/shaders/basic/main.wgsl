#import bindings;
#import types::{Fp64};

@group(0) @binding(0)
var<storage, read_write> buffer: array<f32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    buffer[id.x] *= 2 * bindings::ONE;
}