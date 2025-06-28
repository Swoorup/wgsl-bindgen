struct VertexInput {
    @location(0) position: vec3<f32>,
    @builtin(vertex_index) vertex_index: u32,
}

struct RegularStruct {
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    return vec4<f32>(input.position, 1.0);
}