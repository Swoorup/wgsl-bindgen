use miette::{IntoDiagnostic, Result};
use pretty_assertions::assert_eq;
use wgsl_bindgen::{
  WgslBindgenOptionBuilder, GlamWgslTypeMap, WgslTypeSerializeStrategy,
};

#[test]
fn test_bevy_bindgen() -> Result<()> {
  let actual = WgslBindgenOptionBuilder::default()
    .module_import_root("bevy_pbr")
    .add_entry_point("tests/shaders/bevy_pbr_wgsl/pbr.wgsl")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .wgsl_type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .build()?
    .generate_string()
    .into_diagnostic()?;

  let expected = include_str!("expected/bindgen_bevy.out.rs");

  assert_eq!(actual, expected);
  Ok(())
}

#[test]
fn test_main_bindgen() -> Result<()> {
  let actual = WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/basic/main.wgsl")
    .additional_scan_dir((None, "tests/shaders/additional"))
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .wgsl_type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .build()?
    .generate_string()
    .into_diagnostic()?;

  let expected = include_str!("expected/bindgen_main.out.rs");

  assert_eq!(actual, expected);
  Ok(())
}

#[test]
#[ignore = "It doesn't like path symbols inside a nested type like array."]
fn test_path_import() -> Result<()> {
  let _ = WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/shaders/basic/path_import.wgsl")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .wgsl_type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .build()?
    .generate_string()
    .into_diagnostic()?;

  Ok(())
}
