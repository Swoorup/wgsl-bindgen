// Texture array demo shader
import package::constants::ONE;
import package::global_bindings::get_time;
import package::global_bindings::globals;

@group(1) @binding(0) var ms_texture: texture_multisampled_2d<f32>;

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
  // A fullscreen triangle/quad setup (assuming the host gives coords in -1..1 range)
  var out: VertexOutput;
  out.clip_position = vec4(position.xyz, ONE);
  out.tex_coords = position.xy * 0.5 + 0.5;
  // flip y for vulkan/wgpu coordinate system
  out.tex_coords.y = 1.0 - out.tex_coords.y;
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims = textureDimensions(ms_texture);
    let coords = vec2<i32>(in.tex_coords * vec2<f32>(dims));
    
    // Average all 4 samples to do a manual resolve
    var color = vec4<f32>(0.0);
    for (var i = 0; i < 4; i++) {
        color += textureLoad(ms_texture, coords, i);
    }
    
    // Add a pulsing effect using global time
    let pulse = 0.8 + 0.2 * sin(get_time() * 2.0);
    return color * 0.25 * pulse;
}

// --- Shader for rendering shapes INTO the MSAA texture ---

struct ShapeOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_msaa(@builtin(vertex_index) vi: u32) -> ShapeOutput {
    // Generate 3 vertices for a spinning triangle
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.8),
        vec2<f32>(-0.8, -0.8),
        vec2<f32>(0.8, -0.8)
    );
    
    var colors = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.0, 0.5),
        vec3<f32>(0.0, 1.0, 0.5),
        vec3<f32>(1.0, 1.0, 0.0)
    );

    let t = get_time();
    let c = cos(t);
    let s = sin(t);
    // Simple 2D rotation matrix
    let rot = mat2x2<f32>(
        vec2<f32>(c, s),
        vec2<f32>(-s, c)
    );
    
    // Also move it with mouse!
    let offset = globals.mouse_pos;
    
    var out: ShapeOutput;
    out.clip_position = vec4<f32>(rot * positions[vi] * 0.5 + offset, 0.0, 1.0);
    out.color = colors[vi];
    return out;
}

@fragment
fn fs_msaa(in: ShapeOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
