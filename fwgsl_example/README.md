# fwgsl_example

This crate demonstrates integrating [fwgsl](https://github.com/ubugeeei/fwgsl) with
[wgsl-bindgen](https://github.com/Swoorup/wgsl-bindgen) for a fully type-safe GPU shader
pipeline — including **automatic** Rust enum generation from fwgsl algebraic data types
(ADTs), with support for both **simple enums** and **data-carrying enums**.

## Automatic ADT Extraction

The key feature is that Rust enums are generated **automatically** from structured
`// @fwgsl-adt:` annotation comments injected into the WGSL source by `build.rs`.
**No `WgslEnumDefinition` objects are needed** — wgsl-bindgen detects the annotations
and emits matching Rust types without any manual configuration.

### Annotation format

```text
// @fwgsl-adt: TypeName Variant0:tag0 Variant1:tag1 Variant2:tag2:StructName2
```

* `Variant:tag` — tag-only variant (simple enum constructor, no payload)
* `Variant:tag:StructName` — data-carrying variant; `StructName` is the WGSL struct
  that holds the constructor's payload

## Pipeline

```text
shaders/scale_bias.fwgsl   — functional helpers (scale, bias, saturate)
shaders/color_compute.fwgsl — simple ADT: data Color = Red | Green | Blue
shaders/shape_compute.fwgsl — data-carrying: data Shape = Circle F32 | Rect F32 F32
    │  fwgsl compiler (build.rs)
    ▼
WGSL helper functions + structs + ADT metadata from HIR
    │  build.rs injects annotations (no WgslEnumDefinition needed)
    ▼
// @fwgsl-adt: Color Red:0 Green:1 Blue:2
// @fwgsl-adt: Shape Circle:0:Circle Rect:1:Rect
    │  wgsl-bindgen (auto-detects annotations)
    ▼
src/shader_bindings.rs:
  pub enum Color { Red, Green, Blue }              ← #[repr(u32)] simple enum
  pub enum Shape { Circle(Circle), Rect(Rect) }   ← data-carrying enum
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
unsafe impl bytemuck::Pod for Color {}
```

### Data-carrying enum (`Shape`)

```rust
#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Circle(shape_compute::Circle),  // Circle { field0: f32 }  ← radius
    Rect(shape_compute::Rect),      // Rect   { field0: f32, field1: f32 }  ← width, height
}

impl Shape {
    pub fn tag(&self) -> u32 { ... }  // returns the WGSL discriminant
}
```

## What is fwgsl?

[fwgsl](https://github.com/ubugeeei/fwgsl) is a pure functional language for WebGPU that
compiles to WGSL. It provides:

- **Pure functional syntax** with Hindley-Milner type inference
- **Algebraic data types** and pattern matching
- **Dimension-carrying tensor types**
- **Expression-oriented** `let`, `if`, and `match`

## Current Integration Notes

fwgsl is an early-stage project. Two bridging steps are done in `build.rs`:

- **`@group`/`@binding` annotations** — fwgsl does not yet emit these, so they are
  added by hand alongside the fwgsl-generated helpers.

- **`alias Color = u32;`** — for simple enum ADTs, fwgsl uses bare `u32` discriminants
  but does not emit a type alias. `build.rs` adds these so naga can validate the WGSL.

- **ADT annotation injection** — after compiling fwgsl, `build.rs` walks the HIR and
  prepends `// @fwgsl-adt:` lines so wgsl-bindgen can auto-detect the enum types.

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
| `src/main.rs` | Demonstrates Color (simple) and Shape (data-carrying) generated enums |
| `src/shader_bindings.rs` | Auto-generated; do not edit manually |
