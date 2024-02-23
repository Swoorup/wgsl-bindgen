#import "../../../example/src/more-shader-files/reachme" as reachme 

@group(0) @binding(0)
var<storage> rts: array<reachme::RtsStruct>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    // buffer[id.x] *= 2 * other::ONE;
}

