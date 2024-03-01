//! This module provides functionality for building a shader registry.
//!
//! This will create a `ShaderRegistry` enum with a variant for each entry in `entries`,
//! and functions for creating the pipeline layout and shader module for each variant.
use derive_more::Constructor;
use enumflags2::BitFlags;
use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{WgslEntryResult, WgslShaderSourceType};

#[derive(Constructor)]
struct ShaderRegistryBuilder<'a, 'b> {
  entries: &'a [WgslEntryResult<'b>],
  source_type: BitFlags<WgslShaderSourceType>,
}

impl<'a, 'b> ShaderRegistryBuilder<'a, 'b> {
  fn build_registry_enum(&self) -> TokenStream {
    let variants = self.entries.iter().map(|entry| {
      let name = format_ident!("{}", create_enum_variant_name(&entry.mod_name));
      quote! { #name }
    });

    quote! {
      #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
      pub enum ShaderRegistry {
        #( #variants, )*
      }
    }
  }

  fn build_create_pipeline_layout_fn(&self) -> TokenStream {
    let match_arms = self.entries.iter().map(|entry| {
      let mod_path = format_ident!("{}", entry.mod_name);
      let enum_variant = format_ident!("{}", create_enum_variant_name(&entry.mod_name));

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

    let match_arms = self.entries.iter().map(|entry| {
      let mod_path = format_ident!("{}", entry.mod_name);
      let enum_variant = format_ident!("{}", create_enum_variant_name(&entry.mod_name));

      quote! {
        Self::#enum_variant => {
          #mod_path::#fn_name(#params)
        }
      }
    });

    quote! {
      pub fn #fn_name(&self, #param_defs) -> wgpu::ShaderModule {
        match self {
          #( #match_arms, )*
        }
      }
    }
  }

  fn build_shader_files_fn(&self) -> TokenStream {
    if !self
      .source_type
      .contains(WgslShaderSourceType::UseComposerWithPath)
    {
      return quote!();
    }

    let match_arms = self.entries.iter().map(|entry| {
      let mod_path = format_ident!("{}", entry.mod_name);
      let enum_variant = format_ident!("{}", create_enum_variant_name(&entry.mod_name));

      quote! {
        Self::#enum_variant => #mod_path::SHADER_FILES
      }
    });

    quote! {
      pub fn shader_files(&self) -> &[&str] {
        match self {
          #( #match_arms, )*
        }
      }
    }
  }

  fn build_enum_impl(&self) -> TokenStream {
    let create_shader_module_fns = self
      .source_type
      .iter()
      .map(|source_ty| self.build_create_shader_module(source_ty))
      .collect::<Vec<_>>();

    let create_pipeline_layout_fn = self.build_create_pipeline_layout_fn();

    let create_shader_files_fn = self.build_shader_files_fn();

    quote! {
      impl ShaderRegistry {
        #create_pipeline_layout_fn
        #(#create_shader_module_fns)*
        #create_shader_files_fn
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
  source_type: BitFlags<WgslShaderSourceType>,
) -> TokenStream {
  ShaderRegistryBuilder::new(entries, source_type).build()
}

fn create_enum_variant_name(v: &str) -> String {
  // Remove unnecessary characters
  let cleaned: String = v
    .chars()
    .filter(|ch| ch.is_alphanumeric() || *ch == '_')
    .collect();
  // Convert to PascalCase
  cleaned.to_pascal_case()
}
