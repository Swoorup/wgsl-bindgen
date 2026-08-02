struct NestedArrays {
    values: array<array<vec3<f32>, 2>, 3>,
}

struct RuntimeNestedArrays {
    count: u32,
    values: array<array<vec3<f32>, 2>>,
}

@group(0) @binding(0)
var<storage, read> nested: NestedArrays;

@group(0) @binding(1)
var<storage, read> runtime_nested: RuntimeNestedArrays;

@compute @workgroup_size(1)
fn main() {
    if nested.values[2][1].x == 0.0 {
        return;
    }
    if runtime_nested.count > 0u && arrayLength(&runtime_nested.values) > 0u {
        let value = runtime_nested.values[0][1].x;
        if value == 0.0 {
            return;
        }
    }
}
