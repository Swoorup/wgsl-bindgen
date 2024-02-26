use std::io::Write;
use std::path::PathBuf;

use derive_builder::Builder;
use miette::Diagnostic;
use naga_oil::compose::{
  ComposableModuleDescriptor, Composer, ComposerError, NagaModuleDescriptor,
  ShaderLanguage,
};
use thiserror::Error;

use self::source_file::SourceFile;
use crate::{
  bevy_util::*, WgslEntryResult, WgslTypeMap, WgslTypeMapBuild, WgslTypeSerializeStrategy,
};
use crate::{create_rust_bindings, CreateModuleError, SourceFilePath};
pub use naga::valid::Capabilities as WgslShaderIRCapabilities;

const PKG_VER: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

/// Enum representing the possible errors that can occur in the `wgsl_bindgen` process.
///
/// This enum is used to represent all the different kinds of errors that can occur
/// when parsing WGSL shaders, generating Rust bindings, or performing other operations
/// in `wgsl_bindgen`.
#[derive(Debug, Error, Diagnostic)]
pub enum WgslBindgenError {
  #[error("All required fields need to be set upfront: {0}")]
  OptionBuilderError(#[from] WgslBindgenOptionBuilderError),

  #[error(transparent)]
  #[diagnostic(transparent)]
  DependencyTreeError(#[from] DependencyTreeError),

  #[error("Failed to compose modules with entry `{entry}`\n{msg}")]
  NagaModuleComposeError {
    entry: String,
    msg: String,
    inner: naga_oil::compose::ComposerErrorInner,
  },

  #[error(transparent)]
  ModuleCreationError(#[from] CreateModuleError),

  #[error(transparent)]
  WriteOutputError(#[from] std::io::Error),

  #[error("Output file is not specified. Maybe use `generate_string` instead")]
  OutputFileNotSpecified,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgslShaderSourceOutputType {
  /// Include the final shader string directly in the output
  #[default]
  FinalShaderString,

  /// Use Composer including helper functions which will be executed on runtime
  Composer,
}

/// A struct representing a directory to scan for additional source files.
///
/// This struct is used to represent a directory to scan for additional source files
/// when generating Rust bindings for WGSL shaders. The `module_import_root` field
/// is used to specify the root prefix or namespace that should be applied to all
/// shaders given as the entrypoints, and the `directory` field is used to specify
/// the directory to scan for additional source files.
#[derive(Debug, Clone, Default)]
pub struct AdditionalScanDirectory {
  pub module_import_root: Option<String>,
  pub directory: String,
}

impl From<(Option<&str>, &str)> for AdditionalScanDirectory {
  fn from((module_import_root, directory): (Option<&str>, &str)) -> Self {
    Self {
      module_import_root: module_import_root.map(ToString::to_string),
      directory: directory.to_string(),
    }
  }
}

#[derive(Debug, Default, Builder)]
#[builder(
  setter(into),
  field(private),
  build_fn(private, name = "fallible_build")
)]
pub struct WgslBindgenOption {
  /// A vector of entry points to be added. Each entry point is represented as a `String`.
  #[builder(setter(each(name = "add_entry_point", into)))]
  pub entry_points: Vec<String>,

  /// The root prefix/namespace if any applied to all shaders given as the entrypoints.
  #[builder(default, setter(strip_option, into))]
  pub module_import_root: Option<String>,

  /// A boolean flag indicating whether to emit a rerun-if-changed directive to Cargo. Defaults to `true`.
  #[builder(default = "true")]
  pub emit_rerun_if_change: bool,

  /// A boolean flag indicating whether to skip header comments. Enabling headers allows to not rerun if contents did not change.
  #[builder(default = "false")]
  pub skip_header_comments: bool,

  /// A boolean flag indicating whether to skip the hash check. This will avoid reruns of bindings generation if
  /// entry shaders including their imports has not changed. Defaults to `false`.
  #[builder(default = "false")]
  pub skip_hash_check: bool,

  /// Derive [encase::ShaderType](https://docs.rs/encase/latest/encase/trait.ShaderType.html#)
  /// for user defined WGSL structs when `WgslTypeSerializeStrategy::Encase`.
  /// else derive bytemuck
  #[builder(default)]
  pub serialization_strategy: WgslTypeSerializeStrategy,

  /// Derive [serde::Serialize](https://docs.rs/serde/1.0.159/serde/trait.Serialize.html)
  /// and [serde::Deserialize](https://docs.rs/serde/1.0.159/serde/trait.Deserialize.html)
  /// for user defined WGSL structs when `true`.
  #[builder(default = "false")]
  pub derive_serde: bool,

  /// The type of output for the shader source. Defaults to `FinalShaderString`.
  #[builder(default)]
  pub shader_source_output_type: WgslShaderSourceOutputType,

  /// A mapping operation for WGSL built-in types. This is used to map WGSL built-in types to their corresponding representations.
  #[builder(setter(custom))]
  pub wgsl_type_map: WgslTypeMap,

  /// The output file path for the generated Rust bindings. Defaults to `None`.
  #[builder(default, setter(strip_option, into))]
  pub output_file: Option<PathBuf>,

  /// The additional set of directories to scan for source files.
  #[builder(default, setter(into, each(name = "additional_scan_dir", into)))]
  pub additional_scan_dirs: Vec<AdditionalScanDirectory>,

  /// The capabilities of naga to support. Defaults to `None`.
  #[builder(default, setter(strip_option))]
  pub ir_capabilities: Option<WgslShaderIRCapabilities>,
}

impl WgslBindgenOptionBuilder {
  pub fn build(&self) -> Result<WGSLBindgen, WgslBindgenError> {
    let options = self.fallible_build()?;
    WGSLBindgen::new(options)
  }

  pub fn wgsl_type_map(&mut self, map_build: impl WgslTypeMapBuild) -> &mut Self {
    let serialization_strategy = self
      .serialization_strategy
      .expect("Serialization strategy must be set before `wgs_type_map`");

    self.wgsl_type_map = Some(map_build.build(serialization_strategy));
    self
  }
}

pub struct WGSLBindgen {
  dependency_tree: DependencyTree,
  options: WgslBindgenOption,
  content_hash: String,
}

impl WGSLBindgen {
  fn new(options: WgslBindgenOption) -> Result<Self, WgslBindgenError> {
    let entry_points = options
      .entry_points
      .iter()
      .cloned()
      .map(SourceFilePath::new)
      .collect();

    let dependency_tree = DependencyTree::try_build(
      options.module_import_root.clone(),
      entry_points,
      options.additional_scan_dirs.clone(),
    )?;

    let content_hash = Self::get_contents_hash(&options, &dependency_tree);

    if options.emit_rerun_if_change {
      for file in Self::iter_files_to_watch(&dependency_tree) {
        println!("cargo:rerun-if-changed={}", file);
      }
    }

    Ok(Self {
      dependency_tree,
      options,
      content_hash,
    })
  }

  fn iter_files_to_watch(dep_tree: &DependencyTree) -> impl Iterator<Item = String> {
    dep_tree
      .all_files_including_dependencies()
      .into_iter()
      .map(|path| path.to_string())
  }

  fn get_contents_hash(options: &WgslBindgenOption, dep_tree: &DependencyTree) -> String {
    let mut hasher = blake3::Hasher::new();

    hasher.update(format!("{:?}", options).as_bytes());
    hasher.update(PKG_VER.as_bytes());

    for SourceFile { content, .. } in dep_tree.parsed_files() {
      hasher.update(content.as_bytes());
    }

    hasher.finalize().to_string()
  }

  fn generate_naga_module_for_entry(
    ir_capabilities: Option<WgslShaderIRCapabilities>,
    entry: SourceWithFullDependenciesResult<'_>,
  ) -> Result<WgslEntryResult, WgslBindgenError> {
    let map_err = |composer: &Composer, err: ComposerError| {
      let msg = err.emit_to_string(composer);
      WgslBindgenError::NagaModuleComposeError {
        entry: entry.source_file.file_path.to_string(),
        inner: err.inner,
        msg,
      }
    };

    let mut composer = match ir_capabilities {
      Some(ir_capabilities) => Composer::default().with_capabilities(ir_capabilities),
      _ => Composer::default(),
    };
    let source = entry.source_file;

    for dependency in entry.full_dependencies.iter() {
      composer
        .add_composable_module(ComposableModuleDescriptor {
          source: &dependency.content,
          file_path: &dependency.file_path.to_string(),
          language: ShaderLanguage::Wgsl,
          as_name: dependency.module_name.as_ref().map(|name| name.to_string()),
          ..Default::default()
        })
        .map(|_| ())
        .map_err(|err| map_err(&composer, err))?;
    }

    let module = composer
      .make_naga_module(NagaModuleDescriptor {
        source: &source.content,
        file_path: &source.file_path.to_string(),
        ..Default::default()
      })
      .map_err(|err| map_err(&composer, err))?;

    Ok(WgslEntryResult {
      mod_name: source.file_path.file_prefix(),
      naga_module: module,
      source_including_deps: entry,
    })
  }

  pub fn generate_string(&self) -> Result<String, WgslBindgenError> {
    use std::fmt::Write;
    let ir_capabilities = self.options.ir_capabilities;
    let entry_results = self
      .dependency_tree
      .get_source_files_with_full_dependencies()
      .into_iter()
      .map(|it| Self::generate_naga_module_for_entry(ir_capabilities, it))
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

    let output = create_rust_bindings(entry_results, &self.options)?;
    text += &output;

    Ok(text)
  }

  pub fn generate(&self) -> Result<(), WgslBindgenError> {
    let output_path = self
      .options
      .output_file
      .as_ref()
      .ok_or(WgslBindgenError::OutputFileNotSpecified)?;

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
