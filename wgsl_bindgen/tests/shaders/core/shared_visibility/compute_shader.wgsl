#import common::global_time
#import common::shared_data

@compute @workgroup_size(64)
fn compute_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index < arrayLength(&shared_data)) {
        shared_data[index] = shared_data[index] + global_time;
    }
}