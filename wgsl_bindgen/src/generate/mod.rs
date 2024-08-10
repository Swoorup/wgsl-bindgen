use proc_macro2::TokenStream;
use quote::quote;

pub(crate) mod bind_group;
pub(crate) mod consts;
pub(crate) mod entry;
pub(crate) mod pipeline;
pub(crate) mod shader_module;
pub(crate) mod shader_registry;

pub(crate) fn quote_shader_stages(shader_stages: wgpu::ShaderStages) -> TokenStream {
  match shader_stages {
    wgpu::ShaderStages::VERTEX_FRAGMENT => quote!(wgpu::ShaderStages::VERTEX_FRAGMENT),
    wgpu::ShaderStages::COMPUTE => quote!(wgpu::ShaderStages::COMPUTE),
    wgpu::ShaderStages::VERTEX => quote!(wgpu::ShaderStages::VERTEX),
    wgpu::ShaderStages::FRAGMENT => quote!(wgpu::ShaderStages::FRAGMENT),
    _ => {
      let mut stage_tokens = vec![];
      if shader_stages.contains(wgpu::ShaderStages::VERTEX) {
        stage_tokens.push(quote!(wgpu::ShaderStages::VERTEX));
      }
      if shader_stages.contains(wgpu::ShaderStages::FRAGMENT) {
        stage_tokens.push(quote!(wgpu::ShaderStages::FRAGMENT));
      }
      if shader_stages.contains(wgpu::ShaderStages::COMPUTE) {
        stage_tokens.push(quote!(wgpu::ShaderStages::COMPUTE));
      }
      // quote!(#(#stage_tokens)|*)
      unimplemented!()
    }
  }
}
