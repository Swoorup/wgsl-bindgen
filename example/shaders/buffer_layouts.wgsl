struct FixedLayout {
    // Rust's default type map widens vec3<f32> to [f32; 4].
    primary: vec3<f32>,
    accents: array<vec3<f32>, 2>,
    tag: u32,
}

struct RuntimeLayout {
    header: FixedLayout,
    colors: array<vec4<f32>>,
}

// Exercise FixedLayout as both a uniform and a direct storage binding.
@group(0) @binding(0)
var<uniform> uniform_layout: FixedLayout;

@group(0) @binding(1)
var<storage, read> direct_layout: FixedLayout;

// Exercise FixedLayout as the element of a runtime-sized binding array.
@group(0) @binding(2)
var<storage, read> layout_array: array<FixedLayout>;

// Exercise RuntimeLayout, including its fixed header and runtime-sized tail.
@group(0) @binding(3)
var<storage, read> runtime_layout: RuntimeLayout;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions = array(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[index], 0.0, 1.0);
    output.uv = positions[index] * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    return output;
}

fn card(primary: vec3<f32>, accent: vec3<f32>, tag: u32, local: vec2<f32>) -> vec3<f32> {
    let edge = min(min(local.x, 1.0 - local.x), min(local.y, 1.0 - local.y));
    let inset = smoothstep(0.015, 0.045, edge);
    let glow = 0.85 + 0.15 * cos((local.x + local.y) * 6.28318 + f32(tag));
    return mix(vec3<f32>(0.025), mix(primary, accent, 0.22) * glow, inset);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tile = min(vec2<u32>(input.uv * 2.0), vec2<u32>(1));
    let local = fract(input.uv * 2.0);
    let tile_index = tile.y * 2u + tile.x;

    var primary: vec3<f32>;
    var accent: vec3<f32>;
    var tag: u32;

    switch tile_index {
        case 0u: {
            primary = uniform_layout.primary;
            accent = uniform_layout.accents[0];
            tag = uniform_layout.tag;
        }
        case 1u: {
            primary = direct_layout.primary;
            accent = direct_layout.accents[1];
            tag = direct_layout.tag;
        }
        case 2u: {
            let count = arrayLength(&layout_array);
            let item = min(u32(local.x * f32(count)), count - 1u);
            primary = layout_array[item].primary;
            accent = layout_array[item].accents[item % 2u];
            tag = layout_array[item].tag;
        }
        default: {
            let count = arrayLength(&runtime_layout.colors);
            let item = min(u32(local.x * f32(count)), count - 1u);
            primary = runtime_layout.colors[item].rgb;
            accent = runtime_layout.header.accents[item % 2u];
            tag = runtime_layout.header.tag;
        }
    }

    return vec4<f32>(card(primary, accent, tag, local), 1.0);
}
