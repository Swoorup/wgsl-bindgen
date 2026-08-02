struct PaddedArray {
    values: array<vec3<f32>, 2>,
}

@group(0) @binding(0)
var<storage, read> data: PaddedArray;

@compute @workgroup_size(1)
fn main() {
    if data.values[1].x == 0.0 {
        return;
    }
}
