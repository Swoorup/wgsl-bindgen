use std::fs::read_to_string;

use miette::{IntoDiagnostic, Result};
use syn::parse_str;
use wgsl_bindgen::{
  assert_rust_compilation, assert_tokens_snapshot, GlamWgslTypeMap,
  WgslBindgenOptionBuilder, WgslTypeSerializeStrategy,
};

#[test]
fn test_bevy_pbr_integration() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .module_import_root("bevy_pbr")
    .workspace_root("tests/shaders/integration/bevy_pbr_wgsl")
    .add_entry_point("tests/shaders/integration/bevy_pbr_wgsl/pbr.wgsl")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/integration/bevy_pbr.actual.rs".to_string())
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/integration/bevy_pbr.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}
