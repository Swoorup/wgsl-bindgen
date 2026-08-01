struct FixedLayout {
    direction: vec3<i32>,
    samples: array<vec3<i32>, 2>,
    marker: u32,
}

struct RuntimeLayout {
    header: FixedLayout,
    values: array<u32>,
}

@group(0) @binding(0)
var<uniform> uniform_layout: FixedLayout;

@group(0) @binding(1)
var<storage, read> direct_layout: FixedLayout;

@group(0) @binding(2)
var<storage, read> layout_array: array<FixedLayout>;

@group(0) @binding(3)
var<storage, read> runtime_layout: RuntimeLayout;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&layout_array) && id.x < arrayLength(&runtime_layout.values) {
        let value = uniform_layout.direction.x
            + uniform_layout.samples[1].z
            + direct_layout.direction.x
            + layout_array[id.x].samples[0].y
            + i32(runtime_layout.values[id.x]);
        if value == 0 {
            return;
        }
    }
}
