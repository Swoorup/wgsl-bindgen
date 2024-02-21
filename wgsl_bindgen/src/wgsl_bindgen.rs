use std::io::Write;
use std::path::Path;

use derive_builder::Builder;
use miette::Diagnostic;
use naga_oil::compose::{
  ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderLanguage,
};
use thiserror::Error;

use self::source_file::SourceFile;
use crate::{bevy_util::*, WgslTypeMap, WgslTypeSerializeStrategy};
use crate::{create_rust_bindings, CreateModuleError, SourceFilePath, WriteOptions};

const PKG_VER: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

/// Enum representing the possible errors that can occur in the `wgsl_bindgen` process.
///
/// This enum is used to represent all the different kinds of errors that can occur
/// when parsing WGSL shaders, generating Rust bindings, or performing other operations
/// in `wgsl_bindgen`.
#[derive(Debug, Error, Diagnostic)]
pub enum WgslBindgenError {
  #[error("All required fields need to be set upfront")]
  OptionBuilderError(#[from] WgslBindgenOptionBuilderError),
  #[error(transparent)]
  #[diagnostic(transparent)]
  DependencyTreeError(#[from] DependencyTreeError),
  #[error("'{entry}': {inner}")]
  NagaModuleComposeError {
    entry: String,
    inner: naga_oil::compose::ComposerError,
  },
  #[error(transparent)]
  ModuleCreationError(#[from] CreateModuleError),
  #[error(transparent)]
  WriteOutputError(#[from] std::io::Error),
}

#[derive(Default, Builder)]
#[builder(
  setter(into),
  field(private),
  build_fn(private, name = "fallible_build")
)]
pub struct WgslBindgenOption {
  /// A vector of entry points to be added. Each entry point is represented as a `String`.
  #[builder(setter(each(name = "add_entry_point", into)))]
  entry_points: Vec<String>,

  /// The root prefix if any applied to all shaders given as the entrypoints.
  #[builder(default, setter(strip_option, into))]
  module_import_root: Option<String>,

  /// A boolean flag indicating whether to emit a rerun-if-changed directive to Cargo. Defaults to `true`.
  #[builder(default = "true")]
  emit_rerun_if_change: bool,

  /// A boolean flag indicating whether to skip header comments. Enabling headers allows to not rerun if contents did not change.
  #[builder(default = "false")]
  skip_header_comments: bool,

  /// A boolean flag indicating whether to skip the hash check. This will avoid reruns of bindings generation if
  /// entry shaders including their imports has not changed. Defaults to `false`.
  #[builder(default = "false")]
  skip_hash_check: bool,

  /// Derive [encase::ShaderType](https://docs.rs/encase/latest/encase/trait.ShaderType.html#)
  /// for user defined WGSL structs when `WgslTypeSerializeStrategy::Encase`.
  /// else derive bytemuck
  #[builder(default)]
  serialization_strategy: WgslTypeSerializeStrategy,

  /// Derive [serde::Serialize](https://docs.rs/serde/1.0.159/serde/trait.Serialize.html)
  /// and [serde::Deserialize](https://docs.rs/serde/1.0.159/serde/trait.Deserialize.html)
  /// for user defined WGSL structs when `true`.
  #[builder(default = "false")]
  derive_serde: bool,

  /// A mapping operation for WGSL built-in types. This is used to map WGSL built-in types to their corresponding representations.
  #[builder(default, setter(into))]
  wgsl_type_map: Box<dyn WgslTypeMap + 'static>,
}

impl WgslBindgenOptionBuilder {
  pub fn build(&self) -> Result<WGSLBindgen, WgslBindgenError> {
    let options = self.fallible_build()?;
    WGSLBindgen::new(options)
  }
}

pub struct WGSLBindgen {
  dependency_tree: DependencyTree,
  options: WgslBindgenOption,
  generate_options: WriteOptions,
  content_hash: String,
}

impl WGSLBindgen {
  fn new(options: WgslBindgenOption) -> Result<Self, WgslBindgenError> {
    let dependency_tree = DependencyTree::try_build(
      options.module_import_root.clone(),
      options
        .entry_points
        .iter()
        .cloned()
        .map(SourceFilePath::new)
        .collect(),
    )?;

    let content_hash = Self::get_contents_hash(&dependency_tree);

    if options.emit_rerun_if_change {
      for file in Self::iter_files_to_watch(&dependency_tree) {
        println!("cargo:rerun-if-changed={}", file);
      }
    }

    let generate_options = WriteOptions {
      serialization_strategy: options.serialization_strategy,
      derive_serde: options.derive_serde,
      wgsl_type_map: options.wgsl_type_map.clone(),
    };

    Ok(Self {
      dependency_tree,
      options,
      content_hash,
      generate_options,
    })
  }

  fn iter_files_to_watch(dep_tree: &DependencyTree) -> impl Iterator<Item = String> {
    dep_tree
      .all_files_including_dependencies()
      .into_iter()
      .map(|path| path.to_string())
  }

  fn get_contents_hash(dep_tree: &DependencyTree) -> String {
    let mut hasher = blake3::Hasher::new();

    hasher.update(PKG_VER.as_bytes());

    for SourceFile { content, .. } in dep_tree.parsed_files() {
      hasher.update(content.as_bytes());
    }

    hasher.finalize().to_string()
  }

  fn generate_naga_module_for_entry(
    entry: SourceWithFullDependenciesResult<'_>,
  ) -> Result<(String, naga::Module), WgslBindgenError> {
    let map_err = |err| WgslBindgenError::NagaModuleComposeError {
      entry: entry.source_file.file_path.to_string(),
      inner: err,
    };

    let mut composer = Composer::default();
    let source = entry.source_file;

    for dependency in entry.full_dependencies {
      composer
        .add_composable_module(ComposableModuleDescriptor {
          source: &dependency.content,
          file_path: &dependency.file_path.to_string(),
          language: ShaderLanguage::Wgsl,
          as_name: dependency.module_name.as_ref().map(|name| name.to_string()),
          ..Default::default()
        })
        .map_err(map_err)?;
    }

    let module = composer
      .make_naga_module(NagaModuleDescriptor {
        source: &source.content,
        file_path: &source.file_path.to_string(),
        ..Default::default()
      })
      .map_err(map_err)?;

    Ok((source.file_path.file_prefix(), module))
  }

  pub fn generate_string(&self) -> Result<String, WgslBindgenError> {
    use std::fmt::Write;
    let naga_modules = self
      .dependency_tree
      .get_source_files_with_full_dependencies()
      .into_iter()
      .map(Self::generate_naga_module_for_entry)
      .collect::<Result<Vec<_>, _>>()?;

    let mut text = String::new();

    if !self.options.skip_header_comments {
      writeln!(&mut text, "// File automatically generated by {PKG_NAME}^").unwrap();
      writeln!(&mut text, "//").unwrap();
      writeln!(&mut text, "// ^ {PKG_NAME} version {PKG_VER}",).unwrap();
      writeln!(&mut text, "// Changes made to this file will not be saved.").unwrap();
      writeln!(&mut text, "// SourceHash: {}", self.content_hash).unwrap();
      writeln!(&mut text).unwrap();
    }

    let output = create_rust_bindings(naga_modules, &self.generate_options)?;
    text += &output;

    Ok(text)
  }

  pub fn generate(&self, output_file: impl AsRef<Path>) -> Result<(), WgslBindgenError> {
    let output_path = output_file.as_ref();

    let old_content =
      std::fs::read_to_string(output_path).unwrap_or_else(|_| String::new());

    let old_hashstr_comment = old_content
      .lines()
      .find(|line| line.starts_with("// SourceHash:"))
      .unwrap_or("");

    let is_hash_changed =
      || old_hashstr_comment != format!("// SourceHash: {}", &self.content_hash);

    if self.options.skip_hash_check || is_hash_changed() {
      let content = self.generate_string()?;
      std::fs::File::create(output_path)?.write_all(content.as_bytes())?
    }

    Ok(())
  }
}
