#import shared_data::{
  shared_uniforms, 
  vertex_data, 
  shared_texture,
  compute_uniforms,
  output_data
};
// Note: shader_b imports compute_uniforms and output_data from group 1, NOT dynamic_data
// This creates a scenario where group 1 has mixed usage across shaders

@compute @workgroup_size(1)
fn cs_main() {
    let matrix = shared_uniforms.view_matrix;
    let current_time = shared_uniforms.time;
    let vertex_count = arrayLength(&vertex_data);
    
    // This shader uses different bindings from group 1 than shader_a
    // It uses compute_uniforms (binding 1) and output_data (binding 2)
    let scale_factor = compute_uniforms.x;
    let iterations = u32(compute_uniforms.y);
    
    if vertex_count > 0u && iterations > 0u {
        let first_vertex = vertex_data[0];
        let tex_dims = textureDimensions(shared_texture);
        
        // Write to output buffer to demonstrate read_write storage usage
        if arrayLength(&output_data) > 0u {
            output_data[0] = scale_factor * f32(tex_dims.x);
            if arrayLength(&output_data) > 1u {
                output_data[1] = first_vertex.position.x;
            }
        }
    }
}