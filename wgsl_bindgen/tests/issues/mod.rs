use std::fs::read_to_string;

use miette::{IntoDiagnostic, Result};
use syn::parse_str;
use wgsl_bindgen::{assert_tokens_snapshot, *};

#[test]
fn test_issue_35_short_constructor() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/issues/issue_35")
    .add_entry_point("tests/shaders/issues/issue_35/clear.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .short_constructor(2)
    .shader_source_type(WgslShaderSourceType::EmbedSource)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/issues/issue_35.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/issues/issue_35.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_builtin_vertex_encase_issue() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/issues")
    .add_entry_point("tests/shaders/issues/builtin_vertex_issue.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Encase)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/issues/builtin_vertex_encase.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/issues/builtin_vertex_encase.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  // assert_rust_compilation!(parsed_output); // TODO: Fix this test
  Ok(())
}

#[test]
fn test_mixed_builtin_encase_issue() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/issues")
    .add_entry_point("tests/shaders/issues/mixed_builtin_issue.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Encase)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/issues/mixed_builtin_encase.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/issues/mixed_builtin_encase.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  // assert_rust_compilation!(parsed_output); // TODO: Fix this test
  Ok(())
}

#[test]
fn test_builtin_vertex_bytemuck_issue() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/issues")
    .add_entry_point("tests/shaders/issues/builtin_vertex_bytemuck.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/issues/builtin_vertex_bytemuck.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/issues/builtin_vertex_bytemuck.actual.rs").unwrap();

  // Verify the struct is empty and doesn't have problematic padding bytes
  assert!(actual.contains("pub struct VertexInput {}"));

  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_multiple_vertex_shaders_issue() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/issues")
    .add_entry_point("tests/shaders/issues/multiple_vertex_shaders.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/issues/multiple_vertex_shaders.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/issues/multiple_vertex_shaders.actual.rs").unwrap();

  // Verify correct behavior:
  // 1. Both VertexInput and InstanceInput should have VERTEX_ATTRIBUTES implementations
  assert!(actual.contains("impl VertexInput"));
  assert!(actual.contains("impl InstanceInput"));
  assert!(actual.contains("pub const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 1]"));

  // 2. dummy_vertex_shader_entry should have VertexEntry<1> with only VertexInput
  assert!(actual.contains(
    "fn dummy_vertex_shader_entry(vertex_input: wgpu::VertexStepMode) -> VertexEntry<1>"
  ));

  // 3. dummy_instanced_vertex_shader_entry should have VertexEntry<2> with both inputs
  let has_correct_instanced_entry = actual.contains("fn dummy_instanced_vertex_shader_entry(\n    vertex_input: wgpu::VertexStepMode,\n    instance_input: wgpu::VertexStepMode,\n  ) -> VertexEntry<2>") ||
                                   actual.contains("fn dummy_instanced_vertex_shader_entry(vertex_input: wgpu::VertexStepMode, instance_input: wgpu::VertexStepMode) -> VertexEntry<2>");
  assert!(has_correct_instanced_entry);

  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_vec3a_padding_overflow_issue() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/issues")
    .add_entry_point("tests/shaders/issues/vec3a_padding_issue.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/issues/vec3a_padding_overflow.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/issues/vec3a_padding_overflow.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_duplicate_import_vertexinput_issue() -> Result<()> {
  // This test reproduces the issue where importing the same VertexInput struct
  // into multiple shader files causes "duplicate content found" error
  println!("Testing duplicate import issue...");

  let actual = WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/issues/duplicate_import")
    .add_entry_point("tests/shaders/issues/duplicate_import/shader1.wgsl")
    .add_entry_point("tests/shaders/issues/duplicate_import/shader2.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/issues/duplicate_import.actual.rs")
    .build()
    .unwrap()
    .generate_string()
    .into_diagnostic()
    .unwrap();

  // For now, just check that we got some output - we'll refine assertions once we see what's generated
  assert!(!actual.is_empty(), "Generated code should not be empty");

  // Try to parse and compile
  let parsed_output = parse_str(&actual).unwrap();
  assert_rust_compilation!(parsed_output);

  Ok(())
}
