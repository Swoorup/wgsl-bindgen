use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::qs::quote;
use wgsl_bindgen::{
  GlamWgslTypeMap, Regex, WgslBindgenOptionBuilder, WgslShaderIrCapabilities,
  WgslShaderSourceType, WgslTypeSerializeStrategy,
};

fn main() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("shaders")
    .add_entry_point("shaders/testbed.wgsl")
    .add_entry_point("shaders/triangle.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .ir_capabilities(WgslShaderIrCapabilities::PUSH_CONSTANT)
    .override_struct_field_type(
      [("utils::types::VectorsU32", "a", quote!(crate::MyTwoU32))].map(Into::into),
    )
    .add_override_struct_mapping(("utils::types::Scalars", quote!(crate::MyScalars)))
    .add_custom_padding_field_regexp(Regex::new("_pad.*").unwrap())
    .short_constructor(2)
    .shader_source_type(
      WgslShaderSourceType::UseComposerWithPath
        | WgslShaderSourceType::UseComposerEmbed
        | WgslShaderSourceType::UseEmbed,
    )
    .derive_serde(false)
    .output("src/shader_bindings.rs")
    .build()?
    .generate()
    .into_diagnostic()
}
