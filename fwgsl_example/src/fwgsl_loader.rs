//! Runtime fwgsl → WGSL loader for hot-reloading via `ComposerWithRelativePath`.
//!
//! The [`make_fwgsl_load_file`] factory returns a closure that is compatible
//! with the `load_file` parameter of the generated
//! `create_shader_module_relative_path` function.
//!
//! For any file that ends in `.fwgsl` the closure:
//!   1. Reads the file from disk.
//!   2. Splits on the first `-- @wgsl` line (the same convention used at
//!      build time in `build.rs`).
//!   3. Compiles the fwgsl portion through the full fwgsl pipeline.
//!   4. Appends the raw WGSL section and returns the combined string.
//!
//! For any other extension the closure falls back to a plain
//! `std::fs::read_to_string`.
//!
//! ## Hot-reload pattern
//!
//! ```rust,no_run
//! use fwgsl_example::fwgsl_loader::make_fwgsl_load_file;
//!
//! // device: &wgpu::Device (from your wgpu initialisation)
//! // Call this every frame (or whenever you detect a file change) to
//! // get a freshly compiled shader module:
//! //
//! //   let module = shader_bindings::shaders::scale_bias::create_shader_module_relative_path(
//! //       device,
//! //       base_dir,
//! //       Default::default(),
//! //       make_fwgsl_load_file(),
//! //   ).expect("fwgsl → WGSL → shader module");
//! ```

use std::io;

/// The `-- @wgsl` marker that separates the fwgsl section from the raw WGSL
/// section inside a `.fwgsl` file.
const WGSL_SECTION_MARKER: &str = "-- @wgsl";

// ─────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────

/// Returns a `load_file` closure that compiles `.fwgsl` files on the fly.
///
/// Pass the returned closure to `create_shader_module_relative_path` whenever
/// you want the shader source to be read from disk at runtime — enabling
/// live hot-reloading without restarting the process or rerunning `build.rs`.
///
/// # Example
///
/// ```rust,no_run
/// use fwgsl_example::fwgsl_loader::make_fwgsl_load_file;
/// // let module = shaders::scale_bias::create_shader_module_relative_path(
/// //     &device, base_dir, Default::default(), make_fwgsl_load_file()
/// // ).expect("hot-reload failed");
/// ```
pub fn make_fwgsl_load_file() -> impl Fn(&str) -> Result<String, io::Error> + Clone {
  move |path: &str| {
    let raw = std::fs::read_to_string(path)?;

    if std::path::Path::new(path)
      .extension()
      .and_then(|e| e.to_str())
      != Some("fwgsl")
    {
      // Plain WGSL (or any other file type) — return as-is.
      return Ok(raw);
    }

    // Split on the first `-- @wgsl` line.
    let (fwgsl_source, wgsl_section) = split_fwgsl_source(&raw);

    let compiled = compile_fwgsl(fwgsl_source).map_err(|e| {
      io::Error::new(
        io::ErrorKind::InvalidData,
        format!("fwgsl compile error in {path}: {e}"),
      )
    })?;

    Ok(if wgsl_section.is_empty() {
      compiled
    } else {
      format!("{}\n{}", compiled, wgsl_section)
    })
  }
}

// ─────────────────────────────────────────────────────────────────
// fwgsl compilation pipeline (mirrors build.rs)
// ─────────────────────────────────────────────────────────────────

/// Split a `.fwgsl` file on the first `-- @wgsl` marker.
///
/// Returns `(fwgsl_part, wgsl_part)`.  If no marker is found the entire
/// source is treated as fwgsl and the WGSL section is empty.
fn split_fwgsl_source(source: &str) -> (&str, &str) {
  // Find the byte offset of the marker line, if any.
  let mut fwgsl_end = source.len();
  let mut wgsl_start = source.len();

  for line in source.split_inclusive('\n') {
    let trimmed = line.trim_end_matches('\n').trim_end_matches('\r').trim();
    if trimmed == WGSL_SECTION_MARKER {
      let offset = line.as_ptr() as usize - source.as_ptr() as usize;
      fwgsl_end = offset;
      wgsl_start = offset + line.len();
      break;
    }
  }

  (&source[..fwgsl_end], &source[wgsl_start..])
}

/// Compile a fwgsl source string to WGSL with `// @fwgsl-adt:` annotations.
fn compile_fwgsl(source: &str) -> Result<String, String> {
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
    return Err(format!("fwgsl parse errors: {}", errors.join(", ")));
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
    return Err(format!("fwgsl semantic errors: {}", errors.join(", ")));
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
    return Err(format!("fwgsl lowering errors: {}", errors.join(", ")));
  }

  // Phase 4: HIR → MIR
  let mir = fwgsl_mir::lower::lower_hir_to_mir(&hir)
    .map_err(|errors| format!("fwgsl MIR errors: {errors:?}"))?;

  // Phase 5: MIR → WGSL
  let raw_wgsl = fwgsl_wgsl_codegen::emit_wgsl(&mir);

  // Phase 6: Inject `// @fwgsl-adt:` annotations from the HIR
  Ok(inject_adt_annotations(&raw_wgsl, &hir))
}

/// Prepend `// @fwgsl-adt:` annotations for every user-defined ADT.
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

    fn extract_tag_from_token(tok: &str) -> u32 {
      tok.split(':').nth(1).and_then(|s| s.parse().ok()).unwrap_or(u32::MAX)
    }
    variant_tokens.sort_by_key(|tok: &String| extract_tag_from_token(tok));

    annotations.push_str(&format!(
      "// @fwgsl-adt: {} {}\n",
      dt.name,
      variant_tokens.join(" ")
    ));
  }

  format!("{}{}", annotations, wgsl)
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn split_no_marker_returns_whole_source_as_fwgsl() {
    let src = "scale_val : F32 -> F32 -> F32\nscale_val s x = x * s\n";
    let (fwgsl, wgsl) = split_fwgsl_source(src);
    assert_eq!(fwgsl, src);
    assert!(wgsl.is_empty());
  }

  #[test]
  fn split_with_marker_separates_sections() {
    let src = "scale_val : F32 -> F32 -> F32\nscale_val s x = x * s\n-- @wgsl\nalias Foo = u32;\n";
    let (fwgsl, wgsl) = split_fwgsl_source(src);
    assert!(fwgsl.contains("scale_val"));
    assert!(!fwgsl.contains("@wgsl"));
    assert!(wgsl.contains("alias Foo"));
    assert!(!wgsl.contains("scale_val"));
  }

  #[test]
  fn compile_simple_fwgsl_returns_wgsl() {
    let src = "double : F32 -> F32\ndouble x = x * 2.0\n";
    let wgsl = compile_fwgsl(src).expect("compile should succeed");
    assert!(wgsl.contains("fn double"), "expected fn double in: {wgsl}");
  }
}
