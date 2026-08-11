// Only `compute_pass_2.wgsl` uses this, so it is absent from the module
// composed for `compute_pass_1.wgsl`. That offsets the two type arenas
// relative to each other.
@group(0) @binding(0)
var main_tex: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(1)
var<storage, read> buffer_in: array<array<u32, 8>, 8>;

@group(0) @binding(2)
var<storage, read_write> buffer_out: array<array<u32, 8>, 8>;
