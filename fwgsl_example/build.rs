//! Build script for the fwgsl_example crate.
//!
//! Demonstrates integrating [fwgsl](https://github.com/ubugeeei/fwgsl) with
//! wgsl-bindgen using the [`SourcePreprocessor`] hook.
//!
//! ## Pipeline (old approach — avoided here)
//!
//! The old approach required:
//!   1. Reading each `.fwgsl` file manually
//!   2. Compiling it to WGSL
//!   3. Wrapping it with hand-written bind-group / entry-point boilerplate
//!   4. Writing the combined WGSL to `$OUT_DIR/*.wgsl`
//!   5. Passing those temporary paths to `add_entry_point`
//!
//! ## Pipeline (new approach — used here)
//!
//! 1. Register a [`SourcePreprocessor`] closure that compiles any `.fwgsl` file
//!    on-the-fly and returns the combined WGSL string.
//! 2. Pass the **original** `.fwgsl` paths directly to `add_entry_point`.
//! 3. wgsl-bindgen calls the preprocessor for each file, feeds the returned WGSL
//!    to naga, and generates Rust bindings — no temporary files needed.

use std::fs;
use std::path::{Path, PathBuf};

use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{WgslBindgenOptionBuilder, WgslTypeSerializeStrategy};

// ─────────────────────────────────────────────────────────────────
// fwgsl compile pipeline
// ─────────────────────────────────────────────────────────────────

/// Compile a `.fwgsl` source string to annotated WGSL.
///
/// The returned WGSL includes:
/// * One `// @fwgsl-adt:` comment per user-defined ADT (parsed by wgsl-bindgen
///   to emit Rust enums automatically).
/// * The regular WGSL helper functions / structs produced by the fwgsl compiler.
fn compile_fwgsl(source: &str) -> Result<String> {
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

  // Phase 2: Semantic analysis
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

  // Phase 6: Inject `// @fwgsl-adt:` annotations from the HIR
  Ok(inject_adt_annotations(&raw_wgsl, &hir))
}

/// Prepend `// @fwgsl-adt:` comment lines for every user-defined ADT in the HIR.
fn inject_adt_annotations(wgsl: &str, hir: &fwgsl_hir::HirProgram) -> String {
  const BUILTIN_ADT_NAMES: &[&str] = &["Option", "Result", "Pair"];

  let mut annotations = String::new();
  let mut data_types: Vec<&fwgsl_hir::HirDataType> = hir.data_types.iter().collect();
  data_types.sort_by(|a, b| a.name.cmp(&b.name));

  for dt in data_types {
    if BUILTIN_ADT_NAMES.contains(&dt.name.as_str()) {
      continue;
    }

    let mut variant_tokens: Vec<String> = dt
      .constructors
      .iter()
      .map(|c| {
        if c.fields.is_empty() {
          format!("{}:{}", c.name, c.tag)
        } else {
          format!("{}:{}:{}", c.name, c.tag, c.name)
        }
      })
      .collect();

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
// Shader-specific WGSL wrappers
// ─────────────────────────────────────────────────────────────────
//
// Each function takes the annotated fwgsl-generated WGSL and wraps it with
// the hand-written GPU boilerplate (uniform structs, bind group bindings,
// compute entry points).

fn scale_bias_combined_wgsl(fwgsl_wgsl: &str) -> String {
  format!(
    r#"// Combined WGSL for the scale_bias compute shader.

struct ScaleBiasParams {{
    scale: f32,
    bias: f32,
}}

@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@group(0) @binding(1) var<uniform> params: ScaleBiasParams;

// --- fwgsl-generated helper functions ---
{fwgsl_wgsl}
// --- end fwgsl-generated code ---

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if idx >= arrayLength(&data) {{
        return;
    }}
    data[idx] = scale_bias(params.scale, params.bias, data[idx]);
}}
"#
  )
}

fn color_combined_wgsl(fwgsl_wgsl: &str) -> String {
  // Simple enums need `alias X = u32;` so naga can validate function signatures
  // that reference the ADT type name (fwgsl does not emit these automatically).
  let alias_decls = alias_decls_from_annotations(fwgsl_wgsl);
  format!(
    r#"// Combined WGSL for the color_compute shader.

// Type aliases for fwgsl simple enums (→ u32 in WGSL)
{alias_decls}
struct ColorParams {{
    color_tag: u32,
}}

@group(0) @binding(0) var<storage, read_write> output: array<f32, 4>;
@group(0) @binding(1) var<uniform> params: ColorParams;

// --- fwgsl-generated helper functions (with @fwgsl-adt: annotation) ---
{fwgsl_wgsl}
// --- end fwgsl-generated code ---

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) _global_id: vec3<u32>) {{
    let selected_color: Color = params.color_tag;
    output[0] = color_to_r(selected_color);
    output[1] = color_to_g(selected_color);
    output[2] = color_to_b(selected_color);
    output[3] = 1.0;
}}
"#
  )
}

fn shape_combined_wgsl(fwgsl_wgsl: &str) -> String {
  format!(
    r#"// Combined WGSL for the shape_compute shader.

// --- fwgsl-generated structs (with @fwgsl-adt: annotation) ---
{fwgsl_wgsl}
// --- end fwgsl-generated code ---

struct ShapeParams {{
    tag: u32,
    field0: f32,
    field1: f32,
    _pad: f32,
}}

@group(0) @binding(0) var<storage, read_write> output: array<f32, 1>;
@group(0) @binding(1) var<uniform> params: ShapeParams;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) _global_id: vec3<u32>) {{
    var area: f32 = 0.0;
    if params.tag == 0u {{
        area = params.field0 * params.field0;
    }} else {{
        area = params.field0 * params.field1;
    }}
    output[0] = area;
}}
"#
  )
}

/// Extract `alias X = u32;` declarations for every simple (tag-only) ADT in the
/// annotated WGSL.
///
/// Note: this replicates the `// @fwgsl-adt:` annotation parsing that also exists in
/// `wgsl_bindgen::adt`.  The duplication is intentional — `alias_decls_from_annotations`
/// is a *build-time*, build-script-local concern (creating WGSL aliases so naga can
/// validate function signatures like `fn color_to_r(c: Color) -> f32`), whereas
/// `wgsl_bindgen::adt` is the *library-side* parsing that generates Rust enums.
/// Neither is the right place to depend on the other.
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
    let is_simple = tokens.all(|tok| tok.matches(':').count() < 2);
    if is_simple {
      aliases.push_str(&format!("alias {} = u32;\n", type_name));
    }
  }
  aliases
}

// ─────────────────────────────────────────────────────────────────
// Source preprocessor
// ─────────────────────────────────────────────────────────────────

/// Build the [`SourcePreprocessor`] closure that handles `.fwgsl` files.
///
/// For every file whose extension is `.fwgsl` the closure:
///   1. Reads the source from disk,
///   2. Compiles it through the full fwgsl pipeline,
///   3. Wraps the output with the appropriate GPU boilerplate,
///   4. Returns the combined WGSL string — no temporary file is written.
///
/// For any other extension `None` is returned and wgsl-bindgen reads the file
/// from disk as usual.
fn make_fwgsl_preprocessor() -> impl Fn(&Path) -> Option<String> + Send + Sync + 'static {
  move |path: &Path| {
    // Only handle .fwgsl files
    if path.extension().and_then(|e| e.to_str()) != Some("fwgsl") {
      return None;
    }

    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let source = fs::read_to_string(path)
      .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    let fwgsl_wgsl = compile_fwgsl(&source)
      .unwrap_or_else(|e| panic!("failed to compile {}: {}", path.display(), e));

    let combined = match name {
      "scale_bias" => scale_bias_combined_wgsl(&fwgsl_wgsl),
      "color_compute" => color_combined_wgsl(&fwgsl_wgsl),
      "shape_compute" => shape_combined_wgsl(&fwgsl_wgsl),
      _ => fwgsl_wgsl,
    };
    Some(combined)
  }
}

// ─────────────────────────────────────────────────────────────────
// Main entry point
// ─────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
  let manifest_dir = PathBuf::from(
    std::env::var("CARGO_MANIFEST_DIR").into_diagnostic()?,
  );

  // Tell Cargo to re-run this build script when the shader sources change.
  println!("cargo::rerun-if-changed=shaders/");
  println!("cargo::rerun-if-changed=build.rs");

  // Run wgsl-bindgen with the fwgsl preprocessor.
  //
  // .fwgsl files are passed directly — no temporary .wgsl files are written.
  // The preprocessor compiles each .fwgsl file on demand and returns the
  // combined WGSL (fwgsl-generated code + hand-written bind group / entry point
  // boilerplate) as a string.  wgsl-bindgen feeds that string to naga and
  // generates type-safe Rust bindings including ADT enums and From impls.
  WgslBindgenOptionBuilder::default()
    .workspace_root(&manifest_dir)
    .source_preprocessor(make_fwgsl_preprocessor())
    .add_entry_point(manifest_dir.join("shaders/scale_bias.fwgsl").to_str().unwrap())
    .add_entry_point(manifest_dir.join("shaders/color_compute.fwgsl").to_str().unwrap())
    .add_entry_point(manifest_dir.join("shaders/shape_compute.fwgsl").to_str().unwrap())
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .output(manifest_dir.join("src/shader_bindings.rs"))
    .build()
    .into_diagnostic()?
    .generate()
    .into_diagnostic()?;

  Ok(())
}
