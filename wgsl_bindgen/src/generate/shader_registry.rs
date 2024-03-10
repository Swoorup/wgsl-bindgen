//! This module provides functionality for building a shader registry.
//!
//! This will create a `ShaderEntry` enum with a variant for each entry in `entries`,
//! and functions for creating the pipeline layout and shader module for each variant.
use derive_more::Constructor;
use enumflags2::BitFlags;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{sanitize_and_pascal_case, WgslEntryResult, WgslShaderSourceType};

#[derive(Constructor)]
struct ShaderEntryBuilder<'a, 'b> {
  entries: &'a [WgslEntryResult<'b>],
  source_type: BitFlags<WgslShaderSourceType>,
}

impl<'a, 'b> ShaderEntryBuilder<'a, 'b> {
  fn build_registry_enum(&self) -> TokenStream {
    let variants = self
      .entries
      .iter()
      .map(|entry| format_ident!("{}", sanitize_and_pascal_case(&entry.mod_name)));

    quote! {
      #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
      pub enum ShaderEntry {
        #( #variants, )*
      }
    }
  }

  fn build_create_pipeline_layout_fn(&self) -> TokenStream {
    let match_arms = self.entries.iter().map(|entry| {
      let mod_path = format_ident!("{}", entry.mod_name);
      let enum_variant = format_ident!("{}", sanitize_and_pascal_case(&entry.mod_name));

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
      let enum_variant = format_ident!("{}", sanitize_and_pascal_case(&entry.mod_name));

      quote! {
        Self::#enum_variant => {
          #mod_path::#fn_name(#params)
        }
      }
    });

    let return_type = source_type.get_return_type(quote!(wgpu::ShaderModule));

    quote! {
      pub fn #fn_name(&self, #param_defs) -> #return_type {
        match self {
          #( #match_arms, )*
        }
      }
    }
  }

  fn build_shader_entry_filename_fn(&self) -> TokenStream {
    if !self
      .source_type
      .contains(WgslShaderSourceType::UseComposerWithPath)
    {
      return quote!();
    }

    let match_arms = self.entries.iter().map(|entry| {
      let filename = entry
        .source_including_deps
        .source_file
        .file_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
      let enum_variant = format_ident!("{}", sanitize_and_pascal_case(&entry.mod_name));

      quote! {
        Self::#enum_variant => #filename
      }
    });

    quote! {
      pub fn shader_entry_filename(&self) -> &'static str {
        match self {
          #( #match_arms, )*
        }
      }
    }
  }

  fn build_shader_paths_fn(&self) -> TokenStream {
    if !self
      .source_type
      .contains(WgslShaderSourceType::UseComposerWithPath)
    {
      return quote!();
    }

    let match_arms = self.entries.iter().map(|entry| {
      let mod_path = format_ident!("{}", entry.mod_name);
      let enum_variant = format_ident!("{}", sanitize_and_pascal_case(&entry.mod_name));

      quote! {
        Self::#enum_variant => #mod_path::SHADER_PATHS
      }
    });

    quote! {
      pub fn shader_paths(&self) -> &[&str] {
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

    let shader_paths_fn = self.build_shader_paths_fn();
    let shader_entry_filename_fn = self.build_shader_entry_filename_fn();

    quote! {
      impl ShaderEntry {
        #create_pipeline_layout_fn
        #(#create_shader_module_fns)*
        #shader_entry_filename_fn
        #shader_paths_fn
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
  ShaderEntryBuilder::new(entries, source_type).build()
}
