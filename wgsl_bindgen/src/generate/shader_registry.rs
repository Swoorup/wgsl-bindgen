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

    match source_type {
      WgslShaderSourceType::ComposerWithRelativePath => {
        // For ComposerWithRelativePath, we need to pass the entry_point enum to the module function
        let match_arms = self.entries.iter().map(|entry| {
          let mod_path = entry.get_mod_path();
          let enum_variant = entry.get_shader_variant();

          quote! {
            Self::#enum_variant => {
              #mod_path::#fn_name(device, base_dir, shader_defs, load_file)
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
          let mod_path = entry.get_mod_path();
          let enum_variant = entry.get_shader_variant();

          quote! {
            Self::#enum_variant => {
              #mod_path::#fn_name(composer, shader_defs)
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
      .options
      .shader_source_type
      .contains(WgslShaderSourceType::ComposerWithRelativePath)
    {
      return quote!();
    }

    let match_arms = self.entries.iter().map(|entry| {
      let mod_path = entry.get_mod_path();
      let enum_variant = entry.get_shader_variant();
      quote! {
        Self::#enum_variant => #mod_path::SHADER_ENTRY_PATH
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

  fn build_default_shader_defs_fn(&self) -> TokenStream {
    use WgslShaderSourceType::*;

    // Only generate if we're using shader_defs (non-embedded source types)
    if !self
      .options
      .shader_source_type
      .contains(EmbedWithNagaOilComposer)
      && !self
        .options
        .shader_source_type
        .contains(ComposerWithRelativePath)
    {
      return quote!();
    }

    if self.options.shader_defs.is_empty() {
      quote! {
        pub fn default_shader_defs() -> std::collections::HashMap<String, naga_oil::compose::ShaderDefValue> {
          std::collections::HashMap::new()
        }
      }
    } else {
      let entries: Vec<_> = self
        .options
        .shader_defs
        .iter()
        .map(|(key, value)| {
          let key_lit = proc_macro2::Literal::string(key);
          let value_expr = match value {
            naga_oil::compose::ShaderDefValue::Bool(b) => {
              quote!(naga_oil::compose::ShaderDefValue::Bool(#b))
            }
            naga_oil::compose::ShaderDefValue::Int(i) => {
              quote!(naga_oil::compose::ShaderDefValue::Int(#i))
            }
            naga_oil::compose::ShaderDefValue::UInt(u) => {
              quote!(naga_oil::compose::ShaderDefValue::UInt(#u))
            }
          };
          quote!((#key_lit.to_string(), #value_expr))
        })
        .collect();

      quote! {
        pub fn default_shader_defs() -> std::collections::HashMap<String, naga_oil::compose::ShaderDefValue> {
          std::collections::HashMap::from([
            #(#entries),*
          ])
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
    let load_shader_to_composer_module_fns = self
      .options
      .shader_source_type
      .iter()
      .map(|source_ty| self.build_load_shader_to_composer_module(source_ty))
      .collect::<Vec<_>>();

    let relative_path_fn = self.build_relative_path_fn();
    let default_shader_defs_fn = self.build_default_shader_defs_fn();

    // Add global methods if ComposerWithRelativePath is used
    let global_methods = if self
      .options
      .shader_source_type
      .contains(WgslShaderSourceType::ComposerWithRelativePath)
    {
      crate::generate::shader_module::generate_global_load_naga_module_from_path()
    } else {
      quote!()
    };

    quote! {
      impl ShaderEntry {
        #create_pipeline_layout_fn
        #(#create_shader_module_fns)*
        #(#load_shader_to_composer_module_fns)*
        #relative_path_fn
        #default_shader_defs_fn
        #global_methods
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
