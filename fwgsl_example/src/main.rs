//! fwgsl integration example
//!
//! This crate demonstrates using [fwgsl](https://github.com/ubugeeei/fwgsl) as a
//! shader authoring language together with `wgsl-bindgen` for type-safe GPU bindings,
//! including **automatic** extraction of algebraic data types (ADTs), support for
//! **data-carrying enums**, and automatic generation of `From<ADT> for ParamsStruct`
//! conversion traits.
//!
//! ## Pipeline
//!
//! ```text
//! shaders/scale_bias.fwgsl   — functional helpers
//! shaders/color_compute.fwgsl — simple ADT: data Color = Red | Green | Blue
//! shaders/shape_compute.fwgsl — data-carrying ADT: data Shape = Circle F32 | Rect F32 F32
//!     │  fwgsl compiler + HIR-based annotation injection (build.rs)
//!     ▼
//! WGSL with  `// @fwgsl-adt: Color Red:0 Green:1 Blue:2`
//!            `// @fwgsl-adt: Shape Circle:0:Circle Rect:1:Rect`
//!     │  wgsl-bindgen — auto-detects annotations
//!     ▼
//! src/shader_bindings.rs  including:
//!   pub enum Color { Red, Green, Blue }           ← simple #[repr(u32)] enum
//!   pub enum Shape { Circle(Circle), Rect(Rect) } ← data-carrying enum
//!   impl From<Shape> for shaders::shape_compute::ShapeParams ← automatic conversion trait
//! ```

mod fwgsl_loader;
mod shader_bindings;

fn main() {
  use shader_bindings::{Color, Shape};
  use shader_bindings::shaders::shape_compute::{Circle, Rect, ShapeParams};

  println!("fwgsl → WGSL → wgsl-bindgen integration example");
  println!("─────────────────────────────────────────────────────────────────────────────");

  // ── Scale-bias shader ──────────────────────────────────────────────────────────
  let params = shader_bindings::shaders::scale_bias::ScaleBiasParams::new(2.0, 0.5);
  println!();
  println!("[scale_bias] ScaleBiasParams (from fwgsl helpers):");
  println!("  scale  = {}", params.scale);
  println!("  bias   = {}", params.bias);
  println!("  workgroup size = {:?}",
    shader_bindings::shaders::scale_bias::compute::MAIN_WORKGROUP_SIZE);

  // ── Color enum (simple ADT) ────────────────────────────────────────────────────
  println!();
  println!("[color_compute] Color — simple ADT, automatically extracted:");
  for color in [Color::Red, Color::Green, Color::Blue] {
    let tag: u32 = u32::from(color);
    let round_trip = Color::try_from(tag).expect("valid discriminant");
    println!("    {color:?} → u32 {tag} → {round_trip:?}");
  }

  // ── Shape enum (data-carrying ADT) ────────────────────────────────────────────
  println!();
  println!("[shape_compute] Shape — data-carrying ADT + automatic From<Shape> for ShapeParams:");
  println!("  fwgsl: `data Shape = Circle F32 | Rect F32 F32`");
  println!("  Generated: impl From<Shape> for shaders::shape_compute::ShapeParams");
  println!();

  // Construct Shape variants using the generated Rust types
  let my_circle = Shape::Circle(Circle::new(3.0));
  let my_rect   = Shape::Rect(Rect::new(4.0, 5.0));

  for shape in [my_circle, my_rect] {
    // Use the auto-generated From impl to convert the enum to GPU params
    let gpu_params: ShapeParams = ShapeParams::from(shape);

    let description = match shape {
      Shape::Circle(c) => format!("Circle(radius={:.1})", c.field0),
      Shape::Rect(r)   => format!("Rect(width={:.1}, height={:.1})", r.field0, r.field1),
    };

    println!(
      "  {description}  →  ShapeParams {{ tag={}, field0={}, field1={} }}",
      gpu_params.tag, gpu_params.field0, gpu_params.field1,
    );
  }

  // ── Hot-reload path (ComposerWithRelativePath) ────────────────────────────────
  //
  // `ComposerWithRelativePath` generates a `create_shader_module_relative_path`
  // function that reads shader sources at **runtime** via a `load_file` callback.
  // Combined with a filesystem watcher (e.g. `notify`), you can hot-reload
  // shaders without restarting your application.
  //
  // For `.fwgsl` files the callback must compile the fwgsl source to WGSL first.
  // `fwgsl_loader::make_fwgsl_load_file()` provides exactly that callback.
  println!();
  println!("[hot-reload] ComposerWithRelativePath — fwgsl-aware load_file callback:");
  println!("  SHADER_ENTRY_PATH for each shader (relative to base_dir):");
  println!(
    "    scale_bias  → {}",
    shader_bindings::shaders::scale_bias::SHADER_ENTRY_PATH
  );
  println!(
    "    color_compute → {}",
    shader_bindings::shaders::color_compute::SHADER_ENTRY_PATH
  );
  println!(
    "    shape_compute → {}",
    shader_bindings::shaders::shape_compute::SHADER_ENTRY_PATH
  );

  // Demonstrate that the fwgsl-aware load_file callback compiles .fwgsl
  // source to WGSL at runtime — no wgpu device needed for this check.
  let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let base_dir = manifest_dir.to_str().expect("valid UTF-8 path");
  let load_file = fwgsl_loader::make_fwgsl_load_file();

  println!();
  println!("  Compiling scale_bias.fwgsl at runtime via make_fwgsl_load_file():");
  let scale_bias_path = format!(
    "{base_dir}/{}",
    shader_bindings::shaders::scale_bias::SHADER_ENTRY_PATH
  );
  match load_file(&scale_bias_path) {
    Ok(wgsl) => {
      let fn_count = wgsl.matches("fn ").count();
      println!(
        "    ✓ compiled {scale_bias_path} → {fn_count} WGSL function(s)"
      );
    }
    Err(e) => println!("    ✗ error: {e}"),
  }

  println!();
  println!("  To hot-reload a shader (pseudocode — requires a wgpu Device):");
  println!("    let base_dir = env!(\"CARGO_MANIFEST_DIR\");  // or your runtime assets dir");
  println!("    let module = shaders::scale_bias::create_shader_module_relative_path(");
  println!("        &device, base_dir, Default::default(),");
  println!("        fwgsl_loader::make_fwgsl_load_file(),");
  println!("    );");
  println!("  Re-call whenever a .fwgsl file changes (e.g. via a `notify` watcher).");

  println!();
}

