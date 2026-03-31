// Shared utility module imported via WESL import syntax

fn apply_scale(pos: vec4<f32>, scale: f32) -> vec4<f32> {
    return pos * scale;
}
