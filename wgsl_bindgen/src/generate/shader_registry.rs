//! This module provides functionality for building a shader registry.
//!
//! This will create a `ShaderEntry` enum with a variant for each entry in `entries`,
//! and functions for creating the pipeline layout and shader module for each variant.

use derive_more::Constructor;
use enumflags2::BitFlags;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
  sanitize_and_pascal_case, WgslBindgenOption, WgslEntryResult, WgslShaderSourceType,
};

#[derive(Constructor)]
struct ShaderEntryBuilder<'a, 'b> {
  entries: &'a [WgslEntryResult<'b>],
  options: &'a WgslBindgenOption,
}

impl<'a, 'b> ShaderEntryBuilder<'a, 'b> {
  fn build_registry_enum(&self) -> TokenStream {
    let variants = self.entries.iter().map(|entry| entry.get_shader_variant());

    quote! {
      #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
      pub enum ShaderEntry {
        #( #variants, )*
      }
    }
  }

  fn build_create_pipeline_layout_fn(&self) -> TokenStream {
    let match_arms = self.entries.iter().map(|entry| {
      // Convert module path like "lines::segment" to a proper Rust path
      let mod_path = entry.get_mod_path();
      let enum_variant = entry.get_shader_variant();

      quote! {
        Self::#enum_variant => #mod_path::create_pipeline_layout(device)
      }
    });

    quote! {
      pub fn create_pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
        match self {
          #( #match_arms, )*
        }
      }
    }
  }

  fn build_create_shader_module(&self, source_type: WgslShaderSourceType) -> TokenStream {
    let fn_name = format_ident!("{}", source_type.create_shader_module_fn_name());
    let (param_defs, params) = source_type.shader_module_params_defs_and_params();
    let return_type = source_type.get_return_type(quote!(wgpu::ShaderModule));

    let match_arms = self.entries.iter().map(|entry| {
      let mod_path = entry.get_mod_path();
      let enum_variant = entry.get_shader_variant();

      quote! {
        Self::#enum_variant => {
          #mod_path::#fn_name(#params)
        }
      }
    });

    quote! {
      pub fn #fn_name(&self, #param_defs) -> #return_type {
        match self {
          #( #match_arms, )*
        }
      }
    }
  }

  fn build_enum_impl(&self) -> TokenStream {
    let create_shader_module_fns = self
      .options
      .shader_source_type
      .iter()
      .map(|source_ty| self.build_create_shader_module(source_ty))
      .collect::<Vec<_>>();

    let create_pipeline_layout_fn = self.build_create_pipeline_layout_fn();

    quote! {
      impl ShaderEntry {
        #create_pipeline_layout_fn
        #(#create_shader_module_fns)*
      }
    }
  }

  pub fn build(&self) -> TokenStream {
    let enum_def = self.build_registry_enum();
    let enum_impl = self.build_enum_impl();
    quote! {
      #enum_def
      #enum_impl
    }
  }
}

pub(crate) fn build_shader_registry(
  entries: &[WgslEntryResult<'_>],
  options: &WgslBindgenOption,
) -> TokenStream {
  ShaderEntryBuilder::new(entries, options).build()
}
