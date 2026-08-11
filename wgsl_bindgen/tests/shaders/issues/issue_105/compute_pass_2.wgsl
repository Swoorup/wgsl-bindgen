#import common_bindings::{main_tex, buffer_in, buffer_out}

// Writes `buffer_out`, the binding whose generated type was wrong.
@compute @workgroup_size(1)
fn main() {
    buffer_out[0][0] = buffer_in[0][0];
    textureStore(main_tex, vec2<i32>(0, 0), vec4<f32>(1.0));
}
