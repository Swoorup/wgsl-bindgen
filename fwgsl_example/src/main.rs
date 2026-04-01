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

  println!();
  println!("All enums and From impls were generated automatically from `// @fwgsl-adt:`");
  println!("annotations injected by build.rs. No WgslEnumDefinition was needed.");
}

