struct SegmentData {
    start: vec2<f32>,
    end: vec2<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> segment: SegmentData;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // Simple line segment using vertex_index 0 and 1
    if (vertex_index == 0u) {
        output.position = vec4<f32>(segment.start, 0.0, 1.0);
    } else {
        output.position = vec4<f32>(segment.end, 0.0, 1.0);
    }
    
    output.color = segment.color;
    return output;
}

@fragment
fn fs_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
    return color;
}