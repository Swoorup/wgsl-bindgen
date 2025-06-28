use std::fs::read_to_string;

use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::*;

#[test]
fn test_issue_35() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("test/shaders/issue_35")
    .add_entry_point("tests/shaders/issue_35/clear.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .short_constructor(2)
    .shader_source_type(WgslShaderSourceType::EmbedWithNagaOilComposer)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/issue_35.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/issue_35.actual.rs").unwrap();
  insta::assert_snapshot!("issue_35", actual);
  Ok(())
}

#[test]
fn test_builtin_vertex_encase_issue() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders")
    .add_entry_point("tests/shaders/builtin_vertex_issue.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Encase)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/builtin_vertex_issue.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/builtin_vertex_issue.actual.rs").unwrap();
  insta::assert_snapshot!("builtin_vertex_encase_issue", actual);
  Ok(())
}

#[test]
fn test_mixed_builtin_encase_issue() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders")
    .add_entry_point("tests/shaders/mixed_builtin_issue.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Encase)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/mixed_builtin_issue.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/mixed_builtin_issue.actual.rs").unwrap();
  insta::assert_snapshot!("mixed_builtin_encase_issue", actual);
  Ok(())
}

#[test]
fn test_builtin_vertex_bytemuck_issue() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders")
    .add_entry_point("tests/shaders/builtin_vertex_bytemuck.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/builtin_vertex_bytemuck.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/builtin_vertex_bytemuck.actual.rs").unwrap();

  // Verify the struct is empty and doesn't have problematic padding bytes
  assert!(actual.contains("pub struct VertexInput {}"));

  insta::assert_snapshot!("builtin_vertex_bytemuck_issue", actual);
  Ok(())
}

#[test]
fn test_multiple_vertex_shaders_issue() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders")
    .add_entry_point("tests/shaders/multiple_vertex_shaders.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/multiple_vertex_shaders.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/multiple_vertex_shaders.actual.rs").unwrap();

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

  insta::assert_snapshot!("multiple_vertex_shaders_issue", actual);
  Ok(())
}
