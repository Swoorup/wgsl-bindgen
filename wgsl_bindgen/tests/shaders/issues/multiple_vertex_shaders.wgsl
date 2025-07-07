struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct InstanceInput {
    @location(1) position: vec3<f32>,
}

@vertex
fn dummy_vertex_shader(vert_in: VertexInput) -> @builtin(position) vec4<f32> {
    return vec4<f32>(vert_in.position, 1.0);
}

@vertex
fn dummy_instanced_vertex_shader(vert_in: VertexInput, instance_in: InstanceInput) -> @builtin(position) vec4<f32> {
    return vec4<f32>(vert_in.position + instance_in.position, 1.0);
}