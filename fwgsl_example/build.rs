//! Build script for the fwgsl_example crate.
//!
//! This build script demonstrates integrating fwgsl (https://github.com/ubugeeei/fwgsl)
//! with wgsl-bindgen.  The pipeline is:
//!
//!   1. Read `.fwgsl` source files (pure functional shader language)
//!   2. Compile them to WGSL using the fwgsl compiler library
//!   3. Inject `// @fwgsl-adt:` annotation comments derived from the fwgsl HIR
//!   4. Combine the annotated WGSL with hand-written bind group declarations
//!      and compute entry points
//!   5. Run wgsl-bindgen on the combined WGSL
//!
//! **Automatic ADT extraction** — the key difference from the previous
//! `WgslEnumDefinition` approach is that the ADT metadata is embedded directly into
//! the WGSL source as structured comment annotations.  wgsl-bindgen detects and
//! processes these annotations automatically; no manual `WgslEnumDefinition` objects
//! are needed in the build script.

use std::env;
use std::fs;
use std::path::Path;

use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{WgslBindgenOptionBuilder, WgslTypeSerializeStrategy};

// ─────────────────────────────────────────────────────────────────
// fwgsl compile pipeline
// ─────────────────────────────────────────────────────────────────

/// The result of compiling a `.fwgsl` source file.
struct FwgslOutput {
  /// Complete WGSL source text, with `// @fwgsl-adt:` annotations injected at
  /// the top for every user-defined algebraic data type found in the HIR.
  wgsl: String,
}

/// Compile a `.fwgsl` source string to annotated WGSL.
///
/// The returned [`FwgslOutput::wgsl`] string contains:
/// * One `// @fwgsl-adt:` comment per user-defined ADT, encoding variant names,
///   discriminant tags, and (for data-carrying constructors) the WGSL struct name.
/// * The regular WGSL helper functions and struct definitions produced by the
///   fwgsl compiler.
///
/// wgsl-bindgen automatically parses the `// @fwgsl-adt:` lines and emits
/// matching Rust `#[repr(u32)]` enums or data-carrying Rust enums — no
/// `WgslEnumDefinition` is required.
fn compile_fwgsl(source: &str) -> Result<FwgslOutput> {
  // Phase 1: Parse
  let mut parser = fwgsl_parser::parser::Parser::new(source);
  let program = parser.parse_program();

  if parser.diagnostics().has_errors() {
    let errors: Vec<String> = parser
      .diagnostics()
      .iter()
      .filter(|d| d.severity == fwgsl_diagnostics::Severity::Error)
      .map(|d| d.message.clone())
      .collect();
    return Err(miette::miette!("fwgsl parse errors: {}", errors.join(", ")));
  }

  // Phase 2: Semantic analysis — populates constructor / data-type tables
  let mut analyzer = fwgsl_semantic::SemanticAnalyzer::new();
  analyzer.analyze(&program);

  if analyzer.has_errors() {
    let errors: Vec<String> = analyzer
      .diagnostics()
      .iter()
      .filter(|d| d.severity == fwgsl_diagnostics::Severity::Error)
      .map(|d| d.message.clone())
      .collect();
    return Err(miette::miette!("fwgsl semantic errors: {}", errors.join(", ")));
  }

  // Phase 3: AST → HIR
  let mut lowering = fwgsl_ast_lowering::AstLowering::new(&analyzer);
  let hir = lowering.lower_program(&program);

  if lowering.has_errors() {
    let errors: Vec<String> = lowering
      .diagnostics()
      .iter()
      .filter(|d| d.severity == fwgsl_diagnostics::Severity::Error)
      .map(|d| d.message.clone())
      .collect();
    return Err(miette::miette!("fwgsl lowering errors: {}", errors.join(", ")));
  }

  // Phase 4: HIR → MIR
  let mir = fwgsl_mir::lower::lower_hir_to_mir(&hir)
    .map_err(|errors| miette::miette!("fwgsl MIR errors: {:?}", errors))?;

  // Phase 5: MIR → WGSL text
  let raw_wgsl = fwgsl_wgsl_codegen::emit_wgsl(&mir);

  // Phase 6: Inject `// @fwgsl-adt:` annotations derived from the HIR.
  //
  // These annotations encode the ADT type names, variant names, discriminant
  // tags, and (for data-carrying constructors) the names of the corresponding
  // WGSL structs.  wgsl-bindgen parses them automatically — the user does not
  // need to configure anything.
  let annotated_wgsl = inject_adt_annotations(&raw_wgsl, &hir);

  Ok(FwgslOutput { wgsl: annotated_wgsl })
}

/// Prepend `// @fwgsl-adt:` comment lines to the WGSL source for every
/// user-defined algebraic data type found in the HIR.
///
/// Line format:
/// ```text
/// // @fwgsl-adt: TypeName Variant0:tag0 Variant1:tag1 Variant2:tag2:StructName2
/// ```
/// * `Variant:tag` — tag-only (simple enum) constructor, no struct payload
/// * `Variant:tag:StructName` — data-carrying constructor; the WGSL struct
///   that holds the payload has the same name as the constructor in fwgsl
fn inject_adt_annotations(wgsl: &str, hir: &fwgsl_hir::HirProgram) -> String {
  // Built-in ADTs shipped with fwgsl — we don't emit annotations for these.
  const BUILTIN_ADT_NAMES: &[&str] = &["Option", "Result", "Pair"];

  let mut annotations = String::new();

  // Sort data types by name for deterministic output
  let mut data_types: Vec<&fwgsl_hir::HirDataType> = hir.data_types.iter().collect();
  data_types.sort_by(|a, b| a.name.cmp(&b.name));

  for dt in data_types {
    if BUILTIN_ADT_NAMES.contains(&dt.name.as_str()) {
      continue;
    }

    // Build the space-separated list of `VarName:tag` or `VarName:tag:StructName` tokens
    let mut variant_tokens: Vec<String> = dt
      .constructors
      .iter()
      .map(|c| {
        if c.fields.is_empty() {
          // Tag-only variant (simple enum constructor)
          format!("{}:{}", c.name, c.tag)
        } else {
          // Data-carrying variant: struct name = constructor name (fwgsl convention)
          format!("{}:{}:{}", c.name, c.tag, c.name)
        }
      })
      .collect();

    // Sort by tag so the output order is deterministic.
    // Each token is "VarName:tag" or "VarName:tag:StructName"; the tag is the second field.
    fn parse_token_tag(tok: &str) -> u32 {
      tok.split(':').nth(1).and_then(|s| s.parse().ok()).unwrap_or(u32::MAX)
    }
    variant_tokens.sort_by_key(|tok: &String| parse_token_tag(tok));

    annotations.push_str(&format!(
      "// @fwgsl-adt: {} {}\n",
      dt.name,
      variant_tokens.join(" ")
    ));
  }

  format!("{}{}", annotations, wgsl)
}

// ─────────────────────────────────────────────────────────────────
// Per-shader build helpers
// ─────────────────────────────────────────────────────────────────

/// Build the combined WGSL for the scale_bias compute shader.
fn build_scale_bias_wgsl(manifest_dir: &Path) -> Result<String> {
  let fwgsl_path = manifest_dir.join("shaders/scale_bias.fwgsl");
  println!("cargo::rerun-if-changed={}", fwgsl_path.display());

  let source = fs::read_to_string(&fwgsl_path).into_diagnostic()?;
  let out = compile_fwgsl(&source)?;

  Ok(format!(
    r#"// Combined WGSL for the scale_bias compute shader.
//
// Helper functions (scale_val, bias_val, scale_bias, saturate) are generated
// from shaders/scale_bias.fwgsl via the fwgsl compiler.

struct ScaleBiasParams {{
    scale: f32,
    bias: f32,
}}

@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@group(0) @binding(1) var<uniform> params: ScaleBiasParams;

// --- fwgsl-generated helper functions ---
{}
// --- end fwgsl-generated code ---

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if idx >= arrayLength(&data) {{
        return;
    }}
    data[idx] = scale_bias(params.scale, params.bias, data[idx]);
}}
"#,
    out.wgsl
  ))
}

/// Build the combined WGSL for the color compute shader (simple enum ADT).
fn build_color_wgsl(manifest_dir: &Path) -> Result<String> {
  let fwgsl_path = manifest_dir.join("shaders/color_compute.fwgsl");
  println!("cargo::rerun-if-changed={}", fwgsl_path.display());

  let source = fs::read_to_string(&fwgsl_path).into_diagnostic()?;
  let out = compile_fwgsl(&source)?;

  // Build `alias <EnumName> = u32;` so naga can validate function signatures
  // that reference the ADT type name (fwgsl does not emit these automatically).
  let alias_decls = alias_decls_from_annotations(&out.wgsl);

  let wgsl_body = out.wgsl;
  Ok(format!(
    r#"// Combined WGSL for the color_compute shader.
//
// The `Color` ADT (Red=0, Green=1, Blue=2) is a simple enum in fwgsl.
// The `// @fwgsl-adt:` annotation below is injected by build.rs from the HIR;
// wgsl-bindgen automatically generates a Rust `#[repr(u32)] enum Color`.
// No WgslEnumDefinition is required.

// Type aliases for fwgsl simple enums (→ u32 in WGSL)
{alias_decls}
struct ColorParams {{
    color_tag: u32,
}}

@group(0) @binding(0) var<storage, read_write> output: array<f32, 4>;
@group(0) @binding(1) var<uniform> params: ColorParams;

// --- fwgsl-generated helper functions (with @fwgsl-adt: annotation) ---
{wgsl_body}
// --- end fwgsl-generated code ---

// Workgroup size of 1 is intentional: decodes a single Color tag → [R, G, B, A].
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) _global_id: vec3<u32>) {{
    let selected_color: Color = params.color_tag;
    output[0] = color_to_r(selected_color);
    output[1] = color_to_g(selected_color);
    output[2] = color_to_b(selected_color);
    output[3] = 1.0;
}}
"#,
  ))
}

/// Build the combined WGSL for the shape compute shader (data-carrying ADT).
///
/// `data Shape = Circle F32 | Rect F32 F32` produces two WGSL structs:
///   `struct Circle { field0: f32 }` and `struct Rect { field0: f32, field1: f32 }`
///
/// The `// @fwgsl-adt:` annotation is injected by build.rs so wgsl-bindgen
/// automatically generates `pub enum Shape { Circle(Circle), Rect(Rect) }`.
fn build_shape_wgsl(manifest_dir: &Path) -> Result<String> {
  let fwgsl_path = manifest_dir.join("shaders/shape_compute.fwgsl");
  println!("cargo::rerun-if-changed={}", fwgsl_path.display());

  let source = fs::read_to_string(&fwgsl_path).into_diagnostic()?;
  let out = compile_fwgsl(&source)?;

  let wgsl_body = out.wgsl;
  Ok(format!(
    r#"// Combined WGSL for the shape_compute shader.
//
// `data Shape = Circle F32 | Rect F32 F32` produces WGSL structs Circle and Rect.
// The `// @fwgsl-adt:` annotation is injected by build.rs; wgsl-bindgen generates:
//   pub enum Shape {{ Circle(Circle), Rect(Rect) }}
// with a `.tag() -> u32` method.  No WgslEnumDefinition is required.

// --- fwgsl-generated structs (with @fwgsl-adt: annotation) ---
{wgsl_body}
// --- end fwgsl-generated code ---

// Uniform holding either a Circle or Rect's fields plus a tag.
// CPU code populates the appropriate fields using the generated Rust enum.
struct ShapeParams {{
    // Shape tag: 0 = Circle, 1 = Rect
    tag: u32,
    // Circle field: radius (used when tag == 0)
    field0: f32,
    // Rect fields: width and height (used when tag == 1)
    field1: f32,
    // padding
    _pad: f32,
}}

/// Output: computed "area" for the given shape
@group(0) @binding(0) var<storage, read_write> output: array<f32, 1>;
@group(0) @binding(1) var<uniform> params: ShapeParams;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) _global_id: vec3<u32>) {{
    var area: f32 = 0.0;
    if params.tag == 0u {{
        // Circle: area = field0 * field0  (r*r; skip pi for simplicity)
        area = params.field0 * params.field0;
    }} else {{
        // Rect: area = field0 * field1  (w*h)
        area = params.field0 * params.field1;
    }}
    output[0] = area;
}}
"#,
  ))
}

/// Extract `alias X = u32;` declarations from the `// @fwgsl-adt:` annotations
/// embedded in the WGSL for every *simple* (tag-only) ADT.
///
/// fwgsl does not emit these aliases automatically, but they are needed so naga
/// can validate function signatures like `fn color_to_r(c: Color) -> f32`.
fn alias_decls_from_annotations(wgsl: &str) -> String {
  const PREFIX: &str = "// @fwgsl-adt:";
  let mut aliases = String::new();

  for line in wgsl.lines() {
    let line = line.trim();
    if !line.starts_with(PREFIX) {
      continue;
    }
    let rest = line[PREFIX.len()..].trim();
    let mut tokens = rest.split_whitespace();
    let type_name = match tokens.next() {
      Some(n) if !n.is_empty() => n,
      _ => continue,
    };
    // Only emit alias for simple enums (no `:StructName` in any variant token)
    let is_simple = tokens.all(|tok| tok.matches(':').count() < 2);
    if is_simple {
      aliases.push_str(&format!("alias {} = u32;\n", type_name));
    }
  }
  aliases
}

// ─────────────────────────────────────────────────────────────────
// Main entry point
// ─────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
  let out_dir = std::path::PathBuf::from(env::var("OUT_DIR").into_diagnostic()?);
  let manifest_dir = std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").into_diagnostic()?);

  // Build all three shaders
  let scale_bias_wgsl = build_scale_bias_wgsl(&manifest_dir)?;
  let color_wgsl = build_color_wgsl(&manifest_dir)?;
  let shape_wgsl = build_shape_wgsl(&manifest_dir)?;

  // Write combined WGSL files to build output directory
  let scale_bias_path = out_dir.join("scale_bias_compute.wgsl");
  let color_path = out_dir.join("color_compute.wgsl");
  let shape_path = out_dir.join("shape_compute.wgsl");
  fs::write(&scale_bias_path, &scale_bias_wgsl).into_diagnostic()?;
  fs::write(&color_path, &color_wgsl).into_diagnostic()?;
  fs::write(&shape_path, &shape_wgsl).into_diagnostic()?;

  // Run wgsl-bindgen on all three shaders.
  //
  // ADT enums are generated automatically from the `// @fwgsl-adt:` annotations
  // embedded in each WGSL file — no WgslEnumDefinition is required.
  WgslBindgenOptionBuilder::default()
    .workspace_root(out_dir)
    .add_entry_point(scale_bias_path.to_str().unwrap())
    .add_entry_point(color_path.to_str().unwrap())
    .add_entry_point(shape_path.to_str().unwrap())
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .output(manifest_dir.join("src/shader_bindings.rs"))
    .build()
    .into_diagnostic()?
    .generate()
    .into_diagnostic()?;

  Ok(())
}
