//! fwgsl integration example
//!
//! This crate demonstrates using [fwgsl](https://github.com/ubugeeei/fwgsl) as a
//! shader authoring language together with `wgsl-bindgen` for type-safe GPU bindings.
//!
//! ## Pipeline
//!
//! ```text
//! shaders/scale_bias.fwgsl   (fwgsl source — a pure functional shader language)
//!     │
//!     ▼  (fwgsl compiler, invoked from build.rs)
//! WGSL helper functions
//!     │
//!     ▼  (combined with hand-written bind group declarations in build.rs)
//! scale_bias_compute.wgsl
//!     │
//!     ▼  (wgsl-bindgen, invoked from build.rs)
//! src/shader_bindings.rs   (type-safe Rust bindings)
//! ```
//!
//! The generated `shader_bindings` module provides Rust types like `ScaleBiasParams`
//! and `WgpuBindGroup0` that mirror the GPU shader's data layout exactly.

mod shader_bindings;

fn main() {
  // Demonstrate that the types generated from the fwgsl-sourced shader are
  // accessible and usable. No actual GPU is required to compile and run this.

  // Use the generated constructor (short_constructor is not set, so `new` is used)
  let params = shader_bindings::scale_bias_compute::ScaleBiasParams::new(2.0, 0.5);

  println!("fwgsl → WGSL → wgsl-bindgen integration example");
  println!("─────────────────────────────────────────────────");
  println!("ScaleBiasParams from generated bindings:");
  println!("  scale  = {}", params.scale);
  println!("  bias   = {}", params.bias);
  println!();
  println!(
    "Compute workgroup size: {:?}",
    shader_bindings::scale_bias_compute::compute::MAIN_WORKGROUP_SIZE
  );
  println!();
  println!("The shader (shaders/scale_bias.fwgsl) was written in fwgsl,");
  println!("compiled to WGSL by the fwgsl compiler in build.rs, and then");
  println!("processed by wgsl-bindgen to produce these type-safe Rust bindings.");
}
