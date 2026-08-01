struct MixedLayout {
    accents: array<vec3<f32>, 2>,
    tag: u32,
    values: array<vec3<f32>>,
}

@group(0) @binding(0)
var<storage, read> mixed: MixedLayout;

@compute @workgroup_size(1)
fn main() {
    if mixed.tag > 0u && arrayLength(&mixed.values) > 0u {
        let value = mixed.accents[1].x + mixed.values[0].y;
        if value == 0.0 {
            return;
        }
    }
}
