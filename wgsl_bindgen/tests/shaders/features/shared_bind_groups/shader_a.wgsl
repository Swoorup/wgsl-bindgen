#import shared_data::{
  shared_uniforms, 
  vertex_data, 
  shared_texture, 
  shared_sampler, 
  dynamic_data
};
// Note: shader_a only imports dynamic_data from group 1, NOT compute_settings or output_buffer

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let transform = shared_uniforms.view_matrix;
    let vertex = vertex_data[vertex_index % arrayLength(&vertex_data)];
    
    // This shader only uses dynamic_data from group 1 (binding 0)
    let dynamic_len = arrayLength(&dynamic_data);
    if dynamic_len > 0u {
        let dynamic_array = dynamic_data[0];
        return vec4<f32>(vertex.position + vec3<f32>(dynamic_array[0]), 1.0);
    }
    return vec4<f32>(vertex.position, 1.0);
}

@fragment  
fn fs_main() -> @location(0) vec4<f32> {
    let time_factor = shared_uniforms.time;
    let tex_color = textureSample(shared_texture, shared_sampler, vec2<f32>(0.5, 0.5));
    return vec4<f32>(1.0, 0.0, 0.0, 1.0) * tex_color;
}