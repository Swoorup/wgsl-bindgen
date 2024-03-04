use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{
    GlamWgslTypeMap, WgslBindgenOptionBuilder, WgslShaderSourceType, WgslTypeSerializeStrategy,
};

fn main() -> Result<()> {
    WgslBindgenOptionBuilder::default()
        .workspace_root("assets/shader")
        .add_entry_point("assets/shader/utils/testbed.wgsl")
        .add_entry_point("assets/shader/triangle.wgsl")
        .skip_hash_check(true)
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .type_map(GlamWgslTypeMap)
        .derive_serde(false)
        .output("src/shader.rs")
        .short_constructor(2)
        .shader_source_type(
            WgslShaderSourceType::UseComposerWithPath
                | WgslShaderSourceType::UseComposerEmbed
                | WgslShaderSourceType::UseEmbed,
        )
        .build()?
        .generate()
        .into_diagnostic()
}
