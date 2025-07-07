// Shared data structures used by multiple shaders
struct SharedUniforms {
    view_matrix: mat4x4<f32>,
    time: f32,
}

struct VertexData {
    position: vec3<f32>,
    normal: vec3<f32>,
}

// Group 0: Shared by both shaders (basic rendering resources)
@group(0) @binding(0) var<uniform> shared_uniforms: SharedUniforms;
@group(0) @binding(1) var<storage, read> vertex_data: array<VertexData>;
@group(0) @binding(2) var shared_texture: texture_2d<f32>;
@group(0) @binding(3) var shared_sampler: sampler;

// Group 1: Partially shared bindings (tests partial usage scenario)
// binding 0: uniform in group 0, storage in group 1 (tests address space confusion)
@group(1) @binding(0) var<storage, read> dynamic_data: array<array<f32, 4>>;
// binding 1: uniform buffer in group 1 (different from group 0 binding 1 which is storage)
@group(1) @binding(1) var<uniform> compute_uniforms: vec4<f32>;
// binding 2: read_write storage buffer (only used by compute shader)
@group(1) @binding(2) var<storage, read_write> output_data: array<f32>;