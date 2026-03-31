//! Build script for the fwgsl_example crate.
//!
//! This build script demonstrates integrating fwgsl (https://github.com/ubugeeei/fwgsl)
//! with wgsl-bindgen. The pipeline is:
//!
//!   1. Read `.fwgsl` source files (pure functional shader language)
//!   2. Compile them to WGSL using the fwgsl compiler library
//!   3. Extract ADT (algebraic data type) metadata from the fwgsl semantic analysis
//!   4. Combine the generated WGSL helper functions with hand-written bind group
//!      declarations and a compute entry point
//!   5. Run wgsl-bindgen on the combined WGSL to generate type-safe Rust bindings,
//!      passing the extracted ADT info so it can also emit Rust enums

use std::env;
use std::fs;
use std::path::PathBuf;

use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{WgslBindgenOptionBuilder, WgslEnumDefinition, WgslEnumVariant, WgslTypeSerializeStrategy};

/// The result of compiling a `.fwgsl` file.
struct FwgslOutput {
  /// The generated WGSL source text (helper functions only; no entry points).
  wgsl: String,
  /// Simple (no-field) algebraic data types extracted from the semantic analysis.
  /// Each entry is `(type_name, [(constructor_name, discriminant), ...])`.
  enums: Vec<WgslEnumDefinition>,
}

/// Compile a fwgsl source string to WGSL using the fwgsl compiler pipeline.
/// Also returns the ADT enum metadata so callers can pass it to wgsl-bindgen.
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

  // Phase 2: Semantic analysis — also captures data type / constructor info
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

  // Extract simple enum ADTs (constructors with no fields) before we lose the analyzer.
  //
  // fwgsl lowers simple enums to bare `u32` discriminant values in WGSL — there is no
  // type alias or struct emitted for them.  We collect the names and tags here so that:
  //   a) we can emit `alias <Name> = u32;` in the WGSL to make it valid for naga, and
  //   b) we can pass them to wgsl-bindgen so it emits matching Rust #[repr(u32)] enums.
  let mut enums: Vec<WgslEnumDefinition> = Vec::new();

  for (type_name, data_info) in &analyzer.data_types {
    // Skip built-in ADTs such as Option / Result / Pair — they are not user-defined.
    let builtin_names = ["Option", "Result", "Pair"];
    if builtin_names.contains(&type_name.as_str()) {
      continue;
    }

    // Only process simple enums: all constructors must have no fields.
    let all_empty = data_info
      .constructors
      .iter()
      .all(|con_name| {
        analyzer
          .constructors
          .get(con_name)
          .is_some_and(|ci| matches!(ci.fields, fwgsl_typechecker::ConstructorFields::Empty))
      });

    if !all_empty {
      continue;
    }

    let mut variants: Vec<WgslEnumVariant> = data_info
      .constructors
      .iter()
      .filter_map(|con_name| {
        let ci = analyzer.constructors.get(con_name)?;
        Some(WgslEnumVariant {
          name: con_name.clone(),
          discriminant: ci.tag,
        })
      })
      .collect();

    // Sort by discriminant so the output is deterministic.
    variants.sort_by_key(|v| v.discriminant);

    enums.push(WgslEnumDefinition {
      name: type_name.clone(),
      variants,
    });
  }

  // Sort enums by name for deterministic output.
  enums.sort_by(|a, b| a.name.cmp(&b.name));

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
  let wgsl = fwgsl_wgsl_codegen::emit_wgsl(&mir);

  Ok(FwgslOutput { wgsl, enums })
}

/// Build the WGSL source for the scale_bias compute shader.
///
/// Combines the fwgsl-generated helper functions with hand-written bind group
/// declarations and a compute entry point.
fn build_scale_bias_wgsl(manifest_dir: &std::path::Path) -> Result<(String, Vec<WgslEnumDefinition>)> {
  let fwgsl_path = manifest_dir.join("shaders/scale_bias.fwgsl");
  println!("cargo::rerun-if-changed={}", fwgsl_path.display());

  let source = fs::read_to_string(&fwgsl_path).into_diagnostic()?;
  let out = compile_fwgsl(&source)?;
  let generated_fns = out.wgsl;

  let full_wgsl = format!(
    r#"// Combined WGSL for the scale_bias compute shader.
//
// Helper functions (scale_val, bias_val, scale_bias, saturate) are generated
// from shaders/scale_bias.fwgsl via the fwgsl compiler.
// The struct, bindings, and entry point below are hand-written because fwgsl
// does not yet emit @group/@binding annotations.

struct ScaleBiasParams {{
    scale: f32,
    bias: f32,
}}

@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@group(0) @binding(1) var<uniform> params: ScaleBiasParams;

// --- fwgsl-generated helper functions ---
{generated_fns}
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
  );

  Ok((full_wgsl, out.enums))
}

/// Build the WGSL source for the color compute shader.
///
/// This shader uses fwgsl algebraic data types (ADTs).  The build step:
///  1. Compiles the fwgsl source to get the WGSL helper functions.
///  2. Emits `alias <EnumName> = u32;` declarations so naga can validate the WGSL.
///  3. Returns the enum metadata so wgsl-bindgen can emit Rust `#[repr(u32)]` enums.
fn build_color_wgsl(manifest_dir: &std::path::Path) -> Result<(String, Vec<WgslEnumDefinition>)> {
  let fwgsl_path = manifest_dir.join("shaders/color_compute.fwgsl");
  println!("cargo::rerun-if-changed={}", fwgsl_path.display());

  let source = fs::read_to_string(&fwgsl_path).into_diagnostic()?;
  let out = compile_fwgsl(&source)?;

  // Build `alias <EnumName> = u32;` declarations so the generated WGSL is valid.
  // fwgsl lowering keeps the ADT type names in function signatures (e.g. `c: Color`)
  // but does not emit the corresponding type definition — we need to do that here.
  let alias_decls: String = out
    .enums
    .iter()
    .map(|e| format!("alias {} = u32;\n", e.name))
    .collect::<Vec<_>>()
    .join("");

  let generated_fns = out.wgsl;

  let full_wgsl = format!(
    r#"// Combined WGSL for the color_compute shader.
//
// The color_to_r / color_to_g / color_to_b helper functions are generated from
// shaders/color_compute.fwgsl via the fwgsl compiler.
//
// The `Color` ADT is a simple enum in fwgsl, which compiles to `u32` discriminant
// values (Red=0, Green=1, Blue=2).  The `alias Color = u32;` below makes the WGSL
// valid for naga.  The corresponding Rust `#[repr(u32)] enum Color` is emitted by
// wgsl-bindgen using the ADT metadata extracted from the fwgsl semantic analyzer.

// Type aliases for fwgsl ADTs (simple enums → u32)
{alias_decls}
struct ColorParams {{
    /// The selected Color variant (0=Red, 1=Green, 2=Blue).
    color_tag: u32,
}}

/// Output buffer: will hold the decoded [R, G, B, A] components.
@group(0) @binding(0) var<storage, read_write> output: array<f32, 4>;
/// Uniform buffer holding the color selector.
@group(0) @binding(1) var<uniform> params: ColorParams;

// --- fwgsl-generated helper functions ---
{generated_fns}
// --- end fwgsl-generated code ---

// Workgroup size of 1 is intentional: this shader decodes a single Color tag
// into [R, G, B, A] components. A real production shader would batch many
// elements and use a larger workgroup size for GPU efficiency.
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) _global_id: vec3<u32>) {{
    let selected_color: Color = params.color_tag;
    output[0] = color_to_r(selected_color);
    output[1] = color_to_g(selected_color);
    output[2] = color_to_b(selected_color);
    output[3] = 1.0;
}}
"#
  );

  Ok((full_wgsl, out.enums))
}

fn main() -> Result<()> {
  let out_dir = PathBuf::from(env::var("OUT_DIR").into_diagnostic()?);
  let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").into_diagnostic()?);

  // Build both shaders
  let (scale_bias_wgsl, scale_bias_enums) = build_scale_bias_wgsl(&manifest_dir)?;
  let (color_wgsl, color_enums) = build_color_wgsl(&manifest_dir)?;

  // Write the combined WGSL files to the build output directory
  let scale_bias_path = out_dir.join("scale_bias_compute.wgsl");
  let color_path = out_dir.join("color_compute.wgsl");
  fs::write(&scale_bias_path, &scale_bias_wgsl).into_diagnostic()?;
  fs::write(&color_path, &color_wgsl).into_diagnostic()?;

  // Run wgsl-bindgen on both shaders together.
  // The Color enum metadata from the color shader is passed as a custom enum so that
  // wgsl-bindgen emits a matching Rust #[repr(u32)] enum in the output file.
  let mut builder = WgslBindgenOptionBuilder::default();
  builder
    .workspace_root(out_dir.clone())
    .add_entry_point(scale_bias_path.to_str().unwrap())
    .add_entry_point(color_path.to_str().unwrap())
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .output(manifest_dir.join("src/shader_bindings.rs"));

  // Register all extracted enums from both shaders
  for e in scale_bias_enums.into_iter().chain(color_enums) {
    builder.add_custom_enum(e);
  }

  builder.build().into_diagnostic()?.generate().into_diagnostic()?;

  Ok(())
}

