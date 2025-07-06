use std::fs::read_to_string;

use miette::{IntoDiagnostic, Result};
use syn::parse_str;
use wgsl_bindgen::{
  assert_rust_compilation, assert_tokens_snapshot, GlamWgslTypeMap, Regex,
  WgslBindgenOptionBuilder, WgslShaderSourceType, WgslTypeSerializeStrategy,
};

#[test]
fn test_bevy_bindgen() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .module_import_root("bevy_pbr")
    .workspace_root("tests/shaders/bevy_pbr_wgsl")
    .add_entry_point("tests/shaders/bevy_pbr_wgsl/pbr.wgsl")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/bindgen_bevy.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/bindgen_bevy.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_main_bindgen() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/basic/main.wgsl")
    .workspace_root("tests/shaders")
    .additional_scan_dir((None, "tests/shaders/additional"))
    .override_struct_alignment([("main::Style", 256)].map(Into::into))
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .ir_capabilities(naga::valid::Capabilities::PUSH_CONSTANT)
    .shader_source_type(
      WgslShaderSourceType::EmbedSource | WgslShaderSourceType::ComposerWithRelativePath,
    )
    .output("tests/output/bindgen_main.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/bindgen_main.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_struct_alignment_minimal() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/minimal.wgsl")
    .workspace_root("tests/shaders")
    .override_struct_alignment([(".*::Uniforms", 256)].map(Into::into))
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/bindgen_minimal.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/bindgen_minimal.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_struct_alignment_padding() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/padding.wgsl")
    .workspace_root("tests/shaders")
    .add_custom_padding_field_regexp(Regex::new("_padding").unwrap())
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/bindgen_padding.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/bindgen_padding.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_struct_layouts() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/layouts.wgsl")
    .workspace_root("tests/shaders")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .override_bind_group_entry_module_path(
      [("color_texture", "bindings"), ("color_sampler", "bindings")].map(Into::into),
    )
    .output("tests/output/bindgen_layouts.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/bindgen_layouts.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
fn test_relative_path_bindgen() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/basic/main.wgsl")
    .workspace_root("tests/shaders/additional")
    .additional_scan_dir((None, "tests/shaders/additional"))
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .ir_capabilities(naga::valid::Capabilities::PUSH_CONSTANT)
    .shader_source_type(WgslShaderSourceType::ComposerWithRelativePath)
    .output("tests/output/bindgen_relative_path.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/bindgen_relative_path.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}

#[test]
#[ignore = "It doesn't like path symbols inside a nested type like array."]
fn test_path_import() -> Result<()> {
  let _ = WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/basic/path_import.wgsl")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .build()?
    .generate_string()
    .into_diagnostic()?;

  Ok(())
}

#[test]
fn test_shared_bind_group_visibility() -> Result<()> {
  let actual = WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/shared_bindings_visibility/compute_shader.wgsl")
    .add_entry_point("tests/shaders/shared_bindings_visibility/render_shader.wgsl")
    .workspace_root("tests/shaders/shared_bindings_visibility")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .shader_source_type(WgslShaderSourceType::EmbedSource)
    .output("tests/output/bindgen_shared_visibility.actual.rs".to_string())
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
