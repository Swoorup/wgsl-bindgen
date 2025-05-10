@group(0) @binding(0) var color_texture: texture_2d<f32>;
@group(0) @binding(1) var color_sampler: sampler;

/// The time since startup in seconds.
/// Wraps to 0 after 1 hour.
@group(0) @binding(2) var<uniform> time: f32;
