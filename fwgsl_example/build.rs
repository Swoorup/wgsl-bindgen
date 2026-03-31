//! Build script for the fwgsl_example crate.
//!
//! This build script demonstrates integrating fwgsl (https://github.com/ubugeeei/fwgsl)
//! with wgsl-bindgen. The pipeline is:
//!
//!   1. Read `.fwgsl` source files (pure functional shader language)
//!   2. Compile them to WGSL using the fwgsl compiler library
//!   3. Combine the generated WGSL helper functions with hand-written bind group
//!      declarations and a compute entry point
//!   4. Run wgsl-bindgen on the combined WGSL to generate type-safe Rust bindings

use std::env;
use std::fs;
use std::path::PathBuf;

use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{WgslBindgenOptionBuilder, WgslTypeSerializeStrategy};

/// Compile a fwgsl source string to WGSL using the fwgsl compiler pipeline.
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
    return Err(miette::miette!(
      "fwgsl semantic errors: {}",
      errors.join(", ")
    ));
  }

  // Phase 3: AST -> HIR lowering
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

  // Phase 4: HIR -> MIR lowering
  let mir = fwgsl_mir::lower::lower_hir_to_mir(&hir)
    .map_err(|errors| miette::miette!("fwgsl MIR errors: {:?}", errors))?;

  // Phase 5: MIR -> WGSL text
  Ok(fwgsl_wgsl_codegen::emit_wgsl(&mir))
}

fn main() -> Result<()> {
  let out_dir = PathBuf::from(env::var("OUT_DIR").into_diagnostic()?);
  let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").into_diagnostic()?);

  // Tell Cargo to rerun this build script if any fwgsl source files change.
  let fwgsl_path = manifest_dir.join("shaders/scale_bias.fwgsl");
  println!("cargo::rerun-if-changed={}", fwgsl_path.display());

  // Step 1: Compile the fwgsl shader to WGSL helper functions.
  let fwgsl_source = fs::read_to_string(&fwgsl_path).into_diagnostic()?;
  let generated_fns = compile_fwgsl(&fwgsl_source)?;

  // Step 2: Build the full WGSL by combining:
  //   - Struct definitions and bind group declarations (hand-written, because
  //     fwgsl does not yet support @group/@binding annotations)
  //   - The helper functions generated from the fwgsl source
  //   - The compute entry point that calls those helpers (hand-written)
  let full_wgsl = format!(
    r#"// Combined WGSL for the scale_bias compute shader.
//
// Helper functions (scale_val, bias_val, scale_bias, saturate) are generated
// from shaders/scale_bias.fwgsl via the fwgsl compiler.
// The struct, bindings, and entry point below are hand-written because fwgsl
// does not yet emit @group/@binding annotations.

/// Uniform parameters controlling the scale-bias transformation.
struct ScaleBiasParams {{
    /// Multiplicative scale factor applied to each element.
    scale: f32,
    /// Additive bias applied after scaling.
    bias: f32,
}}

/// Input/output storage buffer of f32 values.
@group(0) @binding(0) var<storage, read_write> data: array<f32>;
/// Uniform buffer holding the transformation parameters.
@group(0) @binding(1) var<uniform> params: ScaleBiasParams;

// --- fwgsl-generated helper functions ---
{generated_fns}
// --- end fwgsl-generated code ---

/// Compute entry point: applies scale_bias to every element in `data`.
@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let idx = global_id.x;
    if idx >= arrayLength(&data) {{
        return;
    }}
    data[idx] = scale_bias(params.scale, params.bias, data[idx]);
}}
"#
  );

  // Step 3: Write the combined WGSL to a file in the build output directory.
  let wgsl_out_path = out_dir.join("scale_bias_compute.wgsl");
  fs::write(&wgsl_out_path, &full_wgsl).into_diagnostic()?;

  // Step 4: Run wgsl-bindgen to generate Rust bindings from the combined WGSL.
  WgslBindgenOptionBuilder::default()
    .workspace_root(out_dir.clone())
    .add_entry_point(wgsl_out_path.to_str().unwrap())
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .output(manifest_dir.join("src/shader_bindings.rs"))
    .build()
    .into_diagnostic()?
    .generate()
    .into_diagnostic()?;

  Ok(())
}
