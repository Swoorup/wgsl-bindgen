import package::shared_data::{shared_uniforms, vertex_data, shared_texture, compute_uniforms, output_data};

@compute @workgroup_size(1)
fn cs_main() {
    let matrix = shared_uniforms.view_matrix;
    let current_time = shared_uniforms.time;
    let vertex_count = arrayLength(&vertex_data);

    let scale_factor = compute_uniforms.x;
    let iterations = u32(compute_uniforms.y);

    if vertex_count > 0u && iterations > 0u {
        let first_vertex = vertex_data[0];
        let tex_dims = textureDimensions(shared_texture);

        if arrayLength(&output_data) > 0u {
            output_data[0] = scale_factor * f32(tex_dims.x);
            if arrayLength(&output_data) > 1u {
                output_data[1] = first_vertex.position.x;
            }
        }
    }
}
