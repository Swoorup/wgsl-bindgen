use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{
  GlamWgslTypeMap, Regex, WgslBindgenOptionBuilder, WgslShaderIrCapabilities,
  WgslShaderSourceType, WgslTypeSerializeStrategy,
};

fn main() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("shaders")
    .add_entry_point("shaders/fullscreen_effects.wgsl")
    .add_entry_point("shaders/simple_array_demo.wgsl")
    .add_entry_point("shaders/overlay.wgsl")
    .add_entry_point("shaders/gradient_triangle.wgsl")
    .add_entry_point("shaders/compute_demo/particle_physics.wgsl")
    .add_entry_point("shaders/compute_demo/particle_renderer.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .ir_capabilities(WgslShaderIrCapabilities::PUSH_CONSTANT)
    .add_custom_padding_field_regexp(Regex::new("_pad.*").unwrap())
    .short_constructor(2)
    .shader_source_type(WgslShaderSourceType::ComposerWithRelativePath)
    .derive_serde(false)
    .output("src/shader_bindings.rs")
    .build()?
    .generate()
    .into_diagnostic()?;
  Ok(())
}
