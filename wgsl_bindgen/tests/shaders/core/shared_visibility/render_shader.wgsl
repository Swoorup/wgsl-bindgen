#import common::global_time
#import common::shared_data

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    let data_value = shared_data[vertex_index % arrayLength(&shared_data)];
    output.position = vec4<f32>(data_value * global_time, 0.0, 0.0, 1.0);
    output.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color * global_time;
}