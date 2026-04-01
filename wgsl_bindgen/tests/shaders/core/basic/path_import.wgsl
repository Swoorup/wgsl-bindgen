import package::external::reachme::RtsStruct;

@group(0) @binding(0)
var<storage> rts: array<RtsStruct>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let v = rts[id.x].value;
}

