use std::fs::read_to_string;

use miette::{IntoDiagnostic, Result};
use syn::parse_str;
use wgsl_bindgen::{assert_tokens_snapshot, *};

/// Test that EmbedWithWesl compiles a simple WESL shader (without imports) into
/// a valid shader module binding.
#[cfg(feature = "wesl")]
#[test]
fn test_wesl_embed_basic() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/features/wesl")
    .entry_points(vec![
      "tests/shaders/features/wesl/test_shader.wgsl".to_string()
    ])
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .shader_source_type(WgslShaderSourceType::EmbedWithWesl)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/features/wesl_basic.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual = read_to_string("tests/output/features/wesl_basic.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  Ok(())
}

/// Test that EmbedWithWesl correctly enables conditional translation features
/// via shader_defs (Bool values only; non-Bool defs are ignored for WESL).
#[cfg(feature = "wesl")]
#[test]
fn test_wesl_embed_with_features() -> Result<()> {
  let shader_defs = vec![("USE_TEXTURE".to_string(), ShaderDefValue::Bool(true))];

  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/features/wesl")
    .entry_points(vec![
      "tests/shaders/features/wesl/test_shader.wgsl".to_string()
    ])
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .shader_source_type(WgslShaderSourceType::EmbedWithWesl)
    .add_shader_defs(shader_defs)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .output("tests/output/features/wesl_with_features.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/features/wesl_with_features.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  Ok(())
}
