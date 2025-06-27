struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Create a rectangle in the top-left corner
    let x = f32((vertex_index & 1u) * 2u);
    let y = f32((vertex_index >> 1u) * 2u);
    
    // Calculate effective scale with proper retina display handling
    let dpi_scale = info.scale_factor;
    
    // Use normalized scaling that works well across different DPI settings
    // Higher DPI displays don't need as much additional scaling since they're already sharp
    let dpi_adjusted_scale = mix(1.2, 0.9, clamp((dpi_scale - 1.0) / 2.0, 0.0, 1.0));
    
    // Only scale up for very large windows, never scale down for small windows  
    let min_dimension = min(info.window_width, info.window_height);
    let size_boost = max(1.0, min_dimension / 1200.0); // Conservative boost for large screens
    
    // Final effective scale that provides good readability without over-scaling on retina
    let effective_scale = dpi_adjusted_scale * size_boost;
    
    // Calculate overlay dimensions with generous text space
    let base_height = 0.5; // Larger base height for better text readability
    let overlay_height = min(0.8, base_height * effective_scale); // Allow larger overlay
    
    // Position in normalized device coordinates (-1 to 1)
    out.clip_position = vec4<f32>(
        x * 2.0 - 1.0,  // Full width
        1.0 - y * overlay_height,  // Adaptive height
        0.0,
        1.0
    );
    
    // Adjust texture coordinates to maintain proper text size and prevent stretching
    // Keep text at a consistent, readable size regardless of overlay scaling
    let text_scale_x = 1.0; // Keep full width for text
    let text_scale_y = min(1.0, 0.8 / effective_scale); // Scale to maintain readable text size
    out.tex_coords = vec2<f32>(x * text_scale_x, y * text_scale_y);
    
    return out;
}

struct InfoData {
    demo_index: f32,
    total_demos: f32,
    time: f32,
    scale_factor: f32,
    window_width: f32,
    window_height: f32,
    padding1: f32,
    padding2: f32,
}

@group(0) @binding(0) var<uniform> info: InfoData;
@group(0) @binding(1) var text_texture: texture_2d<f32>;
@group(0) @binding(2) var text_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the text texture with better filtering for crisp text on high DPI
    let text_color = textureSample(text_texture, text_sampler, in.tex_coords);
    
    // Dark semi-transparent background with gradient
    let bg_color = vec4<f32>(0.05, 0.05, 0.08, 0.9);
    
    // Vertical gradient effect
    let gradient = 1.0 - in.tex_coords.y * 0.5;
    
    // Add a subtle animated color shift based on demo index
    let color_shift = 0.1 * sin(info.time * 0.5 + info.demo_index * 3.14159);
    let tinted_bg = bg_color + vec4<f32>(color_shift * 0.2, color_shift * 0.1, color_shift * 0.3, 0.0);
    
    // Mix background and text
    let final_color = mix(tinted_bg * gradient, vec4<f32>(1.0, 1.0, 1.0, 1.0), text_color.a);
    
    return final_color;
}