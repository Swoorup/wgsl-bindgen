//! Build script for the fwgsl_example crate.
//!
//! Compiles `.fwgsl` source files to WGSL and feeds them to wgsl-bindgen via
//! a [`SourcePreprocessor`] hook — no temporary `.wgsl` files are written.
//!
//! ## File layout
//!
//! Each `.fwgsl` file is self-contained: it holds both the pure-functional
//! fwgsl logic **and** the GPU setup ceremony (uniform structs, bind group
//! bindings, compute entry points), separated by a `-- @wgsl` line:
//!
//! ```text
//! -- fwgsl code (compiled to WGSL helper functions / structs)
//! scale_bias : F32 -> F32 -> F32 -> F32
//! scale_bias s b x = x * s + b
//!
//! -- @wgsl
//! struct ScaleBiasParams { scale: f32, bias: f32, }
//! @group(0) @binding(0) var<storage, read_write> data: array<f32>;
//! @group(0) @binding(1) var<uniform> params: ScaleBiasParams;
//! @compute @workgroup_size(64, 1, 1)
//! fn main(...) { ... }
//! ```
//!
//! The preprocessor:
//! 1. Splits the file on `-- @wgsl`.
//! 2. Compiles the fwgsl part (everything above the marker) to WGSL.
//! 3. Returns `fwgsl_output + raw_wgsl_section` — a complete WGSL module.

use std::fs;
use std::path::{Path, PathBuf};

use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{WgslBindgenOptionBuilder, WgslTypeSerializeStrategy};

// ─────────────────────────────────────────────────────────────────
// fwgsl compile pipeline
// ─────────────────────────────────────────────────────────────────

/// Compile a `.fwgsl` source string to annotated WGSL.
///
/// Returns WGSL with `// @fwgsl-adt:` comment annotations prepended for every
/// user-defined ADT.  wgsl-bindgen parses these automatically to emit Rust enums.
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
// Source preprocessor
// ─────────────────────────────────────────────────────────────────

/// The marker line that separates fwgsl code from the raw WGSL section.
///
/// Everything above this line in a `.fwgsl` file is compiled through the fwgsl
/// pipeline.  Everything below it is raw WGSL and is appended verbatim after
/// the compiled output.
const WGSL_SECTION_MARKER: &str = "-- @wgsl";

/// Build the generic [`SourcePreprocessor`] for `.fwgsl` files.
///
/// For every file whose extension is `.fwgsl` the closure:
///   1. Reads the source from disk.
///   2. Splits on the first `-- @wgsl` line.
///   3. Compiles the fwgsl portion to annotated WGSL.
///   4. Appends the raw WGSL section (if any) and returns the combined string.
///
/// For any other extension `None` is returned and wgsl-bindgen falls back to
/// reading the file from disk normally.
fn make_fwgsl_preprocessor() -> impl Fn(&Path) -> Option<String> + Send + Sync + 'static {
  move |path: &Path| {
    if path.extension().and_then(|e| e.to_str()) != Some("fwgsl") {
      return None;
    }

    let full_source = fs::read_to_string(path)
      .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));

    // Split on the first `-- @wgsl` line.
    let (fwgsl_source, wgsl_section) = match full_source
      .lines()
      .enumerate()
      .find(|(_, line)| line.trim() == WGSL_SECTION_MARKER)
    {
      Some((idx, _)) => {
        let fwgsl_part: String = full_source
          .lines()
          .take(idx)
          .collect::<Vec<_>>()
          .join("\n");
        let wgsl_part: String = full_source
          .lines()
          .skip(idx + 1)
          .collect::<Vec<_>>()
          .join("\n");
        (fwgsl_part, wgsl_part)
      }
      None => (full_source.clone(), String::new()),
    };

    let compiled_wgsl = compile_fwgsl(&fwgsl_source)
      .unwrap_or_else(|e| panic!("failed to compile {}: {}", path.display(), e));

    // Combine: compiled fwgsl output followed by the raw WGSL section.
    let combined = if wgsl_section.is_empty() {
      compiled_wgsl
    } else {
      format!("{}\n{}", compiled_wgsl, wgsl_section)
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

  // Run wgsl-bindgen with the generic fwgsl preprocessor.
  //
  // Each `.fwgsl` file is fully self-contained — no per-shader wrapper
  // functions are needed here.  The preprocessor splits on `-- @wgsl`,
  // compiles the fwgsl portion, and appends the raw WGSL section.
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

