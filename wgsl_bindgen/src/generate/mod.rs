use proc_macro2::TokenStream;
use quote::quote;

pub(crate) mod bind_group;
pub(crate) mod consts;
pub(crate) mod entry;
pub(crate) mod pipeline;
pub(crate) mod shader_module;
pub(crate) mod shader_registry;

pub(crate) fn quote_naga_capabilities(capabilities: naga::valid::Capabilities) -> TokenStream {
  match capabilities {
    caps if caps == naga::valid::Capabilities::empty() => quote!(wgpu::naga::valid::Capabilities::empty()),
    caps if caps == naga::valid::Capabilities::all() => quote!(wgpu::naga::valid::Capabilities::all()),
    caps if caps == naga::valid::Capabilities::PUSH_CONSTANT => quote!(wgpu::naga::valid::Capabilities::PUSH_CONSTANT),
    _ => {
      // For complex combinations or unknown capabilities, use from_bits_retain to preserve all information
      let bits = capabilities.bits();
      quote!(wgpu::naga::valid::Capabilities::from_bits_retain(#bits))
    }
  }
}

pub(crate) fn quote_shader_stages(shader_stages: wgpu::ShaderStages) -> TokenStream {
  match shader_stages {
    wgpu::ShaderStages::VERTEX_FRAGMENT => quote!(wgpu::ShaderStages::VERTEX_FRAGMENT),
    wgpu::ShaderStages::COMPUTE => quote!(wgpu::ShaderStages::COMPUTE),
    wgpu::ShaderStages::VERTEX => quote!(wgpu::ShaderStages::VERTEX),
    wgpu::ShaderStages::FRAGMENT => quote!(wgpu::ShaderStages::FRAGMENT),
    wgpu::ShaderStages::TASK => quote!(wgpu::ShaderStages::TASK),
    wgpu::ShaderStages::MESH => quote!(wgpu::ShaderStages::MESH),
    _ if shader_stages == wgpu::ShaderStages::all() => {
      quote!(wgpu::ShaderStages::all())
    }
    _ => {
      // For complex combinations, first try const-compatible union() method for common cases
      let mut stage_tokens = vec![];
      let mut reconstructed_bits = 0u32;
      
      if shader_stages.contains(wgpu::ShaderStages::VERTEX) {
        stage_tokens.push(quote!(wgpu::ShaderStages::VERTEX));
        reconstructed_bits |= wgpu::ShaderStages::VERTEX.bits();
      }
      if shader_stages.contains(wgpu::ShaderStages::FRAGMENT) {
        stage_tokens.push(quote!(wgpu::ShaderStages::FRAGMENT));
        reconstructed_bits |= wgpu::ShaderStages::FRAGMENT.bits();
      }
      if shader_stages.contains(wgpu::ShaderStages::COMPUTE) {
        stage_tokens.push(quote!(wgpu::ShaderStages::COMPUTE));
        reconstructed_bits |= wgpu::ShaderStages::COMPUTE.bits();
      }
      if shader_stages.contains(wgpu::ShaderStages::TASK) {
        stage_tokens.push(quote!(wgpu::ShaderStages::TASK));
        reconstructed_bits |= wgpu::ShaderStages::TASK.bits();
      }
      if shader_stages.contains(wgpu::ShaderStages::MESH) {
        stage_tokens.push(quote!(wgpu::ShaderStages::MESH));
        reconstructed_bits |= wgpu::ShaderStages::MESH.bits();
      }
      
      // Check if we captured all bits - if not, fall back to from_bits_retain to preserve all information
      if reconstructed_bits != shader_stages.bits() {
        // We missed some bits, use from_bits_retain to preserve all information
        let bits = shader_stages.bits();
        quote!(wgpu::ShaderStages::from_bits_retain(#bits))
      } else if stage_tokens.len() == 1 {
        stage_tokens[0].clone()
      } else {
        // Chain union calls: A.union(B).union(C)...
        let mut result = stage_tokens[0].clone();
        for token in &stage_tokens[1..] {
          result = quote!(#result.union(#token));
        }
        result
      }
    }
  }
}
