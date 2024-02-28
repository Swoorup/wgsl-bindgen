use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{
    GlamWgslTypeMap, WgslBindgenOptionBuilder, WgslShaderSourceType, WgslTypeSerializeStrategy,
};

fn main() -> Result<()> {
    WgslBindgenOptionBuilder::default()
        .add_entry_point("src/shader/testbed.wgsl")
        .add_entry_point("src/shader/triangle.wgsl")
        .skip_hash_check(true)
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .type_map(GlamWgslTypeMap)
        .derive_serde(false)
        .output_file("src/shader.rs")
        .short_constructor(2)
        .shader_source_type(WgslShaderSourceType::UseComposerWithIncludeStr)
        .build()?
        .generate()
        .into_diagnostic()
}
