// Simple texture array for testing
@group(0) @binding(0) var texture_array: binding_array<texture_2d<f32>, 2>;
@group(0) @binding(1) var sampler_array: binding_array<sampler, 2>;

/// The time since startup in seconds.
@group(0) @binding(2) var<uniform> time: f32;