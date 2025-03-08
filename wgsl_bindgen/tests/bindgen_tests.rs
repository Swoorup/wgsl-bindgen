use std::fs::read_to_string;

use miette::{IntoDiagnostic, Result};
use pretty_assertions::assert_eq;
use wgsl_bindgen::*;

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
  let expected = read_to_string("tests/output/bindgen_bevy.expected.rs").unwrap();

  assert_eq!(actual, expected);
  Ok(())
}

#[test]
fn test_main_bindgen() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/basic/main.wgsl")
    .workspace_root("tests/shaders/additional")
    .additional_scan_dir((None, "tests/shaders/additional"))
    .override_struct_alignment([("main::Style", 256)].map(Into::into))
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .ir_capabilities(naga::valid::Capabilities::PUSH_CONSTANT)
    .shader_source_type(
      WgslShaderSourceType::EmbedSource
        | WgslShaderSourceType::HardCodedFilePathWithNagaOilComposer,
    )
    .output("tests/output/bindgen_main.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/bindgen_main.actual.rs").unwrap();
  let expected = read_to_string("tests/output/bindgen_main.expected.rs").unwrap();

  assert_eq!(actual, expected);
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
  let expected = read_to_string("tests/output/bindgen_minimal.expected.rs").unwrap();

  assert_eq!(actual, expected);
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
  let expected = read_to_string("tests/output/bindgen_padding.expected.rs").unwrap();

  assert_eq!(actual, expected);
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
