#import vertices::VertexIn

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex_main(input: VertexIn) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 1.0, 1.0);
}