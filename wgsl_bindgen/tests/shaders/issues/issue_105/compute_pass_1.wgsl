#import common_bindings::buffer_in

// Reads `buffer_in` only. Never mentions `main_tex` or `buffer_out`.
@compute @workgroup_size(1)
fn main() {
    let value = buffer_in[0][0];
}
