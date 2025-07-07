use std::fs::read_to_string;

use miette::{IntoDiagnostic, Result};
use syn::parse_str;
use wgsl_bindgen::{assert_tokens_snapshot, *};

#[test]
fn test_shader_defs_basic() -> Result<()> {
  let shader_defs = vec![
    ("USE_TIME".to_string(), ShaderDefValue::Bool(true)),
    ("USE_SCALE".to_string(), ShaderDefValue::Bool(true)),
    ("DEBUG_MODE".to_string(), ShaderDefValue::Bool(false)),
  ];

  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/features/shader_defs")
    .entry_points(vec!["tests/shaders/features/shader_defs/test_shader.wgsl".to_string()])
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .shader_source_type(WgslShaderSourceType::EmbedSource)
    .add_shader_defs(shader_defs)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/features/shader_defs_basic.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/features/shader_defs_basic.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_shader_defs_with_texture() -> Result<()> {
  let shader_defs = vec![
    ("USE_TIME".to_string(), ShaderDefValue::Bool(true)),
    ("USE_SCALE".to_string(), ShaderDefValue::Bool(true)),
    ("USE_TEXTURE".to_string(), ShaderDefValue::Bool(true)),
    ("DEBUG_MODE".to_string(), ShaderDefValue::Bool(false)),
  ];

  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/features/shader_defs")
    .entry_points(vec!["tests/shaders/features/shader_defs/test_shader.wgsl".to_string()])
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .shader_source_type(WgslShaderSourceType::ComposerWithRelativePath)
    .add_shader_defs(shader_defs)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/features/shader_defs_with_texture.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/features/shader_defs_with_texture.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_shader_defs_minimal() -> Result<()> {
  // Test with minimal shader_defs (no optional features enabled)
  let shader_defs = vec![("DEBUG_MODE".to_string(), ShaderDefValue::Bool(false))];

  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/features/shader_defs")
    .entry_points(vec!["tests/shaders/features/shader_defs/test_shader.wgsl".to_string()])
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .shader_source_type(WgslShaderSourceType::ComposerWithRelativePath)
    .add_shader_defs(shader_defs)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/features/shader_defs_minimal.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/features/shader_defs_minimal.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_shader_defs_builder_methods() -> Result<()> {
  // Test the builder helper methods
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/features/shader_defs")
    .entry_points(vec!["tests/shaders/features/shader_defs/test_shader.wgsl".to_string()])
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .shader_source_type(WgslShaderSourceType::ComposerWithRelativePath)
    .add_shader_def("USE_TIME", ShaderDefValue::Bool(true))
    .add_shader_def("USE_SCALE", ShaderDefValue::Bool(false))
    .add_shader_def("DEBUG_MODE", ShaderDefValue::Bool(true))
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/features/shader_defs_builder_methods.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/features/shader_defs_builder_methods.actual.rs")
      .unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}
