struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture_id: u32,
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_id: f32, // Use f32 for smooth interpolation
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 1.0);
    output.texture_id = f32(input.texture_id); // Convert u32 to f32 for interpolation
    return output;
}

@fragment  
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Classic RGB triangle gradient - convert interpolated texture_id back to RGB
    let id = input.texture_id;
    
    // Create smooth RGB gradient based on interpolated texture_id
    // id will be interpolated between 1.0, 2.0, and 3.0
    let r = clamp(2.0 - id, 0.0, 1.0);        // Red: full at id=1, fades to 0 at id=2
    let g = clamp(1.0 - abs(id - 2.0), 0.0, 1.0); // Green: full at id=2, fades at edges
    let b = clamp(id - 2.0, 0.0, 1.0);        // Blue: starts at id=2, full at id=3
    
    return vec4<f32>(r, g, b, 1.0);
}