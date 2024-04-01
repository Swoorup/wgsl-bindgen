//! This file is used for creating direct shader file related functions:
//! such as `create_shader_module`, `create_compute_module`

use std::path::Path;

use derive_more::Constructor;
use enumflags2::BitFlags;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, TokenStreamExt};
use syn::{Ident, Index};

use crate::naga_util::module_to_source;
use crate::quote_gen::create_shader_raw_string_literal;
use crate::{WgslBindgenOption, WgslEntryResult, WgslShaderSourceType};

impl<'a> WgslEntryResult<'a> {
  fn get_label(&self) -> TokenStream {
    let get_label = || {
      Some(
        self
          .source_including_deps
          .source_file
          .file_path
          .file_name()?
          .to_str()?,
      )
    };

    match get_label() {
      Some(label) => quote!(Some(#label)),
      None => quote!(None),
    }
  }
}

impl WgslShaderSourceType {
  pub(crate) fn create_shader_module_fn_name(&self) -> &'static str {
    use WgslShaderSourceType::*;
    match self {
      UseEmbed => "create_shader_module_embed_source",
      UseComposerEmbed => "create_shader_module_embedded",
      UseComposerWithPath => "create_shader_module_from_path",
    }
  }

  pub(crate) fn create_compute_pipeline_fn_name(&self, name: &str) -> String {
    use WgslShaderSourceType::*;
    match self {
      UseEmbed => format!("create_{}_pipeline_embed_source", name),
      UseComposerEmbed => format!("create_{}_pipeline_embedded", name),
      UseComposerWithPath => format!("create_{}_pipeline_from_path", name),
    }
  }

  pub(crate) fn get_return_type(&self, type_to_return: TokenStream) -> TokenStream {
    use WgslShaderSourceType::*;
    match self {
      UseEmbed | UseComposerEmbed => type_to_return,
      UseComposerWithPath => {
        quote!(Result<#type_to_return, naga_oil::compose::ComposerError>)
      }
    }
  }

  pub(crate) fn wrap_return_stmt(&self, stm: TokenStream) -> TokenStream {
    use WgslShaderSourceType::*;
    match self {
      UseComposerWithPath => quote!(Ok(#stm)),
      _ => stm,
    }
  }

  pub(crate) fn get_propagate_operator(&self) -> TokenStream {
    use WgslShaderSourceType::*;
    match self {
      UseComposerWithPath => quote!(?),
      _ => quote!(),
    }
  }

  pub(crate) fn add_composable_naga_module_stmt(
    &self,
    source: TokenStream,
    relative_file_path: String,
    as_name_assignment: TokenStream,
  ) -> TokenStream {
    use WgslShaderSourceType::*;

    let first_part = quote! {
      composer.add_composable_module(
        naga_oil::compose::ComposableModuleDescriptor {
          source: #source,
          file_path: #relative_file_path,
          language: naga_oil::compose::ShaderLanguage::Wgsl,
          shader_defs: shader_defs.clone(),
          #as_name_assignment,
          ..Default::default()
        }
      )
    };
    match self {
      UseComposerWithPath => quote! {
        #first_part ?;
      },
      UseComposerEmbed => quote! {
        #first_part.expect("failed to add composer module");
      },
      _ => panic!("Not supported"),
    }
  }

  pub(crate) fn naga_module_ret_stmt(
    &self,
    source: TokenStream,
    relative_file_path: String,
  ) -> TokenStream {
    use WgslShaderSourceType::*;
    match self {
      UseComposerWithPath => quote! {
        composer.make_naga_module(naga_oil::compose::NagaModuleDescriptor {
          source: #source,
          file_path: #relative_file_path,
          shader_defs,
          ..Default::default()
        })
      },
      UseComposerEmbed => quote! {
        composer.make_naga_module(naga_oil::compose::NagaModuleDescriptor {
          source: #source,
          file_path: #relative_file_path,
          shader_defs,
          ..Default::default()
        }).expect("failed to build naga module")
      },
      _ => panic!("Not supported"),
    }
  }

  pub(crate) fn unwrap_result(&self) -> TokenStream {
    use WgslShaderSourceType::*;
    match self {
      UseComposerWithPath => quote!(.unwrap()),
      _ => quote!(),
    }
  }

  pub(crate) fn shader_module_params_defs_and_params(
    &self,
  ) -> (TokenStream, TokenStream) {
    use WgslShaderSourceType::*;
    match self {
      UseEmbed => {
        let param_defs = quote!(device: &wgpu::Device);
        let params = quote!(device);
        (param_defs, params)
      }
      UseComposerEmbed | UseComposerWithPath => {
        let param_defs = quote! {
          device: &wgpu::Device,
          shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>
        };
        let params = quote!(device, shader_defs);
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

    let pipeline_name =
      format_ident!("{}", source_type.create_compute_pipeline_fn_name(&e.name));

    let entry_point = &e.name;
    // TODO: Include a user supplied module name in the label?
    let label = format!("Compute Pipeline {}", e.name);

    let create_shader_module_fn_name =
      format_ident!("{}", source_type.create_shader_module_fn_name());

    let unwrap_result = source_type.unwrap_result();

    let (param_defs, params) = source_type.shader_module_params_defs_and_params();

    quote! {
        pub fn #pipeline_name(#param_defs) -> wgpu::ComputePipeline {
            let module = super::#create_shader_module_fn_name(#params) #unwrap_result;
            let layout = super::create_pipeline_layout(device);
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(#label),
                layout: Some(&layout),
                module: &module,
                entry_point: #entry_point,
            })
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

fn generate_shader_module_embedded(entry: &WgslEntryResult) -> TokenStream {
  let shader_content = module_to_source(&entry.naga_module).unwrap();
  let create_shader_module_fn =
    format_ident!("{}", WgslShaderSourceType::UseEmbed.create_shader_module_fn_name());
  let shader_literal = create_shader_raw_string_literal(&shader_content);
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
  let shader_str_def = quote!(pub const SHADER_STRING: &'static str = #shader_literal;);

  quote! {
    #create_shader_module
    #shader_str_def
  }
}

struct ComposeShaderModuleBuilder<'a, 'b> {
  entry: &'a WgslEntryResult<'b>,
  entry_source_path: &'a Path,
  output_dir: &'a Path,
  source_type: WgslShaderSourceType,
}

impl<'a, 'b> ComposeShaderModuleBuilder<'a, 'b> {
  fn new(
    entry: &'a WgslEntryResult<'b>,
    output_dir: &'a Path,
    source_type: WgslShaderSourceType,
  ) -> Self {
    let entry_source_path = entry.source_including_deps.source_file.file_path.as_path();

    Self {
      entry,
      output_dir,
      source_type,
      entry_source_path,
    }
  }

  fn generate_constants_for_paths(&self) -> TokenStream {
    if !self.source_type.is_use_composer_with_path() {
      return quote!();
    }

    let (mut module_vars, mut assignments): (Vec<Ident>, Vec<TokenStream>) = self
      .entry
      .source_including_deps
      .full_dependencies
      .iter()
      .map(|dep| {
        let module_name = dep
          .module_name
          .as_ref()
          .map(|name| name.to_string())
          .unwrap()
          .to_uppercase();

        let module_name_var = format_ident!("{}_PATH",
          create_canonical_variable_name(&module_name, true)
        );

        let relative_file_path = get_path_relative_to(&self.output_dir, &dep.file_path);

        let assignment = quote! {
          pub const #module_name_var: &str = include_file_path::include_file_path!(#relative_file_path);
        };

        (module_name_var, assignment)
      }).unzip();

    let shader_entry_path =
      get_path_relative_to(&self.output_dir, &self.entry_source_path);
    let entry_name_var = format_ident!("SHADER_ENTRY_PATH");

    let assignment = quote! {
      pub const #entry_name_var: &str = include_file_path::include_file_path!(#shader_entry_path);
    };

    module_vars.insert(0, entry_name_var);
    assignments.insert(0, assignment);

    quote! {
      #(#assignments)*
      pub const SHADER_PATHS: &[&str] = &[
        #(
          #module_vars,
        )*
      ];
    }
  }

  fn load_shader_modules_fn_name(&self) -> Ident {
    if self.source_type.is_use_composer_with_path() {
      format_ident!("load_shader_modules_from_path")
    } else {
      format_ident!("load_shader_modules_embedded")
    }
  }

  fn load_naga_module_fn_name(&self) -> Ident {
    if self.source_type.is_use_composer_with_path() {
      format_ident!("load_naga_module_from_path")
    } else {
      format_ident!("load_naga_module_embedded")
    }
  }

  fn create_shader_module_fn_name(&self) -> Ident {
    let name = self.source_type.create_shader_module_fn_name();
    format_ident!("{}", name)
  }

  fn load_shader_modules_fn(&self) -> TokenStream {
    let dependency_modules = self
      .entry
      .source_including_deps
      .full_dependencies
      .iter()
      .map(|dep| {
        let as_name = dep
          .module_name
          .as_ref()
          .map(|name| name.to_string())
          .unwrap();
        let as_name_assignment = quote! { as_name: Some(#as_name.into()) };

        let relative_file_path = get_path_relative_to(&self.output_dir, &dep.file_path);
        let source = if self.source_type.is_use_composer_with_path() {
          let mod_var =
            format_ident!("{}_PATH", create_canonical_variable_name(&as_name, true));
          quote!(&std::fs::read_to_string(#mod_var).unwrap())
        } else {
          quote!(include_str!(#relative_file_path))
        };

        self.source_type.add_composable_naga_module_stmt(
          source,
          relative_file_path,
          as_name_assignment,
        )
      })
      .collect::<Vec<_>>();

    let fn_name = self.load_shader_modules_fn_name();
    let return_type = self.source_type.get_return_type(quote!(()));
    let return_stmt = self.source_type.wrap_return_stmt(quote!(()));
    quote! {
      pub fn #fn_name(
        composer: &mut naga_oil::compose::Composer,
        shader_defs: &std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>
      ) -> #return_type {
        #(#dependency_modules)*
        #return_stmt
      }
    }
  }

  fn load_naga_module_fn(&self) -> TokenStream {
    let load_naga_module_fn_name = self.load_naga_module_fn_name();

    let relative_file_path =
      get_path_relative_to(self.output_dir, &self.entry_source_path);

    let source = if self.source_type.is_use_composer_with_path() {
      let mod_var = format_ident!("SHADER_ENTRY_PATH");
      quote!(&std::fs::read_to_string(#mod_var).unwrap())
    } else {
      quote!(include_str!(#relative_file_path))
    };

    let return_type = self.source_type.get_return_type(quote!(wgpu::naga::Module));
    let make_naga_module_stmt = self
      .source_type
      .naga_module_ret_stmt(source, relative_file_path);

    quote! {
      pub fn #load_naga_module_fn_name(
        composer: &mut naga_oil::compose::Composer,
        shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>
      ) -> #return_type {
        #make_naga_module_stmt
      }
    }
  }

  fn create_shader_module_fn(&self) -> TokenStream {
    let create_shader_module_fn = self.create_shader_module_fn_name();
    let load_shader_module_fn = self.load_shader_modules_fn_name();
    let load_naga_module_fn = self.load_naga_module_fn_name();
    let shader_label = self.entry.get_label();
    let return_type = self.source_type.get_return_type(quote!(wgpu::ShaderModule));
    let propagate_operator = self.source_type.get_propagate_operator();
    let return_stmt = self.source_type.wrap_return_stmt(quote! {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
          label: #shader_label,
          source: wgpu::ShaderSource::Wgsl(source)
        })
    });

    quote! {
      pub fn #create_shader_module_fn(
        device: &wgpu::Device,
        shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>
      ) -> #return_type {

        let mut composer = naga_oil::compose::Composer::default();
        #load_shader_module_fn (&mut composer, &shader_defs) #propagate_operator;
        let module = #load_naga_module_fn (&mut composer, shader_defs) #propagate_operator;

        // Mini validation to get module info
        let info = wgpu::naga::valid::Validator::new(
          wgpu::naga::valid::ValidationFlags::empty(),
          wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();

        // Write to wgsl
        let shader_string = wgpu::naga::back::wgsl::write_string(
          &module,
          &info,
          wgpu::naga::back::wgsl::WriterFlags::empty(),
        ).expect("failed to convert naga module to source");

        let source = std::borrow::Cow::Owned(shader_string);
        #return_stmt
      }
    }
  }

  fn build(&self) -> TokenStream {
    let constants = self.generate_constants_for_paths();
    let load_shader_modules_fn = self.load_shader_modules_fn();
    let load_naga_module_fn = self.load_naga_module_fn();
    let create_shader_module_fn = self.create_shader_module_fn();

    quote! {
      #constants
      #load_shader_modules_fn
      #load_naga_module_fn
      #create_shader_module_fn
    }
  }
}

pub(crate) fn shader_module(
  entry: &WgslEntryResult,
  options: &WgslBindgenOption,
) -> TokenStream {
  use WgslShaderSourceType::*;
  let source_type = options.shader_source_type;
  let output_dir = options
    .output
    .as_ref()
    .and_then(|output_file| output_file.parent().map(|p| p.to_path_buf()))
    .unwrap_or_else(|| {
      std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".into())
        .into()
    });

  let mut token_stream = TokenStream::new();

  if source_type.contains(UseEmbed) {
    token_stream.append_all(generate_shader_module_embedded(entry));
  }

  if source_type.contains(UseComposerEmbed) {
    let builder = ComposeShaderModuleBuilder::new(entry, &output_dir, UseComposerEmbed);
    token_stream.append_all(builder.build());
  }

  if source_type.contains(UseComposerWithPath) {
    let builder =
      ComposeShaderModuleBuilder::new(entry, &output_dir, UseComposerWithPath);
    token_stream.append_all(builder.build());
  }

  token_stream
}

fn get_path_relative_to(relative_to: &std::path::Path, file: &std::path::Path) -> String {
  pathdiff::diff_paths(file, relative_to)
    .expect("failed to get relative path")
    .to_str()
    .unwrap()
    .to_string()
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
  use crate::assert_tokens_eq;

  #[test]
  fn test_create_canonical_variable_name() {
    assert_eq!(create_canonical_variable_name("Foo", false), "foo");
    assert_eq!(create_canonical_variable_name("Foo", true), "FOO");
    assert_eq!(create_canonical_variable_name("Foo::Bar", false), "foo_bar");
    assert_eq!(create_canonical_variable_name("Foo::Bar", true), "FOO_BAR");
    assert_eq!(create_canonical_variable_name("Foo Bar", false), "foo_bar");
    assert_eq!(create_canonical_variable_name("Foo Bar", true), "FOO_BAR");
  }

  #[test]
  fn write_compute_module_empty() {
    let source = indoc! {r#"
            @vertex
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = compute_module(&module, WgslShaderSourceType::UseEmbed.into());

    assert_tokens_eq!(quote!(), actual);
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
    let actual = compute_module(&module, WgslShaderSourceType::UseEmbed.into());

    assert_tokens_eq!(
      quote! {
          pub mod compute {
              pub const MAIN1_WORKGROUP_SIZE: [u32; 3] = [1, 2, 3];
              pub fn create_main1_pipeline_embed_source(device: &wgpu::Device) -> wgpu::ComputePipeline {
                  let module = super::create_shader_module_embed_source(device);
                  let layout = super::create_pipeline_layout(device);
                  device
                      .create_compute_pipeline(
                          &wgpu::ComputePipelineDescriptor {
                              label: Some("Compute Pipeline main1"),
                              layout: Some(&layout),
                              module: &module,
                              entry_point: "main1",
                          },
                      )
              }
              pub const MAIN2_WORKGROUP_SIZE: [u32; 3] = [256, 1, 1];
              pub fn create_main2_pipeline_embed_source(device: &wgpu::Device) -> wgpu::ComputePipeline {
                  let module = super::create_shader_module_embed_source(device);
                  let layout = super::create_pipeline_layout(device);
                  device
                      .create_compute_pipeline(
                          &wgpu::ComputePipelineDescriptor {
                              label: Some("Compute Pipeline main2"),
                              layout: Some(&layout),
                              module: &module,
                              entry_point: "main2",
                          },
                      )
              }
          }
      },
      actual
    );
  }
}
