// Global reusable bindings for all shaders
// Group 0 is reserved for global/common resources

struct GlobalUniforms {
    time: f32,           // Time since startup in seconds
    scale_factor: f32,   // UI scale factor for high-DPI displays
    frame_size: vec2<f32>, // Frame width and height in pixels
    mouse_pos: vec2<f32>,  // Mouse position in screen coordinates (-1 to 1)
}

/// Global uniforms available to all shaders
@group(0) @binding(0) var<uniform> globals: GlobalUniforms;

// Convenience accessor functions for backward compatibility
fn get_time() -> f32 { return globals.time; }
fn get_scale_factor() -> f32 { return globals.scale_factor; }
fn get_frame_size() -> vec2<f32> { return globals.frame_size; }
fn get_mouse_pos() -> vec2<f32> { return globals.mouse_pos; }