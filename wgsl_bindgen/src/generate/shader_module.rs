//! This file is used for creating direct shader file related functions:
//! such as `create_shader_module`, `create_compute_module`

use std::path::Path;

use derive_more::Constructor;
use enumflags2::BitFlags;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, TokenStreamExt};
use syn::{Ident, Index};

use crate::generate::quote_naga_capabilities;
use crate::naga_util::module_to_source;
use crate::quote_gen::create_shader_raw_string_literal;
use crate::{WgslBindgenOption, WgslEntryResult, WgslShaderSourceType};

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
    use WgslShaderSourceType::*;
    match self {
      EmbedSource => "create_shader_module_embed_source",
      EmbedWithNagaOilComposer => "create_shader_module_embedded",
      ComposerWithRelativePath => "create_shader_module_relative_path",
    }
  }

  pub(crate) fn load_shader_module_fn_name(&self) -> Ident {
    use WgslShaderSourceType::*;
    match self {
      ComposerWithRelativePath => format_ident!("load_naga_module_from_path"),
      _ => format_ident!("load_shader_module_embedded"),
    }
  }

  pub(crate) fn create_compute_pipeline_fn_name(&self, name: &str) -> Ident {
    use WgslShaderSourceType::*;
    match self {
      EmbedSource => format_ident!("create_{}_pipeline_embed_source", name),
      EmbedWithNagaOilComposer => {
        format_ident!("create_{}_pipeline_embedded", name)
      }
      ComposerWithRelativePath => {
        format_ident!("create_{}_pipeline_relative_path", name)
      }
    }
  }

  pub(crate) fn get_return_type(&self, type_to_return: TokenStream) -> TokenStream {
    use WgslShaderSourceType::*;
    match self {
      EmbedSource => type_to_return,
      EmbedWithNagaOilComposer | ComposerWithRelativePath => {
        quote!(Result<#type_to_return, naga_oil::compose::ComposerError>)
      }
    }
  }

  pub(crate) fn wrap_return_stmt(&self, stm: TokenStream) -> TokenStream {
    use WgslShaderSourceType::*;
    match self {
      EmbedWithNagaOilComposer | ComposerWithRelativePath => quote!(Ok(#stm)),
      _ => stm,
    }
  }

  pub(crate) fn get_propagate_operator(&self) -> TokenStream {
    use WgslShaderSourceType::*;
    match self {
      EmbedWithNagaOilComposer | ComposerWithRelativePath => quote!(?),
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

    match self {
      EmbedWithNagaOilComposer | ComposerWithRelativePath => quote! {
        composer.add_composable_module(
          naga_oil::compose::ComposableModuleDescriptor {
            source: #source,
            file_path: #relative_file_path,
            language: naga_oil::compose::ShaderLanguage::Wgsl,
            shader_defs: shader_defs.clone(),
            #as_name_assignment,
            ..Default::default()
          }
        )?;
      },
      _ => panic!("Not supported"),
    }
  }

  pub(crate) fn generate_make_naga_module_statement(
    &self,
    source: TokenStream,
    relative_file_path: String,
  ) -> TokenStream {
    use WgslShaderSourceType::*;
    match self {
      EmbedWithNagaOilComposer | ComposerWithRelativePath => quote! {
        composer.make_naga_module(naga_oil::compose::NagaModuleDescriptor {
          source: #source,
          file_path: #relative_file_path,
          shader_defs,
          ..Default::default()
        })
      },
      _ => panic!("Not supported"),
    }
  }

  pub(crate) fn shader_module_params_defs_and_params(
    &self,
  ) -> (TokenStream, TokenStream) {
    use WgslShaderSourceType::*;
    match self {
      EmbedSource => {
        let param_defs = quote!(device: &wgpu::Device);
        let params = quote!(device);
        (param_defs, params)
      }
      EmbedWithNagaOilComposer => {
        let param_defs = quote! {
          device: &wgpu::Device,
          shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>
        };
        let params = quote!(device, shader_defs);
        (param_defs, params)
      }
      ComposerWithRelativePath => {
        let param_defs = quote! {
          device: &wgpu::Device,
          base_dir: &str,
          entry_point: ShaderEntry,
          shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>,
          load_file: impl Fn(&str) -> Result<String, std::io::Error>
        };
        let params = quote!(device, base_dir, entry_point, shader_defs, load_file);
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

fn generate_shader_module_embedded(entry: &WgslEntryResult) -> TokenStream {
  let shader_content = module_to_source(&entry.naga_module).unwrap();
  let create_shader_module_fn =
    format_ident!("{}", WgslShaderSourceType::EmbedSource.create_shader_module_fn_name());
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
  let shader_str_def = quote!(pub const SHADER_STRING: &str = #shader_literal;);

  quote! {
    #create_shader_module
    #shader_str_def
  }
}

struct ComposeShaderModuleBuilder<'a, 'b> {
  entry: &'a WgslEntryResult<'b>,
  capabilities: Option<naga::valid::Capabilities>,
  entry_source_path: &'a Path,
  output_dir: &'a Path,
  workspace_root: &'a Path,
  source_type: WgslShaderSourceType,
}

impl<'a, 'b> ComposeShaderModuleBuilder<'a, 'b> {
  fn new(
    entry: &'a WgslEntryResult<'b>,
    capabilities: Option<naga::valid::Capabilities>,
    output_dir: &'a Path,
    workspace_root: &'a Path,
    source_type: WgslShaderSourceType,
  ) -> Self {
    let entry_source_path = entry.source_including_deps.source_file.file_path.as_path();

    Self {
      entry,
      capabilities,
      output_dir,
      workspace_root,
      source_type,
      entry_source_path,
    }
  }

  fn generate_constants_for_paths(&self) -> TokenStream {
    use WgslShaderSourceType::*;

    match self.source_type {
      ComposerWithRelativePath => {
        let shader_entry_path =
          get_path_relative_to(self.workspace_root, self.entry_source_path);
        quote! {
          pub const SHADER_ENTRY_PATH: &str = #shader_entry_path;
        }
      }
      _ => quote!(),
    }
  }

  fn create_shader_module_fn_name(&self) -> Ident {
    let name = self.source_type.create_shader_module_fn_name();
    format_ident!("{}", name)
  }

  fn build_shader_dependency_modules_statements(&self) -> Vec<TokenStream> {
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

        let relative_file_path = get_path_relative_to(self.output_dir, &dep.file_path);
        let source = quote!(include_str!(#relative_file_path));

        self.source_type.add_composable_naga_module_stmt(
          source,
          relative_file_path,
          as_name_assignment,
        )
      })
      .collect::<Vec<_>>();

    dependency_modules
  }

  fn build_load_shader_module_fn(&self) -> TokenStream {
    use WgslShaderSourceType::*;

    let load_shader_module_fn_name = self.source_type.load_shader_module_fn_name();
    let return_type = self.source_type.get_return_type(quote!(wgpu::naga::Module));

    match self.source_type {
      ComposerWithRelativePath => {
        // For the new variant, we don't generate anything here - the global function handles it
        quote!()
      }
      _ => {
        // Keep existing implementation for other variants
        let dependency_modules = self.build_shader_dependency_modules_statements();
        let relative_file_path =
          get_path_relative_to(self.output_dir, self.entry_source_path);

        let source = quote!(include_str!(#relative_file_path));

        let make_naga_module_stmt = self
          .source_type
          .generate_make_naga_module_statement(source, relative_file_path);

        quote! {
          pub fn #load_shader_module_fn_name(
            composer: &mut naga_oil::compose::Composer,
            shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>
          ) -> #return_type {
            #(#dependency_modules)*
            #make_naga_module_stmt
          }
        }
      }
    }
  }

  fn create_shader_module_fn(&self) -> TokenStream {
    use WgslShaderSourceType::*;

    let create_shader_module_fn = self.create_shader_module_fn_name();
    let load_shader_module_fn_name = self.source_type.load_shader_module_fn_name();
    let shader_label = self.entry.get_label();
    let return_type = self.source_type.get_return_type(quote!(wgpu::ShaderModule));
    let propagate_operator = self.source_type.get_propagate_operator();
    let return_stmt = self.source_type.wrap_return_stmt(quote! { shader_module });

    let composer = quote!(naga_oil::compose::Composer::default());

    let composer_with_capabilities = match self.capabilities {
      Some(capabilities) => {
        let capabilities_expr = quote_naga_capabilities(capabilities);
        quote! {
          #composer.with_capabilities(#capabilities_expr)
        }
      }
      None => quote! {
        #composer
      },
    };

    match self.source_type {
      ComposerWithRelativePath => {
        quote! {
          pub fn #create_shader_module_fn(
            device: &wgpu::Device,
            base_dir: &str,
            entry_point: ShaderEntry,
            shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>,
            load_file: impl Fn(&str) -> Result<String, std::io::Error>,
          ) -> #return_type
          {
            let mut composer = #composer_with_capabilities;
            let module = load_naga_module_from_path(base_dir, entry_point, &mut composer, shader_defs, load_file).map_err(|e| {
              naga_oil::compose::ComposerError {
                inner: naga_oil::compose::ComposerErrorInner::ImportNotFound(e, 0),
                source: naga_oil::compose::ErrSource::Constructing {
                  path: "load_naga_module_from_path".to_string(),
                  source: "Generated code".to_string(),
                  offset: 0,
                },
              }
            })?;

            // Use naga-ir feature to create shader module directly from naga module
            let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
              label: #shader_label,
              source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module))
            });

            #return_stmt
          }
        }
      }
      _ => {
        quote! {
          pub fn #create_shader_module_fn(
            device: &wgpu::Device,
            shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>
          ) -> #return_type {

            let mut composer = #composer_with_capabilities;
            let module = #load_shader_module_fn_name (&mut composer, shader_defs) #propagate_operator;

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
            let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
              label: #shader_label,
              source: wgpu::ShaderSource::Wgsl(source)
            });

            #return_stmt
          }
        }
      }
    }
  }

  fn build(&self) -> TokenStream {
    use WgslShaderSourceType::*;

    let constants = self.generate_constants_for_paths();
    let load_shader_module_fn = self.build_load_shader_module_fn();
    let create_shader_module_fn = self.create_shader_module_fn();

    quote! {
      #constants
      #load_shader_module_fn
      #create_shader_module_fn
    }
  }
}

pub(crate) fn generate_global_load_naga_module_from_path() -> TokenStream {
  quote! {
    /// Visits and processes all shader files in a dependency tree.
    ///
    /// This function traverses the shader dependency tree and calls the visitor function
    /// for each file encountered. This allows for custom processing like hot reloading,
    /// caching, or debugging.
    ///
    /// # Arguments
    ///
    /// * `base_dir` - The base directory for resolving relative paths
    /// * `entry_point` - The shader entry point to start traversal from
    /// * `shader_defs` - Shader defines to be used during processing
    /// * `load_file` - Function to load file contents from a path
    /// * `visitor` - Function called for each file with (file_path, file_content)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all files were processed successfully, or an error string.
    pub fn visit_shader_files(
      base_dir: &str,
      entry_point: ShaderEntry,
      load_file: impl Fn(&str) -> Result<String, std::io::Error>,
      mut visitor: impl FnMut(&str, &str),
    ) -> Result<(), String> {
      fn visit_dependencies_recursive(
        base_dir: &str,
        source: &str,
        current_path: &str,
        load_file: &impl Fn(&str) -> Result<String, std::io::Error>,
        visitor: &mut impl FnMut(&str, &str),
        visited: &mut std::collections::HashSet<String>,
      ) -> Result<(), String> {
        // Use naga_oil's preprocessor to get import information
        let (_, imports, _) = naga_oil::compose::get_preprocessor_data(source);

        for import in imports {
          let import_path = if import.import.starts_with('\"') {
            // Strip quotes from string literals
            import.import
              .chars()
              .skip(1)
              .take_while(|c| *c != '\"')
              .collect::<String>()
          } else {
            // For module imports like "global_bindings::time", extract just the module name
            let module_path = if let Some(double_colon_pos) = import.import.find("::") {
              &import.import[..double_colon_pos]
            } else {
              &import.import
            };
            format!("{module_path}.wgsl")
          };

          // Resolve relative import path
          let full_import_path = if import_path.starts_with('/') || import_path.starts_with('\\') {
            format!("{base_dir}{import_path}")
          } else {
            let current_dir = std::path::Path::new(current_path)
              .parent()
              .and_then(|p| p.to_str())
              .unwrap_or("");
            if current_dir.is_empty() {
              std::path::Path::new(base_dir).join(import_path).display().to_string()
            } else {
              std::path::Path::new(base_dir).join(current_dir).join(import_path).display().to_string()
            }
          };

          // Skip if already visited
          if visited.contains(&full_import_path) {
            continue;
          }
          visited.insert(full_import_path.clone());

          // Load the imported file
          let import_source = load_file(&full_import_path)
            .map_err(|e| format!("Failed to load {full_import_path}: {e}"))?;

          // Call visitor for this file
          visitor(&full_import_path, &import_source);

          // Recursively visit its dependencies
          visit_dependencies_recursive(
            base_dir,
            &import_source,
            full_import_path.trim_start_matches(&format!("{base_dir}/")),
            load_file,
            visitor,
            visited,
          )?;
        }

        Ok(())
      }

      // Load entry point source
      let entry_path = format!("{}/{}", base_dir, entry_point.relative_path());
      let entry_source = load_file(&entry_path)
        .map_err(|e| format!("Failed to load entry point {entry_path}: {e}"))?;

      // Call visitor for entry point
      visitor(&entry_path, &entry_source);

      // Visit all dependencies
      let mut visited = std::collections::HashSet::new();
      visit_dependencies_recursive(
        base_dir,
        &entry_source,
        entry_point.relative_path(),
        &load_file,
        &mut visitor,
        &mut visited,
      )?;

      Ok(())
    }

    pub fn load_naga_module_from_path(
      base_dir: &str,
      entry_point: ShaderEntry,
      composer: &mut naga_oil::compose::Composer,
      shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>,
      load_file: impl Fn(&str) -> Result<String, std::io::Error>,
    ) -> Result<wgpu::naga::Module, String>
    {
      // Store file contents for later processing
      let mut files = std::collections::HashMap::<String, String>::new();

      // Use visit_shader_files to collect all files and their contents
      visit_shader_files(
        base_dir,
        entry_point,
        &load_file,
        |file_path, file_content| {
          files.insert(file_path.to_string(), file_content.to_string());
        }
      )?;

      // Process dependency files first (all except entry point)
      let entry_path = format!("{}/{}", base_dir, entry_point.relative_path());

      for (file_path, file_content) in &files {
        if *file_path == entry_path {
          continue; // Skip entry point, process it last
        }

        // Extract module name from file path (remove .wgsl extension)
        let relative_path = file_path.trim_start_matches(&format!("{base_dir}/"));
        let as_name = std::path::Path::new(relative_path)
          .file_stem()
          .and_then(|s| s.to_str())
          .map(|s| s.to_string());

        composer.add_composable_module(naga_oil::compose::ComposableModuleDescriptor {
          source: file_content,
          file_path: relative_path,
          language: naga_oil::compose::ShaderLanguage::Wgsl,
          shader_defs: shader_defs.clone(),
          as_name,
          ..Default::default()
        }).map_err(|e| format!("Failed to add composable module: {e}"))?;
      }

      // Get entry point content
      let entry_source = files.get(&entry_path)
        .ok_or_else(|| format!("Entry point file not found: {entry_path}"))?;

      // Create the final module
      composer.make_naga_module(naga_oil::compose::NagaModuleDescriptor {
        source: entry_source,
        file_path: entry_point.relative_path(),
        shader_defs,
        ..Default::default()
      }).map_err(|e| format!("Failed to create final module: {e}"))
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

  if source_type.contains(EmbedSource) {
    token_stream.append_all(generate_shader_module_embedded(entry));
  }

  let capabilities = options.ir_capabilities;

  if source_type.contains(EmbedWithNagaOilComposer) {
    let builder = ComposeShaderModuleBuilder::new(
      entry,
      capabilities,
      &output_dir,
      &output_dir,
      EmbedWithNagaOilComposer,
    );
    token_stream.append_all(builder.build());
  }

  if source_type.contains(ComposerWithRelativePath) {
    let builder = ComposeShaderModuleBuilder::new(
      entry,
      capabilities,
      &output_dir,
      &options.workspace_root,
      ComposerWithRelativePath,
    );
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
