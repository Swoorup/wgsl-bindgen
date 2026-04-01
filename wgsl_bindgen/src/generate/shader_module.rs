//! This file is used for creating direct shader file related functions:
//! such as `create_shader_module`, `create_compute_module`

use derive_more::Constructor;
use enumflags2::BitFlags;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, TokenStreamExt};
use syn::{Ident, Index};

use crate::generate::quote_naga_capabilities;
use crate::quote_gen::create_shader_raw_string_literal;
use crate::{
  sanitize_and_pascal_case, WgslBindgenOption, WgslEntryResult, WgslShaderSourceType,
};

impl<'a> WgslEntryResult<'a> {
  fn get_label(&self) -> TokenStream {
    let get_label = || {
      self
        .source_including_deps
        .source_file
        .file_path
        .file_name()?
        .to_str()
    };

    match get_label() {
      Some(label) => quote!(Some(#label)),
      None => quote!(None),
    }
  }
}

impl WgslShaderSourceType {
  pub(crate) fn create_shader_module_fn_name(&self) -> &'static str {
    match self {
      WgslShaderSourceType::EmbedSource => "create_shader_module_embed_source",
      WgslShaderSourceType::WeslWithRelativePath => "create_shader_module_from_path",
    }
  }

  pub(crate) fn load_shader_module_fn_name(&self) -> Ident {
    match self {
      WgslShaderSourceType::EmbedSource => format_ident!("load_shader_module_embedded"),
      WgslShaderSourceType::WeslWithRelativePath => {
        format_ident!("load_shader_module_from_path")
      }
    }
  }

  pub(crate) fn create_compute_pipeline_fn_name(&self, name: &str) -> Ident {
    match self {
      WgslShaderSourceType::EmbedSource => {
        format_ident!("create_{}_pipeline_embed_source", name)
      }
      WgslShaderSourceType::WeslWithRelativePath => {
        format_ident!("create_{}_pipeline_from_path", name)
      }
    }
  }

  pub(crate) fn get_return_type(&self, type_to_return: TokenStream) -> TokenStream {
    match self {
      WgslShaderSourceType::EmbedSource => type_to_return,
      WgslShaderSourceType::WeslWithRelativePath => {
        quote!(::std::result::Result<#type_to_return, ::std::string::String>)
      }
    }
  }

  pub(crate) fn wrap_return_stmt(&self, stm: TokenStream) -> TokenStream {
    match self {
      WgslShaderSourceType::EmbedSource => stm,
      WgslShaderSourceType::WeslWithRelativePath => quote!(::std::result::Result::Ok(#stm)),
    }
  }

  pub(crate) fn get_propagate_operator(&self) -> TokenStream {
    match self {
      WgslShaderSourceType::EmbedSource => quote!(),
      WgslShaderSourceType::WeslWithRelativePath => quote!(?),
    }
  }

  pub(crate) fn shader_module_params_defs_and_params(
    &self,
  ) -> (TokenStream, TokenStream) {
    match self {
      WgslShaderSourceType::EmbedSource => {
        let param_defs = quote!(device: &wgpu::Device);
        let params = quote!(device);
        (param_defs, params)
      }
      WgslShaderSourceType::WeslWithRelativePath => {
        let param_defs = quote!(device: &wgpu::Device, base_dir: &::std::path::Path);
        let params = quote!(device, base_dir);
        (param_defs, params)
      }
    }
  }
}

#[derive(Constructor)]
struct ComputeModuleBuilder<'a> {
  module: &'a naga::Module,
  source_type_flags: BitFlags<WgslShaderSourceType>,
}

impl<'a> ComputeModuleBuilder<'a> {
  fn build_compute_pipeline_fn(
    e: &naga::EntryPoint,
    source_type: WgslShaderSourceType,
  ) -> TokenStream {
    // Compute pipeline creation has few parameters and can be generated.

    let pipeline_name = source_type.create_compute_pipeline_fn_name(&e.name);

    let entry_point = &e.name;
    // TODO: Include a user supplied module name in the label?
    let label = format!("Compute Pipeline {}", e.name);

    let create_shader_module_fn_name =
      format_ident!("{}", source_type.create_shader_module_fn_name());

    let (param_defs, params) = source_type.shader_module_params_defs_and_params();

    let return_type = source_type.get_return_type(quote!(wgpu::ComputePipeline));
    let propagate_operator = source_type.get_propagate_operator();

    let module_creation = quote! {
      let module = super::#create_shader_module_fn_name(#params)#propagate_operator;
    };

    let return_value = source_type.wrap_return_stmt(quote! {
      device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
          label: Some(#label),
          layout: Some(&layout),
          module: &module,
          entry_point: Some(#entry_point),
          compilation_options: Default::default(),
          cache: None,
      })
    });

    quote! {
        pub fn #pipeline_name(#param_defs) -> #return_type {
            #module_creation
            let layout = super::create_pipeline_layout(device);
            #return_value
        }
    }
  }

  fn workgroup_size(e: &naga::EntryPoint) -> TokenStream {
    // Use Index to avoid specifying the type on literals.
    let name = format_ident!("{}_WORKGROUP_SIZE", e.name.to_uppercase());
    let [x, y, z] = e.workgroup_size.map(|s| Index::from(s as usize));
    quote!(pub const #name: [u32; 3] = [#x, #y, #z];)
  }

  pub(crate) fn entry_points_iter(&self) -> impl Iterator<Item = &naga::EntryPoint> {
    self
      .module
      .entry_points
      .iter()
      .filter(|e| e.stage == naga::ShaderStage::Compute)
  }

  fn build(&self) -> TokenStream {
    let entry_points: Vec<_> = self
      .entry_points_iter()
      .map(|e| {
        let workgroup_size_constant = Self::workgroup_size(e);

        let create_pipeline_fns = self
          .source_type_flags
          .iter()
          .map(|source_type| Self::build_compute_pipeline_fn(e, source_type))
          .collect::<Vec<_>>();

        quote! {
            #workgroup_size_constant
            #(#create_pipeline_fns)*
        }
      })
      .collect();

    if entry_points.is_empty() {
      // Don't include empty modules.
      quote!()
    } else {
      quote! {
          pub mod compute {
              use super::{_root, _root::*};
              #(#entry_points)*
          }
      }
    }
  }
}
pub(crate) fn compute_module(
  module: &naga::Module,
  source_type_flags: BitFlags<WgslShaderSourceType>,
) -> TokenStream {
  ComputeModuleBuilder::new(module, source_type_flags).build()
}

/// Generate a `create_shader_module_embed_source` function that embeds the
/// pre-compiled WGSL produced by the WESL compiler at build time.
fn generate_shader_module_embedded(entry: &WgslEntryResult) -> TokenStream {
  let create_shader_module_fn =
    format_ident!("{}", WgslShaderSourceType::EmbedSource.create_shader_module_fn_name());
  let shader_literal = create_shader_raw_string_literal(&entry.wgsl_source);
  let shader_label = entry.get_label();
  let create_shader_module = quote! {
      pub fn #create_shader_module_fn(device: &wgpu::Device) -> wgpu::ShaderModule {
          let source = std::borrow::Cow::Borrowed(SHADER_STRING);
          device.create_shader_module(wgpu::ShaderModuleDescriptor {
              label: #shader_label,
              source: wgpu::ShaderSource::Wgsl(source)
          })
      }
  };
  let shader_str_def = quote!(pub const SHADER_STRING: &str = #shader_literal;);

  quote! {
    #create_shader_module
    #shader_str_def
  }
}

/// Generate a `create_shader_module_from_path` function that uses the WESL compiler
/// at **runtime** to load and compile shaders from disk.
///
/// The generated function signature is:
/// ```rust,ignore
/// pub fn create_shader_module_from_path(
///     device: &wgpu::Device,
///     base_dir: &::std::path::Path,
/// ) -> ::std::result::Result<wgpu::ShaderModule, ::std::string::String>
/// ```
fn generate_shader_module_from_path(entry: &WgslEntryResult) -> TokenStream {
  let fn_name = format_ident!(
    "{}",
    WgslShaderSourceType::WeslWithRelativePath.create_shader_module_fn_name()
  );
  let shader_label = entry.get_label();
  let wesl_module_path = entry.wesl_module_path();

  quote! {
    /// Load and compile this shader at runtime from `base_dir` using the WESL compiler.
    ///
    /// `base_dir` should be the same directory that was used as `workspace_root` when
    /// generating these bindings (i.e. the root that contains the shader files).
    ///
    /// **Runtime dependency**: requires the `wesl` crate with the `naga-ext` feature.
    pub fn #fn_name(
        device: &wgpu::Device,
        base_dir: &::std::path::Path,
    ) -> ::std::result::Result<wgpu::ShaderModule, ::std::string::String> {
        let module_path: ::wesl::syntax::ModulePath = #wesl_module_path
            .parse()
            .expect("wgsl_bindgen generated an invalid WESL module path");
        let compiler = ::wesl::Wesl::new(base_dir);
        let compiled = compiler
            .compile(&module_path)
            .map_err(|e| e.to_string())?;
        let source = compiled.to_string();
        ::std::result::Result::Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: #shader_label,
            source: wgpu::ShaderSource::Wgsl(::std::borrow::Cow::Owned(source)),
        }))
    }
  }
}

pub(crate) fn shader_module(
  entry: &WgslEntryResult,
  options: &WgslBindgenOption,
) -> TokenStream {
  let mut tokens = generate_shader_module_embedded(entry);
  if options
    .shader_source_type
    .contains(WgslShaderSourceType::WeslWithRelativePath)
  {
    tokens.extend(generate_shader_module_from_path(entry));
  }
  tokens
}

fn create_canonical_variable_name(name: &str, is_const: bool) -> String {
  let canonical_name = name
    .replace("::", "_")
    .replace(" ", "_")
    .chars()
    .filter(|c| c.is_alphanumeric() || *c == '_')
    .collect::<String>();

  if is_const {
    canonical_name.to_uppercase()
  } else {
    canonical_name.to_lowercase()
  }
}

#[cfg(test)]
mod tests {
  use indoc::indoc;

  use super::*;
  use crate::assert_tokens_snapshot;

  #[test]
  fn test_create_canonical_variable_name() {
    assert_eq!("foo", create_canonical_variable_name("Foo", false));
    assert_eq!("FOO", create_canonical_variable_name("Foo", true));
    assert_eq!("foo_bar", create_canonical_variable_name("Foo::Bar", false));
    assert_eq!("FOO_BAR", create_canonical_variable_name("Foo::Bar", true));
    assert_eq!("foo_bar", create_canonical_variable_name("Foo Bar", false));
    assert_eq!("FOO_BAR", create_canonical_variable_name("Foo Bar", true));
  }

  #[test]
  fn write_compute_module_empty() {
    let source = indoc! {r#"
            @vertex
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = compute_module(&module, WgslShaderSourceType::EmbedSource.into());

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_compute_module_multiple_entries() {
    let source = indoc! {r#"
            @compute
            @workgroup_size(1,2,3)
            fn main1() {}

            @compute
            @workgroup_size(256)
            fn main2() {}
        "#
    };

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = compute_module(&module, WgslShaderSourceType::EmbedSource.into());

    assert_tokens_snapshot!(actual);
  }
}
