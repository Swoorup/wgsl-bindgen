use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{
  GlamWgslTypeMap, Regex, RustWgslTypeMap, WgslBindgenOptionBuilder,
  WgslShaderIrCapabilities, WgslShaderSourceType, WgslTypeSerializeStrategy,
};

fn main() -> Result<()> {
  generate_demo_bindings()?;
  generate_buffer_layout_bindings()
}

fn generate_demo_bindings() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("shaders")
    .add_entry_point("shaders/fullscreen_effects.wgsl")
    .add_entry_point("shaders/simple_array_demo.wgsl")
    .add_entry_point("shaders/overlay.wgsl")
    .add_entry_point("shaders/gradient_triangle.wgsl")
    .add_entry_point("shaders/multisampled_texture_demo.wgsl")
    .add_entry_point("shaders/compute_demo/particle_physics.wgsl")
    .add_entry_point("shaders/compute_demo/particle_renderer.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(GlamWgslTypeMap)
    .ir_capabilities(
      WgslShaderIrCapabilities::IMMEDIATES
        | WgslShaderIrCapabilities::TEXTURE_AND_SAMPLER_BINDING_ARRAY,
    )
    .add_custom_padding_field_regexp(Regex::new("_pad.*").unwrap())
    .short_constructor(2)
    .shader_source_type(
      WgslShaderSourceType::EmbedSource | WgslShaderSourceType::ComposerWithRelativePath,
    )
    .derive_serde(false)
    .output("src/shader_bindings.rs")
    .build()?
    .generate()
    .into_diagnostic()?;
  Ok(())
}

fn generate_buffer_layout_bindings() -> Result<()> {
  WgslBindgenOptionBuilder::default()
    .workspace_root("shaders")
    .add_entry_point("shaders/buffer_layouts.wgsl")
    .skip_hash_check(true)
    .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
    .type_map(RustWgslTypeMap)
    .shader_source_type(WgslShaderSourceType::EmbedSource)
    .output("src/buffer_layout_bindings.rs")
    .build()?
    .generate()
    .into_diagnostic()?;
  Ok(())
}
