use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{
    qs::quote, GlamWgslTypeMap, Regex, WgslBindgenOptionBuilder, WgslShaderSourceType,
    WgslTypeSerializeStrategy,
};

fn main() -> Result<()> {
    WgslBindgenOptionBuilder::default()
        .workspace_root("assets/shader")
        .add_entry_point("assets/shader/utils/testbed.wgsl")
        .add_entry_point("assets/shader/triangle.wgsl")
        .skip_hash_check(true)
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .type_map(GlamWgslTypeMap)
        .add_custom_struct_field_mapping("types::VectorsU32", [("a", quote!(crate::MyTwoU32))])
        .add_custom_struct_mapping(("types::Scalars", quote!(crate::MyScalars)))
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
