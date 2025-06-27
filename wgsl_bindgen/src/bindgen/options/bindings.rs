use quote::format_ident;
use syn::Ident;

use crate::qs::{quote, Index, TokenStream};
use crate::FastIndexMap;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum BindResourceType {
  Buffer,
  Sampler,
  Texture,
  AccelerationStructure,
  BufferArray,
  SamplerArray,
  TextureArray,
}

#[derive(Clone)]
pub struct BindingGenerator {
  pub bind_group_layout: BindGroupLayoutGenerator,
  pub pipeline_layout: PipelineLayoutGenerator,
}

impl std::fmt::Debug for BindingGenerator {
  fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // skip the debug generation for this,
    // as the output changes on every build due to fns
    Ok(())
  }
}

/// Represents a generator for creating WGSL bind group layout structures.
///
/// This struct is used to generate the code for creating a bind group layout in WGSL.
/// It contains the necessary information for generating the code, such as the prefix name for the layout,
/// whether the generated code uses lifetimes, the type of the entry struct, a function for constructing entries,
/// and a map of binding resource types to their corresponding token streams.
#[derive(Clone, Debug)]
pub struct BindGroupLayoutGenerator {
  /// The prefix for the bind group layout.
  pub name_prefix: String,

  /// Indicates whether the generated code uses lifetimes.
  ///
  /// If this is `true`, the generated code will include lifetimes. If it's `false`, the generated code will not include lifetimes.
  pub uses_lifetime: bool,

  /// The type of the entry struct in the generated code.
  ///
  /// This is represented as a `TokenStream` that contains the code for the type of the entry struct.
  pub entry_struct_type: TokenStream,

  /// A function for constructing entries in the generated code.
  ///
  /// This function takes a binding index, a `TokenStream` for the binding variable, and a `WgslBindResourceType` for the resource type,
  /// and returns a `TokenStream` that contains the code for constructing an entry.
  pub entry_constructor: fn(usize, TokenStream, BindResourceType) -> TokenStream,

  /// A map of binding resource types to their corresponding token streams.
  ///
  /// This map is used to generate the code for the binding resources in the bind group layout.
  pub binding_type_map: FastIndexMap<BindResourceType, TokenStream>,
}

impl BindGroupLayoutGenerator {
  pub(crate) fn bind_group_name_ident(&self, group_index: u32) -> Ident {
    format_ident!("{}{}", self.name_prefix, group_index)
  }

  pub(crate) fn bind_group_entries_struct_name_ident(&self, group_index: u32) -> Ident {
    format_ident!("{}{}{}", self.name_prefix, group_index, "Entries")
  }
}

#[derive(Clone, Debug)]
pub struct PipelineLayoutGenerator {
  pub layout_name: String,
  pub bind_group_layout_type: TokenStream,
}

pub trait GetBindingsGeneratorConfig {
  fn get_generator_config(self) -> BindingGenerator;
}
impl GetBindingsGeneratorConfig for BindingGenerator {
  fn get_generator_config(self) -> BindingGenerator {
    self
  }
}

impl Default for BindingGenerator {
  fn default() -> BindingGenerator {
    WgpuGetBindingsGeneratorConfig.get_generator_config()
  }
}

pub struct WgpuGetBindingsGeneratorConfig;
impl WgpuGetBindingsGeneratorConfig {
  fn get_bind_group_layout_generator_config() -> BindGroupLayoutGenerator {
    let binding_type_map = vec![
      (BindResourceType::Buffer, quote! { wgpu::BufferBinding<'a> }),
      (BindResourceType::Sampler, quote! { &'a wgpu::Sampler }),
      (BindResourceType::Texture, quote! { &'a wgpu::TextureView }),
      (BindResourceType::AccelerationStructure, quote! { &'a wgpu::Tlas }),
      (BindResourceType::BufferArray, quote! { &'a [wgpu::BufferBinding<'a>] }),
      (BindResourceType::SamplerArray, quote! { &'a [&'a wgpu::Sampler] }),
      (BindResourceType::TextureArray, quote! { &'a [&'a wgpu::TextureView] }),
    ]
    .into_iter()
    .collect::<FastIndexMap<_, _>>();

    fn entry_constructor(
      binding: usize,
      binding_var: TokenStream,
      resource_type: BindResourceType,
    ) -> TokenStream {
      let resource = match resource_type {
        BindResourceType::Buffer => {
          quote!(wgpu::BindingResource::Buffer(#binding_var))
        }
        BindResourceType::Sampler => {
          quote!(wgpu::BindingResource::Sampler(#binding_var))
        }
        BindResourceType::Texture => {
          quote!(wgpu::BindingResource::TextureView(#binding_var))
        }
        BindResourceType::AccelerationStructure => {
          quote!(wgpu::BindingResource::AccelerationStructure(#binding_var))
        }
        BindResourceType::BufferArray => {
          quote!(wgpu::BindingResource::BufferArray(#binding_var))
        }
        BindResourceType::SamplerArray => {
          quote!(wgpu::BindingResource::SamplerArray(#binding_var))
        }
        BindResourceType::TextureArray => {
          quote!(wgpu::BindingResource::TextureViewArray(#binding_var))
        }
      };

      let binding = Index::from(binding);
      quote! {
        wgpu::BindGroupEntry {
          binding: #binding,
          resource: #resource,
        }
      }
    }

    BindGroupLayoutGenerator {
      name_prefix: "WgpuBindGroup".into(),
      uses_lifetime: true,
      entry_struct_type: quote!(wgpu::BindGroupEntry<'a>),
      entry_constructor,
      binding_type_map,
    }
  }

  fn get_pipeline_layout_generator() -> PipelineLayoutGenerator {
    PipelineLayoutGenerator {
      layout_name: "WgpuPipelineLayout".into(),
      bind_group_layout_type: quote!(wgpu::BindGroupLayout),
    }
  }
}

impl GetBindingsGeneratorConfig for WgpuGetBindingsGeneratorConfig {
  fn get_generator_config(self) -> BindingGenerator {
    let bind_group_layout = Self::get_bind_group_layout_generator_config();
    let pipeline_layout = Self::get_pipeline_layout_generator();
    BindingGenerator {
      bind_group_layout,
      pipeline_layout,
    }
  }
}
