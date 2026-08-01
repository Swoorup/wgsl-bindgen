@group(0) @binding(0)
var<storage, read> values: array<vec3<f32>, 2>;

@group(0) @binding(1)
var<storage, read> nested_values: array<array<vec3<f32>, 2>, 3>;

@compute @workgroup_size(1)
fn main() {
    if values[0].x == 0.0 || nested_values[2][1].x == 0.0 {
        return;
    }
}
