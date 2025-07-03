#import global_bindings::{get_time, get_frame_size}

struct VertexInput {
    @location(0) quad_pos: vec2<f32>,           // Quad vertex position (-1 to 1)
    @location(1) position_and_size: vec4<f32>,  // xyz = particle position, w = particle_type + energy
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) energy: f32,
    @location(2) particle_type: f32,
    @location(3) world_position: vec2<f32>, // World position for fragment calculations
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Get frame size for proper aspect ratio
    let frame = get_frame_size();
    let aspect_ratio = frame.x / frame.y;
    
    // Extract particle data
    let particle_world_pos = input.position_and_size.xyz;
    let particle_data = input.position_and_size.w;
    let particle_type = floor(particle_data) % 4.0;
    let energy = fract(particle_data) * 10.0;
    
    // Calculate particle size in pixels, then convert to screen space
    var particle_pixels = 8.0; // Base size in pixels
    if particle_type > 90.0 {
        particle_pixels = 50.0; // DEBUG particle - huge and obvious
    } else if particle_type < 0.5 {
        particle_pixels = 10.0; // Flock particles
    } else if particle_type < 1.5 {
        particle_pixels = 9.0; // Wanderer particles  
    } else if particle_type < 2.5 {
        particle_pixels = 12.0; // Orbiter particles
    } else {
        particle_pixels = 14.0; // Repulser particles
    }
    
    // Size variation based on energy
    particle_pixels *= (0.8 + energy * 0.4);
    
    // Convert pixels to normalized screen coordinates
    // In normalized screen space, the full height spans 2.0 units (-1 to 1)
    let particle_size = particle_pixels * 2.0 / frame.y;
    
    // If particles are in screen space, use them directly
    let screen_center = vec2<f32>(
        particle_world_pos.x,  // Direct screen space
        particle_world_pos.y
    );
    
    // Apply quad offset to create billboard, accounting for aspect ratio to keep particles round
    let quad_offset = vec2<f32>(
        input.quad_pos.x * particle_size / aspect_ratio,  // Compress X to counteract stretch
        input.quad_pos.y * particle_size
    );
    let billboard_pos = screen_center + quad_offset;
    
    output.position = vec4<f32>(billboard_pos, 0.0, 1.0);
    
    // Store world position and quad coordinate for fragment shader
    output.world_position = input.quad_pos; // Use quad pos for distance calculation
    
    // Set output values
    output.particle_type = particle_type;
    output.energy = energy;
    
    // Color coding based on particle type - muted, sophisticated palette
    if particle_type > 90.0 {
        // DEBUG particle - muted red
        output.color = vec3<f32>(0.7, 0.2, 0.2);
    } else if particle_type < 0.5 {
        // Type 0: Flock particles - Deep blue to teal
        output.color = vec3<f32>(0.15, 0.4 + energy * 0.25, 0.6 + energy * 0.2);
    } else if particle_type < 1.5 {
        // Type 1: Wanderer particles - Forest green to olive
        output.color = vec3<f32>(0.3 + energy * 0.2, 0.5 + energy * 0.3, 0.2 + energy * 0.15);
    } else if particle_type < 2.5 {
        // Type 2: Orbiter particles - Warm amber to burnt orange
        output.color = vec3<f32>(0.6 + energy * 0.2, 0.35 + energy * 0.25, 0.15 + energy * 0.1);
    } else {
        // Type 3: Repulser particles - Deep purple to plum
        output.color = vec3<f32>(0.4 + energy * 0.2, 0.25 + energy * 0.15, 0.5 + energy * 0.3);
    }
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Get frame info and time
    let frame = get_frame_size();
    let time = get_time();
    
    // Calculate distance from center of particle using quad coordinates
    let dist_from_center = length(input.world_position);
    
    // Create sharp circular particle
    let particle_radius = 0.8; // Radius in quad space (-1 to 1)
    
    // Simple sharp circle - discard pixels outside radius
    if dist_from_center > particle_radius {
        discard;
    }
    
    // Create a hard edge circle with mild anti-aliasing
    let edge_width = 0.08; // Increased anti-aliasing for smoother, professional edges
    let intensity = 1.0 - smoothstep(particle_radius - edge_width, particle_radius, dist_from_center);
    
    // Stable per-particle seed for consistent effects
    let particle_id = input.particle_type * 1000.0 + input.energy * 100.0;
    let stable_seed = fract(sin(particle_id * 12.9898) * 43758.5453);
    
    // Gentle time-based pulsing effect
    let slow_time = time * 0.6;
    let pulse = 0.85 + 0.15 * sin(slow_time * (1.0 + input.particle_type * 0.3) + stable_seed * 6.28);
    
    // Apply color with pulsing
    var final_color = input.color * intensity * pulse;
    
    // Add glow effect - brighter center with soft falloff
    let glow_radius = particle_radius * 0.6;
    let core_glow = 1.0 - smoothstep(0.0, glow_radius, dist_from_center);
    let edge_glow = 0.3 * (1.0 - smoothstep(glow_radius, particle_radius, dist_from_center));
    let total_glow = core_glow + edge_glow;
    
    // Apply glow to color
    final_color *= (0.8 + total_glow * 0.4);
    
    let alpha = intensity;
    
    return vec4<f32>(final_color, alpha);
}