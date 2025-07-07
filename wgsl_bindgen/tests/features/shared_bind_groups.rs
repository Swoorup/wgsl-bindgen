use std::fs::read_to_string;

use miette::{IntoDiagnostic, Result};
use syn::parse_str;
use wgsl_bindgen::{assert_tokens_snapshot, *};

#[test]
fn test_shared_bind_groups_minimal() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("tests/shaders/features/shared_bind_groups")
    .entry_points(vec![
      "tests/shaders/features/shared_bind_groups/shader_a.wgsl".to_string(),
      "tests/shaders/features/shared_bind_groups/shader_b.wgsl".to_string(),
    ])
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .short_constructor(2)
    .shader_source_type(WgslShaderSourceType::EmbedSource)
    .derive_serde(false)
    .emit_rerun_if_change(false)
    .skip_header_comments(true)
    .override_texture_filterability(vec![
      // Test making shared_texture non-filterable to verify regex filtering works
      OverrideTextureFilterability::from((".*shared_texture.*", false)),
    ])
    .override_sampler_type(vec![
      // Test making shared_sampler use NonFiltering type
      OverrideSamplerType::from((".*shared_sampler.*", SamplerType::NonFiltering)),
    ])
    .output("tests/output/features/shared_bind_groups_minimal.actual.rs")
    .build()?
    .generate()
    .into_diagnostic()?;

  let actual =
    read_to_string("tests/output/features/shared_bind_groups_minimal.actual.rs").unwrap();
  let parsed_output = parse_str(&actual).unwrap();
  assert_tokens_snapshot!(parsed_output);
  assert_rust_compilation!(parsed_output);
  Ok(())
}
