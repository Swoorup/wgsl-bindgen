use std::fs::read_to_string;

use miette::{IntoDiagnostic, Result};
use pretty_assertions::assert_eq;
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
  let expected = read_to_string("tests/output/issue_35.expected.rs").unwrap();

  assert_eq!(expected, actual);
  Ok(())
}
