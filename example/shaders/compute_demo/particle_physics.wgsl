// Demonstrates Vec3A padding fix for compute shaders
#import global_bindings::time

struct Job {
    position: vec3<f32>,
    direction: vec3<f32>,
    accum: vec3<f32>,
    depth: u32,
}

@group(1) @binding(0)
var<storage, read_write> jobs: array<Job>;

struct Params {
    scale: f32,
    damping: f32,
}

@group(1) @binding(1)
var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if index >= arrayLength(&jobs) {
        return;
    }
    
    // Use global time binding instead of params.time
    let t = time;
    let scale = params.scale;
    let damping = params.damping;
    
    // Simple physics simulation to test the Vec3A struct
    let job = &jobs[index];
    
    // Update position based on direction
    (*job).position = (*job).position + (*job).direction * 0.016; // ~60fps
    
    // Add some wave motion based on time and position
    let wave = sin(t + (*job).position.x * 0.1) * scale;
    (*job).position.y = (*job).position.y + wave * 0.01;
    
    // Accumulate movement
    (*job).accum = (*job).accum + abs((*job).direction) * 0.1;
    
    // Apply damping
    (*job).direction = (*job).direction * damping;
    
    // Increment depth counter
    (*job).depth = (*job).depth + 1u;
    
    // Bounce off boundaries
    if abs((*job).position.x) > 10.0 || abs((*job).position.y) > 10.0 || abs((*job).position.z) > 10.0 {
        (*job).direction = -(*job).direction * 0.8;
    }
}