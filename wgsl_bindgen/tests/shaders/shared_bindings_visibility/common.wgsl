// Common bindings shared between compute and render shaders
@group(0) @binding(0) var<uniform> global_time: f32;
@group(0) @binding(1) var<storage, read_write> shared_data: array<f32>;