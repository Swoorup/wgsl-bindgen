// Shared types module
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec3<f32>,
}

struct CommonData {
    transform: mat4x4<f32>,
    time: f32,
}