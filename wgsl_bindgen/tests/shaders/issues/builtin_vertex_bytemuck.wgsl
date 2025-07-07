struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}