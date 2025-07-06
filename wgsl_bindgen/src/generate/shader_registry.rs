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
      // Convert module path like "lines::segment" to a proper Rust path
      let mod_path_parts: Vec<_> = entry
        .mod_name
        .split("::")
        .map(|part| format_ident!("{}", part))
        .collect();
      let enum_variant = format_ident!("{}", sanitize_and_pascal_case(&entry.mod_name));

      quote! {
        Self::#enum_variant => #(#mod_path_parts)::*::create_pipeline_layout(device)
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

    match source_type {
      WgslShaderSourceType::ComposerWithRelativePath => {
        // For ComposerWithRelativePath, we need to pass the entry_point enum to the module function
        let match_arms = self.entries.iter().map(|entry| {
          let mod_path_parts: Vec<_> = entry.mod_name.split("::").map(|part| format_ident!("{}", part)).collect();
          let enum_variant =
            format_ident!("{}", sanitize_and_pascal_case(&entry.mod_name));

          quote! {
            Self::#enum_variant => {
              #(#mod_path_parts)::*::#fn_name(device, base_dir, *self, shader_defs, load_file)
            }
          }
        });

        quote! {
          pub fn #fn_name(&self, #param_defs) -> #return_type
          {
            match self {
              #( #match_arms, )*
            }
          }
        }
      }
      _ => {
        let match_arms = self.entries.iter().map(|entry| {
          let mod_path_parts: Vec<_> = entry
            .mod_name
            .split("::")
            .map(|part| format_ident!("{}", part))
            .collect();
          let enum_variant =
            format_ident!("{}", sanitize_and_pascal_case(&entry.mod_name));

          quote! {
            Self::#enum_variant => {
              #(#mod_path_parts)::*::#fn_name(#params)
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
    }
  }

  fn build_load_shader_to_composer_module(
    &self,
    source_type: WgslShaderSourceType,
  ) -> TokenStream {
    match source_type {
      WgslShaderSourceType::EmbedSource => {
        quote!()
      }
      WgslShaderSourceType::ComposerWithRelativePath => {
        // For ComposerWithRelativePath, we reference the global function directly
        quote!()
      }
      _ => {
        let fn_name = format_ident!("{}", source_type.load_shader_module_fn_name());

        let match_arms = self.entries.iter().map(|entry| {
          let mod_path_parts: Vec<_> = entry
            .mod_name
            .split("::")
            .map(|part| format_ident!("{}", part))
            .collect();
          let enum_variant =
            format_ident!("{}", sanitize_and_pascal_case(&entry.mod_name));

          quote! {
            Self::#enum_variant => {
              #(#mod_path_parts)::*::#fn_name(composer, shader_defs)
            }
          }
        });

        let return_type = source_type.get_return_type(quote!(wgpu::naga::Module));

        quote! {
          pub fn #fn_name(&self,
            composer: &mut naga_oil::compose::Composer,
            shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>
          ) -> #return_type {
            match self {
              #( #match_arms, )*
            }
          }
        }
      }
    }
  }

  fn build_relative_path_fn(&self) -> TokenStream {
    if !self
      .source_type
      .contains(WgslShaderSourceType::ComposerWithRelativePath)
    {
      return quote!();
    }

    let match_arms = self.entries.iter().map(|entry| {
      let mod_path_parts: Vec<_> = entry
        .mod_name
        .split("::")
        .map(|part| format_ident!("{}", part))
        .collect();
      let enum_variant = format_ident!("{}", sanitize_and_pascal_case(&entry.mod_name));

      quote! {
        Self::#enum_variant => #(#mod_path_parts)::*::SHADER_ENTRY_PATH
      }
    });

    quote! {
      pub fn relative_path(&self) -> &'static str {
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
    let load_shader_to_composer_module_fns = self
      .source_type
      .iter()
      .map(|source_ty| self.build_load_shader_to_composer_module(source_ty))
      .collect::<Vec<_>>();

    let relative_path_fn = self.build_relative_path_fn();

    quote! {
      impl ShaderEntry {
        #create_pipeline_layout_fn
        #(#create_shader_module_fns)*
        #(#load_shader_to_composer_module_fns)*
        #relative_path_fn
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
