use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{WgslTypeSerializeStrategy, WgslBindgenOptionBuilder, WgslGlamTypeMap};

fn main() -> Result<()> {
    WgslBindgenOptionBuilder::default()
        .add_entry_point("src/shader/testbed.wgsl")
        .add_entry_point("src/shader/triangle.wgsl")
        .skip_hash_check(true)
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .wgsl_type_map(WgslGlamTypeMap)
        .derive_serde(false)
        .build()?
        .generate("src/shader.rs")
        .into_diagnostic()
}
