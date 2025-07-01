struct VertexInput {
    @location(0) position_and_size: vec4<f32>, // xyz = position, w = size
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    output.position = vec4<f32>(input.position_and_size.xyz, 1.0);
    
    // Color based on position for visual variety
    let pos = input.position_and_size.xyz;
    output.color = vec3<f32>(
        0.5 + 0.5 * sin(pos.x * 3.0),
        0.5 + 0.5 * sin(pos.y * 3.0 + 2.0),
        0.5 + 0.5 * sin((pos.x + pos.y) * 2.0 + 4.0)
    );
    
    // Size is in the w component but we don't use point_size on all platforms
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple colored points - each particle gets a nice color
    // Add some pulsing effect based on position
    let pulse = 0.8 + 0.2 * sin(length(input.position.xy) * 5.0);
    let final_color = input.color * pulse;
    
    return vec4<f32>(final_color, 0.9);
}