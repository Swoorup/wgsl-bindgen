use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::qs::{format_ident, quote};
use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::{Ident, TokenStream};
use regex::Regex;
use std::io::Write;

#[cfg(test)]
use toml;

#[macro_export]
macro_rules! assert_tokens_snapshot {
  ($output:expr) => {{
    let mut settings = insta::Settings::new();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_omit_expression(true);
    settings.bind(|| {
      let formatted_output = $crate::pretty_print(&$output);
      insta::assert_snapshot!(formatted_output);
    });
  }};
}

#[macro_export]
macro_rules! assert_rust_compilation {
    ($output:expr) => {{
        // TODO: Current this requires removing include_absolute_path! macro from the output (See #45)

        /*
        let formatted_output = $crate::pretty_print(&$output);
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

#[macro_export]
macro_rules! assert_rust_compilation_working {
    ($output:expr) => {{
        let formatted_output = $crate::pretty_print(&$output);
        // Extract test name automatically using stdext function_name macro
        let full_name = stdext::function_name!();
        // Extract just the function name (after the last ::) and sanitize for use as project name
        let test_name = full_name.split("::").last().unwrap_or(full_name).replace("::", "_");
        if let Err(e) = $crate::test_helper::try_compilation_test_with_name(&formatted_output, &test_name) {
            panic!("Generated code failed to compile: {e}\n\n");
        }
    }};
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

  // Read workspace Cargo.toml to get the actual dependency versions
  #[cfg(test)]
  let workspace_deps = read_workspace_dependencies().unwrap_or_default();
  #[cfg(not(test))]
  let workspace_deps: std::collections::HashMap<String, String> =
    std::collections::HashMap::new();

  for dep in dependencies {
    match dep.as_str() {
      "wgpu" => {
        let wgpu_version = workspace_deps
          .get("wgpu")
          .map(|s| s.as_str())
          .unwrap_or("25.0");
        let naga_version = workspace_deps
          .get("naga")
          .map(|s| s.as_str())
          .unwrap_or("25.0");
        cargo_toml.push_str(&format!("wgpu = {{ version = \"{wgpu_version}\", features = [\"wgsl\", \"naga-ir\"] }}\nnaga = {{ version = \"{naga_version}\", features = [\"wgsl-out\"] }}\n"));
      }
      "glam" => {
        let version = workspace_deps
          .get("glam")
          .map(|s| s.as_str())
          .unwrap_or("0.30");
        cargo_toml.push_str(&format!("glam = \"{version}\"\n"));
      }
      "bytemuck" => {
        let version = workspace_deps
          .get("bytemuck")
          .map(|s| s.as_str())
          .unwrap_or("1.13");
        cargo_toml.push_str(&format!(
          "bytemuck = {{ version = \"{version}\", features = [\"derive\"] }}\n"
        ));
      }
      "encase" => {
        let version = workspace_deps
          .get("encase")
          .map(|s| s.as_str())
          .unwrap_or("0.11");
        cargo_toml.push_str(&format!("encase = \"{version}\"\n"));
      }
      "naga_oil" => {
        let version = workspace_deps
          .get("naga_oil")
          .map(|s| s.as_str())
          .unwrap_or("0.18");
        cargo_toml.push_str(&format!("naga_oil = \"{version}\"\n"));
      }
      "include_absolute_path" => {
        let version = workspace_deps
          .get("include_absolute_path")
          .map(|s| s.as_str())
          .unwrap_or("0.1");
        cargo_toml.push_str(&format!("include_absolute_path = \"{version}\"\n"));
      }
      _ => {}
    }
  }

  cargo_toml
}

/// Read dependency versions from workspace Cargo.toml
#[cfg(test)]
fn read_workspace_dependencies(
) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
  use std::collections::HashMap;

  // Find workspace root by looking for Cargo.toml with [workspace] section
  let workspace_root = find_workspace_root()?;
  let cargo_toml_path = workspace_root.join("Cargo.toml");

  let content = std::fs::read_to_string(cargo_toml_path)?;

  // Parse TOML using the toml crate
  let parsed: toml::Value = content.parse()?;

  let mut deps = HashMap::new();

  // Navigate to workspace.dependencies section
  if let Some(workspace) = parsed.get("workspace") {
    if let Some(dependencies) = workspace.get("dependencies") {
      if let Some(deps_table) = dependencies.as_table() {
        for (name, value) in deps_table {
          let version = match value {
            // Simple string version: name = "1.0"
            toml::Value::String(version_str) => version_str.clone(),
            // Complex dependency: name = { version = "1.0", features = [...] }
            toml::Value::Table(table) => {
              if let Some(version_value) = table.get("version") {
                if let Some(version_str) = version_value.as_str() {
                  version_str.to_string()
                } else {
                  continue;
                }
              } else {
                continue;
              }
            }
            _ => continue,
          };

          deps.insert(name.clone(), version);
        }
      }
    }
  }

  Ok(deps)
}

/// Find the workspace root directory
#[cfg(test)]
fn find_workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
  let mut current = std::env::current_dir()?;

  loop {
    let cargo_toml = current.join("Cargo.toml");
    if cargo_toml.exists() {
      let content = std::fs::read_to_string(&cargo_toml)?;
      if content.contains("[workspace]") {
        return Ok(current);
      }
    }

    match current.parent() {
      Some(parent) => current = parent.to_path_buf(),
      None => return Err("Could not find workspace root".into()),
    }
  }
}
