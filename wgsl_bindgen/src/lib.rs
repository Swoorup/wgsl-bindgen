//! # wgsl_bindgen
//! wgsl_bindgen is an experimental library for generating typesafe Rust bindings from WGSL shaders to [wgpu](https://github.com/gfx-rs/wgpu).
//!
//! ## Features
//! The `WgslBindgenOptionBuilder` is used to configure the generation of Rust bindings from WGSL shaders. This facilitates a shader focused workflow where edits to WGSL code are automatically reflected in the corresponding Rust file. For example, changing the type of a uniform in WGSL will raise a compile error in Rust code using the generated struct to initialize the buffer.
//!
//! Writing Rust code to interact with WGSL shaders can be tedious and error prone, especially when the types and functions in the shader code change during development. wgsl_bindgen is not a rendering library and does not offer high level abstractions like a scene graph or material system. However, using generated code still has a number of advantages compared to writing the code by hand.
//!
//! The code generated by wgsl_bindgen can help with valid API usage like:
//! - setting all bind groups and bind group bindings
//! - setting correct struct fields and field types for vertex input buffers
//! - setting correct struct struct fields and field types for storage and uniform buffers
//! - configuring shader initialization
//! - getting vertex attribute offsets for vertex buffers
//! - const validation of struct memory layouts when using bytemuck
//!
//! Here's an example of how to use `WgslBindgenOptionBuilder` to generate Rust bindings from WGSL shaders:
//!
//! ```no_run
//! use miette::{IntoDiagnostic, Result};
//! use wgsl_bindgen::{WgslTypeSerializeStrategy, WgslBindgenOptionBuilder, GlamWgslTypeMap};
//!
//! fn main() -> Result<()> {
//!     WgslBindgenOptionBuilder::default()
//!         .workspace_root("src/shader")
//!         .add_entry_point("src/shader/testbed.wgsl")
//!         .add_entry_point("src/shader/triangle.wgsl")
//!         .skip_hash_check(true)
//!         .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
//!         .type_map(GlamWgslTypeMap)
//!         .derive_serde(false)
//!         .output("src/shader.rs".to_string())
//!         .build()?
//!         .generate()
//!         .into_diagnostic()
//! }
//! ```

#[allow(dead_code, unused)]
extern crate wgpu_types as wgpu;

use bevy_util::SourceWithFullDependenciesResult;
use case::CaseExt;
use derive_more::IsVariant;
use generate::entry::{self, entry_point_constants, vertex_struct_impls};
use generate::{bind_group, consts, pipeline, shader_module, shader_registry};
use heck::ToPascalCase;
use proc_macro2::{Span, TokenStream};
use qs::{format_ident, quote, Ident, Index};
use quote_gen::{custom_vector_matrix_assertions, RustModBuilder, MOD_STRUCT_ASSERTIONS};
use thiserror::Error;

pub mod bevy_util;
mod bindgen;
mod generate;
mod naga_util;
mod quote_gen;
mod structs;
mod types;
mod wgsl;
mod wgsl_type;

pub mod qs {
  pub use proc_macro2::TokenStream;
  pub use quote::{format_ident, quote};
  pub use syn::{Ident, Index};
}

pub use bindgen::*;
pub use naga::FastIndexMap;
pub use regex::Regex;
pub use types::*;
pub use wgsl_type::*;

/// Enum representing the possible serialization strategies for WGSL types.
///
/// This enum is used to specify how WGSL types should be serialized when converted
/// to Rust types.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, IsVariant)]
pub enum WgslTypeSerializeStrategy {
  #[default]
  Encase,
  Bytemuck,
}

/// Errors while generating Rust source for a WGSl shader module.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum CreateModuleError {
  /// Bind group sets must be consecutive and start from 0.
  /// See `bind_group_layouts` for
  /// [PipelineLayoutDescriptor](https://docs.rs/wgpu/latest/wgpu/struct.PipelineLayoutDescriptor.html#).
  #[error("bind groups are non-consecutive or do not start from 0")]
  NonConsecutiveBindGroups,

  /// Each binding resource must be associated with exactly one binding index.
  #[error("duplicate binding found with index `{binding}`")]
  DuplicateBinding { binding: u32 },
}

#[derive(Debug)]
pub(crate) struct WgslEntryResult<'a> {
  mod_name: String,
  naga_module: naga::Module,
  source_including_deps: SourceWithFullDependenciesResult<'a>,
}

fn create_rust_bindings(
  entries: Vec<WgslEntryResult<'_>>,
  options: &WgslBindgenOption,
) -> Result<String, CreateModuleError> {
  let mut mod_builder = RustModBuilder::new(true, true);

  if let Some(custom_wgsl_type_asserts) = custom_vector_matrix_assertions(options) {
    mod_builder.add(MOD_STRUCT_ASSERTIONS, custom_wgsl_type_asserts);
  }

  for entry in entries.iter() {
    let WgslEntryResult {
      mod_name,
      naga_module,
      ..
    } = entry;
    let entry_name = sanitize_and_pascal_case(&mod_name);
    let bind_group_data = bind_group::get_bind_group_data(naga_module)?;
    let shader_stages = wgsl::shader_stages(naga_module);

    // Write all the structs, including uniforms and entry function inputs.
    mod_builder
      .add_items(structs::structs_items(&mod_name, naga_module, options))
      .unwrap();

    mod_builder
      .add_items(consts::consts_items(&mod_name, naga_module))
      .unwrap();

    mod_builder
      .add(mod_name, consts::pipeline_overridable_constants(naga_module, options));

    mod_builder
      .add_items(vertex_struct_impls(mod_name, naga_module))
      .unwrap();

    mod_builder
      .add_items(bind_group::generate_bind_groups_module(
        &mod_name,
        &options,
        naga_module,
        &bind_group_data,
        shader_stages,
      ))
      .unwrap();

    mod_builder.add(
      mod_name,
      shader_module::compute_module(naga_module, options.shader_source_type),
    );
    mod_builder.add(mod_name, entry_point_constants(naga_module));

    mod_builder.add(mod_name, entry::vertex_states(mod_name, naga_module));
    mod_builder.add(mod_name, entry::fragment_states(naga_module));

    let create_pipeline_layout = pipeline::create_pipeline_layout_fn(
      &entry_name,
      naga_module,
      shader_stages,
      &options,
      &bind_group_data,
    );

    mod_builder.add(mod_name, create_pipeline_layout);
    mod_builder.add(mod_name, shader_module::shader_module(entry, options));
  }

  let mod_token_stream = mod_builder.generate();
  let shader_registry =
    shader_registry::build_shader_registry(&entries, options.shader_source_type);

  let output = quote! {
    #![allow(unused, non_snake_case, non_camel_case_types, non_upper_case_globals)]

    #shader_registry
    #mod_token_stream
  };

  Ok(pretty_print(&output))
}

fn pretty_print(tokens: &TokenStream) -> String {
  let file = syn::parse_file(&tokens.to_string()).unwrap();
  prettyplease::unparse(&file)
}

fn indexed_name_ident(name: &str, index: u32) -> Ident {
  format_ident!("{name}{index}")
}

fn sanitize_and_pascal_case(v: &str) -> String {
  v.chars()
    .filter(|ch| ch.is_alphanumeric() || *ch == '_')
    .collect::<String>()
    .to_pascal_case()
}

fn sanitized_upper_snake_case(v: &str) -> String {
  v.chars()
    .filter(|ch| ch.is_alphanumeric() || *ch == '_')
    .collect::<String>()
    .to_snake()
    .to_uppercase()
}

// Tokenstreams can't be compared directly using PartialEq.
// Use pretty_print to normalize the formatting and compare strings.
// Use a colored diff output to make differences easier to see.
#[cfg(test)]
#[macro_export]
macro_rules! assert_tokens_eq {
  ($a:expr, $b:expr) => {
    pretty_assertions::assert_eq!(crate::pretty_print(&$a), crate::pretty_print(&$b))
  };
}

#[cfg(test)]
mod test {
  use indoc::indoc;

  use self::bevy_util::source_file::SourceFile;
  use super::*;

  fn create_shader_module(
    source: &str,
    options: WgslBindgenOption,
  ) -> Result<String, CreateModuleError> {
    let naga_module = naga::front::wgsl::parse_str(source).unwrap();
    let dummy_source = SourceFile::create(SourceFilePath::new(""), None, "".into());
    let entry = WgslEntryResult {
      mod_name: "test".into(),
      naga_module,
      source_including_deps: SourceWithFullDependenciesResult {
        full_dependencies: Default::default(),
        source_file: &dummy_source,
      },
    };

    Ok(create_rust_bindings(vec![entry], &options)?)
  }

  #[test]
  fn create_shader_module_embed_source() {
    let source = indoc! {r#"
      var<push_constant> consts: vec4<f32>;

      @fragment
      fn fs_main() {}
    "#};

    let actual = create_shader_module(source, WgslBindgenOption::default()).unwrap();

    assert_tokens_eq!(
      quote! {
          #![allow(unused, non_snake_case, non_camel_case_types, non_upper_case_globals)]
          #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
          pub enum ShaderEntry {
              Test,
          }
          impl ShaderEntry {
              pub fn create_pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
                  match self {
                      Self::Test => test::create_pipeline_layout(device),
                  }
              }
              pub fn create_shader_module_embed_source(
                  &self,
                  device: &wgpu::Device,
              ) -> wgpu::ShaderModule {
                  match self {
                      Self::Test => test::create_shader_module_embed_source(device),
                  }
              }
          }
          mod _root {
              pub use super::*;
          }
          pub mod test {
              use super::{_root, _root::*};
              pub const ENTRY_FS_MAIN: &str = "fs_main";
              #[derive(Debug)]
              pub struct FragmentEntry<const N: usize> {
                  pub entry_point: &'static str,
                  pub targets: [Option<wgpu::ColorTargetState>; N],
                  pub constants: std::collections::HashMap<String, f64>,
              }
              pub fn fragment_state<'a, const N: usize>(
                  module: &'a wgpu::ShaderModule,
                  entry: &'a FragmentEntry<N>,
              ) -> wgpu::FragmentState<'a> {
                  wgpu::FragmentState {
                      module,
                      entry_point: Some(entry.entry_point),
                      targets: &entry.targets,
                      compilation_options: wgpu::PipelineCompilationOptions {
                          constants: &entry.constants,
                          ..Default::default()
                      },
                  }
              }
              pub fn fs_main_entry(
                  targets: [Option<wgpu::ColorTargetState>; 0],
              ) -> FragmentEntry<0> {
                  FragmentEntry {
                      entry_point: ENTRY_FS_MAIN,
                      targets,
                      constants: Default::default(),
                  }
              }
              #[derive(Debug)]
              pub struct WgpuPipelineLayout;
              impl WgpuPipelineLayout {
                  pub fn bind_group_layout_entries(
                      entries: [wgpu::BindGroupLayout; 0],
                  ) -> [wgpu::BindGroupLayout; 0] {
                      entries
                  }
              }
              pub fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
                  device
                      .create_pipeline_layout(
                          &wgpu::PipelineLayoutDescriptor {
                              label: Some("Test::PipelineLayout"),
                              bind_group_layouts: &[],
                              push_constant_ranges: &[
                                  wgpu::PushConstantRange {
                                      stages: wgpu::ShaderStages::FRAGMENT,
                                      range: 0..16,
                                  },
                              ],
                          },
                      )
              }
              pub fn create_shader_module_embed_source(
                  device: &wgpu::Device,
              ) -> wgpu::ShaderModule {
                  let source = std::borrow::Cow::Borrowed(SHADER_STRING);
                  device
                      .create_shader_module(wgpu::ShaderModuleDescriptor {
                          label: None,
                          source: wgpu::ShaderSource::Wgsl(source),
                      })
              }
              pub const SHADER_STRING: &'static str = r#"
var<push_constant> consts: vec4<f32>;

@fragment 
fn fs_main() {
    return;
}
"#;
          }
      },
      actual.parse().unwrap()
    );
  }

  #[test]
  fn create_shader_module_consecutive_bind_groups() {
    let source = indoc! {r#"
            struct A {
                f: vec4<f32>
            };
            @group(0) @binding(0) var<uniform> a: A;
            @group(1) @binding(0) var<uniform> b: A;

            @vertex
            fn vs_main() -> @builtin(position) vec4<f32> {
              return vec4<f32>(0.0, 0.0, 0.0, 1.0);
            }

            @fragment
            fn fs_main() {}
        "#};

    create_shader_module(source, WgslBindgenOption::default()).unwrap();
  }

  #[test]
  fn create_shader_module_non_consecutive_bind_groups() {
    let source = indoc! {r#"
            @group(0) @binding(0) var<uniform> a: vec4<f32>;
            @group(1) @binding(0) var<uniform> b: vec4<f32>;
            @group(3) @binding(0) var<uniform> c: vec4<f32>;

            @fragment
            fn main() {}
        "#};

    let result = create_shader_module(source, WgslBindgenOption::default());
    assert!(matches!(result, Err(CreateModuleError::NonConsecutiveBindGroups)));
  }

  #[test]
  fn create_shader_module_repeated_bindings() {
    let source = indoc! {r#"
            struct A {
                f: vec4<f32>
            };
            @group(0) @binding(2) var<uniform> a: A;
            @group(0) @binding(2) var<uniform> b: A;

            @fragment
            fn main() {}
        "#};

    let result = create_shader_module(source, WgslBindgenOption::default());
    assert!(matches!(result, Err(CreateModuleError::DuplicateBinding { binding: 2 })));
  }
}
