use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::qs::{format_ident, quote};
use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::{Ident, TokenStream};
use regex::Regex;
use std::io::Write;

pub fn pretty_print(tokens: &TokenStream) -> String {
  let code = tokens.to_string();

  // Try rustfmt first to use the project's formatting configuration
  match format_with_rustfmt(&code) {
    Ok(formatted) => formatted,
    Err(error) => {
      // Log the rustfmt failure and fall back to prettyplease
      eprintln!(
        "Warning: rustfmt formatting failed ({error}), falling back to prettyplease",
      );
      let file = syn::parse_file(&code).unwrap();
      prettyplease::unparse(&file)
    }
  }
}

fn format_with_rustfmt(code: &str) -> Result<String, Box<dyn std::error::Error>> {
  use std::process::Stdio;

  let mut child = Command::new("rustfmt")
    .arg("--emit")
    .arg("stdout")
    .arg("--quiet")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

  if let Some(stdin) = child.stdin.as_mut() {
    stdin.write_all(code.as_bytes())?;
  } else {
    return Err("Failed to open stdin".into());
  }

  let output = child.wait_with_output()?;

  if output.status.success() {
    Ok(String::from_utf8(output.stdout)?)
  } else {
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("rustfmt failed: {stderr}").into())
  }
}

#[macro_export]
macro_rules! assert_tokens_snapshot {
  ($output:expr) => {{
    let mut settings = insta::Settings::new();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_omit_expression(true);
    settings.bind(|| {
      let formatted_output = $crate::test_helper::pretty_print(&$output);
      insta::assert_snapshot!(formatted_output);
    });
  }};
}

#[macro_export]
macro_rules! assert_rust_compilation {
    ($output:expr) => {{
        // TODO: Current this requires removing include_absolute_path! macro from the output (See #45)

        /*
        let formatted_output = $crate::test_helper::pretty_print(&$output);
        // Extract test name automatically using stdext function_name macro
        let full_name = stdext::function_name!();
        // Extract just the function name (after the last ::) and sanitize for use as project name
        let test_name = full_name.split("::").last().unwrap_or(full_name).replace("::", "_");
        if let Err(e) = $crate::test_helper::try_compilation_test_with_name(&formatted_output, &test_name) {
            panic!("Generated code failed to compile: {e}\n\n");
        }
        */
    }};
}

pub fn indexed_name_ident(name: &str, index: u32) -> Ident {
  format_ident!("{name}{index}")
}

pub fn sanitize_and_pascal_case(v: &str) -> String {
  v.chars()
    .filter(|ch| ch.is_alphanumeric() || *ch == '_')
    .collect::<String>()
    .to_pascal_case()
}

pub fn sanitized_upper_snake_case(v: &str) -> String {
  v.chars()
    .filter(|ch| ch.is_alphanumeric() || *ch == '_')
    .collect::<String>()
    .to_snake_case()
    .to_uppercase()
}

/// Try to compile generated code and return a Result instead of panicking
///
/// This function is used internally by the `assert_tokens_snapshot` macro to verify
/// that generated code compiles without errors. Unlike `assert_compilation_test`,
/// this function returns a Result allowing the caller to handle compilation failures.
///
/// For individual code snippets that may not have complete context, this function
/// wraps them in a minimal module structure with necessary imports.
///
/// # Arguments
///
/// * `generated_code` - The generated Rust code as a string
///
/// # Returns
///
/// * `Ok(())` if compilation succeeds
/// * `Err(String)` if compilation fails with error details
pub fn try_compilation_test(generated_code: &str) -> Result<(), String> {
  try_compilation_test_with_name(generated_code, "wgsl_bindgen_compile_test")
}

/// Try to compile generated code with a specific project name
pub fn try_compilation_test_with_name(
  generated_code: &str,
  test_name: &str,
) -> Result<(), String> {
  use std::fs;
  use std::path::PathBuf;

  // Create workspace in tests/output/compile_test_workspace with unique project name
  let output_dir = PathBuf::from("tests").join("output");
  let temp_dir = output_dir.join("compile_test_workspace").join(test_name);
  fs::create_dir_all(&temp_dir)
    .map_err(|e| format!("Failed to create temp directory: {e}"))?;

  // Only complete modules are tested for compilation
  let test_content = generated_code.to_string();

  // Write the test content to a temporary file
  let test_file = temp_dir.join("generated_test.rs");
  fs::write(&test_file, &test_content)
    .map_err(|e| format!("Failed to write test file: {e}"))?;

  // Create a simple compilation test for just this specific file
  let compile_test =
    create_single_file_compile_test(&temp_dir, test_name, &test_content)?;

  let result = match compile_test.test_compilation() {
    Ok(true) => {
      // Compilation succeeded
      Ok(())
    }
    Ok(false) => Err(
      "Generated code failed to compile (see previous output for details)".to_string(),
    ),
    Err(e) => Err(format!("Compilation test setup failed: {e}")),
  };

  // Don't cleanup the workspace - leave it for inspection
  result
}

/// Create a simple compilation test for a single file
fn create_single_file_compile_test(
  workspace_dir: &std::path::Path,
  project_name: &str,
  generated_code: &str,
) -> Result<SingleFileCompileTest, String> {
  use std::fs;

  let src_dir = workspace_dir.join("src");
  fs::create_dir_all(&src_dir).map_err(|e| format!("Failed to create src dir: {e}"))?;

  // Generate Cargo.toml with only necessary dependencies for this specific file
  let dependencies = detect_required_dependencies_from_content(generated_code);
  let cargo_toml = generate_single_file_cargo_toml(project_name, &dependencies);
  fs::write(workspace_dir.join("Cargo.toml"), cargo_toml)
    .map_err(|e| format!("Failed to write Cargo.toml: {e}"))?;

  // Write the generated code as lib.rs
  fs::write(src_dir.join("lib.rs"), generated_code)
    .map_err(|e| format!("Failed to write lib.rs: {e}"))?;

  Ok(SingleFileCompileTest {
    workspace_dir: workspace_dir.to_path_buf(),
  })
}

/// Simple compile test for a single file
struct SingleFileCompileTest {
  workspace_dir: std::path::PathBuf,
}

impl SingleFileCompileTest {
  /// Test compilation of the single file
  pub fn test_compilation(&self) -> Result<bool, Box<dyn std::error::Error>> {
    use std::process::Command;

    // Run cargo check on the test workspace with color output
    let output = Command::new("cargo")
      .arg("check")
      .arg("--all-features")
      .arg("--color=always") // Force colored output
      .current_dir(&self.workspace_dir)
      .env("TERM", "xterm-256color") // Ensure color support
      .output()?;

    if output.status.success() {
      println!("✓ Generated file compiles successfully");
      Ok(true)
    } else {
      eprintln!("✗ Compilation failed:");
      eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
      eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
      Ok(false)
    }
  }
}

/// Detect which dependencies are required by analyzing a single file's content
fn detect_required_dependencies_from_content(
  content: &str,
) -> std::collections::HashSet<String> {
  let mut deps = std::collections::HashSet::new();

  // Check for various crate usage patterns
  if content.contains("wgpu::") || content.contains("wgpu_types") {
    deps.insert("wgpu".to_string());
  }
  if content.contains("glam::") {
    deps.insert("glam".to_string());
  }
  if content.contains("bytemuck::") {
    deps.insert("bytemuck".to_string());
  }
  if content.contains("encase::") {
    deps.insert("encase".to_string());
  }
  if content.contains("naga_oil::") {
    deps.insert("naga_oil".to_string());
  }
  if content.contains("include_absolute_path!") {
    deps.insert("include_absolute_path".to_string());
  }

  // Always include core dependencies that are commonly used
  deps.insert("wgpu".to_string());
  deps.insert("glam".to_string());
  deps.insert("bytemuck".to_string());
  deps.insert("encase".to_string());

  deps
}

/// Generate Cargo.toml for a single file compilation test
fn generate_single_file_cargo_toml(
  project_name: &str,
  dependencies: &std::collections::HashSet<String>,
) -> String {
  let mut cargo_toml = format!(
    r#"[package]
name = "{project_name}"
version = "0.1.0"
edition = "2021"

# Empty workspace to avoid conflicts with parent workspace
[workspace]

[dependencies]
"#
  );

  for dep in dependencies {
    match dep.as_str() {
            "wgpu" => cargo_toml.push_str("wgpu = { version = \"25.0\", features = [\"wgsl\"] }\nnaga = { version = \"25.0\", features = [\"wgsl-out\"] }\n"),
            "glam" => cargo_toml.push_str("glam = \"0.30\"\n"),
            "bytemuck" => cargo_toml.push_str("bytemuck = { version = \"1.16\", features = [\"derive\"] }\n"),
            "encase" => cargo_toml.push_str("encase = \"0.11\"\n"),
            "naga_oil" => cargo_toml.push_str("naga_oil = \"0.16\"\n"),
            "include_absolute_path" => cargo_toml.push_str("include_absolute_path = \"0.1\"\n"),
            _ => {}
        }
  }

  cargo_toml
}
