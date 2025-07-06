// Texture array demo shader
#import global_bindings::get_time
#import constants as Constants

@group(1) @binding(0) var texture_array: binding_array<texture_2d<f32>, 2>;
@group(1) @binding(1) var sampler_array: binding_array<sampler, 2>;

struct Uniforms {
  color_rgb: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
  @location(0) position: vec3<f32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
  //A fullscreen triangle.
  var out: VertexOutput;
  out.clip_position = vec4(in.position.xyz, Constants::ONE);
  out.tex_coords = in.position.xy * 0.5 + 0.5;
  return out;
}

struct PushConstants {
    color_matrix: mat4x4<f32>
}

var<push_constant> push_constants: PushConstants;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_uv = in.tex_coords;
    let t = get_time() * 0.5;
    
    // Create animated UV coordinates for texture array sampling
    let uv1 = base_uv + vec2<f32>(sin(t) * 0.1, cos(t * 1.3) * 0.1);
    let uv2 = base_uv * (1.0 + 0.2 * sin(t * 0.7)) + vec2<f32>(cos(t * 0.8) * 0.05, sin(t * 1.1) * 0.05);
    
    // Sample from both textures in the array using different samplers
    let color1 = textureSample(texture_array[0], sampler_array[0], uv1).rgb;
    let color2 = textureSample(texture_array[1], sampler_array[1], uv2).rgb;
    
    // Create dynamic blend factor based on position and time
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(base_uv, center);
    let blend_factor = 0.5 + 0.5 * sin(t + dist * 8.0);
    
    // Blend the textures with a ripple effect
    let blended_color = mix(color1, color2, blend_factor);
    
    // Add time-based color modulation
    let color_mod = vec3<f32>(
        0.8 + 0.2 * sin(t),
        0.8 + 0.2 * sin(t + 2.0),
        0.8 + 0.2 * sin(t + 4.0)
    );
    
    // Apply gentler ripple effect for low-res displays
    let ripple = sin(dist * 10.0 - t * 2.5) * 0.25 + 0.75;
    
    // Combine all effects
    let final_color = blended_color * uniforms.color_rgb.rgb * color_mod * ripple;
    
    // Softer vignette for low-res displays
    let vignette = smoothstep(0.0, 0.9, 1.0 - dist * 1.3);
    
    // Apply the color matrix from push constants
    return push_constants.color_matrix * vec4(final_color * vignette, 1.0);
}