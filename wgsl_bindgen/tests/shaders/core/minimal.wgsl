struct Uniforms {
    color: vec4f,
    width: f32,
}

@group(0) @binding(0)
var<uniform> uniform_buf: Uniforms;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
}
