use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{WgslBindgenOptionBuilder, GlamWgslTypeMap, WgslShaderSourceOutputType, WgslTypeSerializeStrategy};

fn main() -> Result<()> {
    WgslBindgenOptionBuilder::default()
        .add_entry_point("src/shader/testbed.wgsl")
        .add_entry_point("src/shader/triangle.wgsl")
        .skip_hash_check(true)
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .wgsl_type_map(GlamWgslTypeMap)
        .derive_serde(false)
        .output_file("src/shader.rs")
        .shader_source_output_type(WgslShaderSourceOutputType::Composer)
        .build()?
        .generate()
        .into_diagnostic()
}
