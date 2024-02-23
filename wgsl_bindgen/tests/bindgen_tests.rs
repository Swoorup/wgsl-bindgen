use miette::{IntoDiagnostic, Result};
use pretty_assertions::assert_eq;
use wgsl_bindgen::{
  WgslBindgenOptionBuilder, WgslGlamTypeMap, WgslTypeSerializeStrategy,
};

#[test]
fn test_bevy_bindgen() -> Result<()> {
  let actual = WgslBindgenOptionBuilder::default()
    .module_import_root("bevy_pbr")
    .add_entry_point("tests/bevy_pbr_wgsl/pbr.wgsl")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .wgsl_type_map(WgslGlamTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .build()?
    .generate_string()
    .into_diagnostic()?;

  let expected = include_str!("./expected/bindgen_bevy.out.rs");

  assert_eq!(actual, expected);
  Ok(())
}

#[test]
fn test_main_bindgen() -> Result<()> {
  let actual = WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/test_shaders/main.wgsl")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .wgsl_type_map(WgslGlamTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .build()?
    .generate_string()
    .into_diagnostic()?;

  let expected = include_str!("./expected/bindgen_main.out.rs");

  assert_eq!(actual, expected);
  Ok(())
}

#[test]
#[ignore = "It doesn't like path symbols inside a nested type like array."]
fn test_path_import() -> Result<()> {
  let _ = WgslBindgenOptionBuilder::default()
    .add_entry_point("tests/test_shaders/path_import.wgsl")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .wgsl_type_map(WgslGlamTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .build()?
    .generate_string()
    .into_diagnostic()?;

  Ok(())
}
