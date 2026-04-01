# fwgsl_example

This crate demonstrates integrating [fwgsl](https://github.com/ubugeeei/fwgsl) with
[wgsl-bindgen](https://github.com/Swoorup/wgsl-bindgen) for a fully type-safe GPU shader
pipeline — including **automatic** Rust enum generation from fwgsl algebraic data types
(ADTs), support for both **simple enums** and **data-carrying enums**, and automatic
generation of `From<ADT> for ParamsStruct` conversion traits.

## Automatic ADT Extraction

Rust enums and their conversion traits are generated **automatically** from structured
`// @fwgsl-adt:` annotation comments injected into the WGSL source by `build.rs`.
**No manual configuration is needed** — wgsl-bindgen detects the annotations and emits
matching Rust types.

### Annotation format

```text
// @fwgsl-adt: TypeName Variant0:tag0 Variant1:tag1 Variant2:tag2:StructName2
```

* `Variant:tag` — tag-only variant (simple enum constructor, no payload)
* `Variant:tag:StructName` — data-carrying variant; `StructName` is the WGSL struct

## Pipeline

```text
shaders/scale_bias.fwgsl   — functional helpers (scale, bias, saturate)
shaders/color_compute.fwgsl — simple ADT: data Color = Red | Green | Blue
shaders/shape_compute.fwgsl — data-carrying: data Shape = Circle F32 | Rect F32 F32
    │  fwgsl compiler (build.rs)
    ▼
WGSL helper functions + structs + ADT metadata from HIR
    │  build.rs injects annotations
    ▼
// @fwgsl-adt: Color Red:0 Green:1 Blue:2
// @fwgsl-adt: Shape Circle:0:Circle Rect:1:Rect
    │  wgsl-bindgen (auto-detects annotations)
    ▼
src/shader_bindings.rs:
  pub enum Color { Red, Green, Blue }              ← #[repr(u32)] simple enum
  pub enum Shape { Circle(Circle), Rect(Rect) }    ← data-carrying enum
  impl From<Shape> for shape_compute::ShapeParams  ← automatic conversion trait
```

## Generated Types

### Simple enum (`Color`)

```rust
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color { Red = 0, Green = 1, Blue = 2 }

impl TryFrom<u32> for Color { ... }   // WGSL discriminant → Rust enum
impl From<Color> for u32 { ... }      // Rust enum → WGSL-compatible u32
unsafe impl bytemuck::Zeroable for Color {}
```

### Data-carrying enum (`Shape`) with automatic `From` impl

```rust
#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Circle(shape_compute::Circle),  // Circle { field0: f32 }  ← radius
    Rect(shape_compute::Rect),      // Rect   { field0: f32, field1: f32 }
}
impl Shape {
    pub fn tag(&self) -> u32 { ... }  // returns the WGSL discriminant
}

// Automatically generated when a `{EnumName}Params` struct exists in the same WGSL module:
impl From<Shape> for shape_compute::ShapeParams {
    fn from(e: Shape) -> Self {
        // tag is set from e.tag(); fields are copied by name; unmatched fields are zeroed
    }
}
```

Usage:

```rust
let shape = Shape::Circle(Circle::new(3.0));
let gpu_params: ShapeParams = ShapeParams::from(shape);
// gpu_params = ShapeParams { tag: 0, field0: 3.0, field1: 0.0, _pad: 0.0 }
```

## Running the Example

```bash
cargo run -p fwgsl_example
```

## Files

| File | Description |
|------|-------------|
| `shaders/scale_bias.fwgsl` | Functional helpers: scale/bias/saturate |
| `shaders/color_compute.fwgsl` | Simple ADT: `data Color = Red \| Green \| Blue` |
| `shaders/shape_compute.fwgsl` | Data-carrying ADT: `data Shape = Circle F32 \| Rect F32 F32` |
| `build.rs` | Compiles fwgsl → WGSL, injects `// @fwgsl-adt:` annotations, runs wgsl-bindgen |
| `src/main.rs` | Demos Color (simple), Shape (data-carrying) + `From<Shape> for ShapeParams` |
| `src/shader_bindings.rs` | Auto-generated; do not edit manually |
