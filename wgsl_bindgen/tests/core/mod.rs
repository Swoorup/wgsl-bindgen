use std::fs::read_to_string;

use miette::{IntoDiagnostic, Result};
use syn::parse_str;
use wgsl_bindgen::{
  assert_rust_compilation, assert_tokens_snapshot, GlamWgslTypeMap, Regex,
  WgslBindgenOptionBuilder, WgslShaderSourceType, WgslTypeSerializeStrategy,
};

#[test]
fn test_basic_bindgen() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/core/basic/main.wgsl")
    .workspace_root("tests/shaders/core")
    .additional_scan_dir((None, "tests/shaders/core/additional"))
    .override_struct_alignment([("main::Style", 256)].map(Into::into))
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .ir_capabilities(naga::valid::Capabilities::PUSH_CONSTANT)
    .shader_source_type(
      WgslShaderSourceType::EmbedSource | WgslShaderSourceType::ComposerWithRelativePath,
    )
    .output("tests/output/core/basic_bindgen.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/core/basic_bindgen.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_struct_alignment() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/core/minimal.wgsl")
    .workspace_root("tests/shaders/core")
    .override_struct_alignment([(".*::Uniforms", 256)].map(Into::into))
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/core/struct_alignment.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/core/struct_alignment.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_custom_padding() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/core/padding.wgsl")
    .workspace_root("tests/shaders/core")
    .add_custom_padding_field_regexp(Regex::new("_padding").unwrap())
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/core/custom_padding.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/core/custom_padding.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_struct_layouts() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/core/layouts.wgsl")
    .workspace_root("tests/shaders/core")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .override_bind_group_entry_module_path(
      [("color_texture", "bindings"), ("color_sampler", "bindings")].map(Into::into),
    )
    .output("tests/output/core/struct_layouts.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/core/struct_layouts.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_relative_path_composer() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/core/basic/main.wgsl")
    .workspace_root("tests/shaders/core/additional")
    .additional_scan_dir((None, "tests/shaders/core/additional"))
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .ir_capabilities(naga::valid::Capabilities::PUSH_CONSTANT)
    .shader_source_type(WgslShaderSourceType::ComposerWithRelativePath)
    .output("tests/output/core/relative_path_composer.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/core/relative_path_composer.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_module_path_generation() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/core")
    .add_entry_point("tests/shaders/core/lines/segment.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/core/module_path_generation.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/core/module_path_generation.actual.rs").unwrap();

  // Verify the module path is lines::segment
  assert!(actual.contains("pub mod lines {"), "Should have lines module");
  assert!(actual.contains("pub mod segment {"), "Should have segment submodule");

  // Verify ShaderEntry enum variant includes module prefix
  assert!(actual.contains("LinesSegment,"), "ShaderEntry variant should be LinesSegment");
  assert!(
    actual.contains("Self::LinesSegment"),
    "Should use Self::LinesSegment in match arms"
  );

  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_shader_visibility_merging() -> Result<()> {
  let actual = WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/core/shared_visibility/compute_shader.wgsl")
    .add_entry_point("tests/shaders/core/shared_visibility/render_shader.wgsl")
    .workspace_root("tests/shaders/core/shared_visibility")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .shader_source_type(WgslShaderSourceType::EmbedSource)
    .output("tests/output/core/shader_visibility_merging.actual.rs".to_string())
    .build()?
    .generate_string()?;

  // Check that the common bind group has combined visibility
  // The generated code now uses union() method for const-compatible chaining
  assert!(
    actual.contains("visibility: wgpu::ShaderStages::VERTEX\n              .union(wgpu::ShaderStages::FRAGMENT)\n              .union(wgpu::ShaderStages::COMPUTE)") ||
    actual.contains("visibility: wgpu::ShaderStages::COMPUTE\n              .union(wgpu::ShaderStages::VERTEX)\n              .union(wgpu::ShaderStages::FRAGMENT)") ||
    actual.contains("visibility: wgpu::ShaderStages::all()"),
    "Common bind group should have combined visibility for COMPUTE, VERTEX, and FRAGMENT stages"
  );

  // Also ensure it compiled successfully
  let parsed_output = parse_str(&actual).unwrap();
  assert_rust_compilation!(parsed_output);

  Ok(())
}

#[test]
#[ignore = "It doesn't like path symbols inside a nested type like array."]
fn test_path_import() -> Result<()> {
  let _ = WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/core/basic/path_import.wgsl")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .build()?
    .generate_string()
    .into_diagnostic()?;

  Ok(())
}
