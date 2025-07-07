struct Job {
    position: vec3<f32>,
    direction: vec3<f32>,
    accum: vec3<f32>,
    depth: u32,
}

@group(0) @binding(0)
var<storage, read_write> jobs: array<Job>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if index >= arrayLength(&jobs) {
        return;
    }
    
    // Simple computation to test the struct
    jobs[index].depth = jobs[index].depth + 1u;
}