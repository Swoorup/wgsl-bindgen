use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{
  WgslTypeSerializeStrategy, WgslBindgenOptionBuilder, WgslGlamTypeMap,
};

#[test]
fn test_bindgen() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .module_import_root("bevy_pbr")
    .add_entry_point("tests/bevy_pbr_wgsl/pbr.wgsl")
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .wgsl_type_map(WgslGlamTypeMap)
    .emit_rerun_if_change(false)
    .build()?
    .generate("tests/out")
    .into_diagnostic()
}
