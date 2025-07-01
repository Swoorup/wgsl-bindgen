// Global reusable bindings for all shaders
// Group 0 is reserved for global/common resources

/// The time since startup in seconds - available to all shaders
@group(0) @binding(0) var<uniform> time: f32;