struct Style {
    color: vec4f,
    width: f32,
    _padding: vec2<f32>
}

@group(0) @binding(0)
var<storage> frame: Style;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
}
