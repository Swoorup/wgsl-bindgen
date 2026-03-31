use std::path::PathBuf;

use derive_more::{AsRef, Deref, Display, From, Into};
use educe::Educe;
use fxhash::FxBuildHasher;
use indexmap::{IndexMap, IndexSet};
use smol_str::SmolStr;

pub type FxIndexMap<K, V> = IndexMap<K, V, FxBuildHasher>;
pub type FxIndexSet<T> = IndexSet<T, FxBuildHasher>;

#[derive(AsRef, Hash, From, Into, Clone, PartialEq, Eq, Educe, Deref, Display)]
#[display("{}", _0.to_str().unwrap())]
#[educe(Debug(name = false, named_field = false))]
pub struct SourceFilePath(PathBuf);

impl SourceFilePath {
  pub fn new(value: impl Into<PathBuf>) -> Self {
    Self(value.into())
  }

  pub fn read_contents(&self) -> Result<String, std::io::Error> {
    std::fs::read_to_string(self.as_path())
  }

  pub fn dir(&self) -> SourceFileDir {
    SourceFileDir(self.parent().unwrap().into())
  }

  pub fn file_prefix(&self) -> String {
    // file_prefix is only available in nightly
    let file_name = self.0.file_stem().unwrap().to_str().unwrap();
    let prefix = file_name.split('.').next().unwrap_or("");
    prefix.to_string()
  }

  pub fn module_path(&self, workspace_root: &std::path::Path) -> String {
    Self::path_to_module_path(&self.0, workspace_root)
  }

  /// Converts a file path to a Rust module path relative to a workspace root.
  ///
  /// Examples:
  /// - `"shaders/lines/segment.wgsl"` with workspace `"shaders"` -> `"lines::segment"`
  /// - `"particle_physics.wgsl"` with workspace `"."` -> `"particle_physics"`
  pub fn path_to_module_path(
    file_path: &std::path::Path,
    workspace_root: &std::path::Path,
  ) -> String {
    // Get the relative path from workspace root
    let relative_path = file_path.strip_prefix(workspace_root).unwrap_or(file_path);

    // Get parent directories and file stem
    let parent = relative_path.parent();
    let file_stem = relative_path.file_stem().unwrap().to_str().unwrap();
    let file_prefix = file_stem.split('.').next().unwrap_or("");

    // Build module path with :: separators
    match parent {
      Some(parent) if !parent.as_os_str().is_empty() => {
        let parent_modules: Vec<&str> =
          parent.iter().filter_map(|s| s.to_str()).collect();
        format!("{}::{}", parent_modules.join("::"), file_prefix)
      }
      _ => file_prefix.to_string(),
    }
  }
}

#[derive(AsRef, Hash, From, Into, Clone, PartialEq, Eq, Educe, Deref, Display)]
#[display("{}", _0.to_str().unwrap())]
#[educe(Debug(name = false, named_field = false))]
pub struct SourceFileDir(PathBuf);

impl SourceFileDir {
  pub fn new(value: impl Into<PathBuf>) -> Self {
    Self(value.into())
  }

  pub fn read_contents(&self) -> Result<String, std::io::Error> {
    std::fs::read_to_string(self.as_path())
  }
}

impl From<&SourceFilePath> for SourceFileDir {
  fn from(value: &SourceFilePath) -> Self {
    value.dir()
  }
}

/// Import part path used in the import statement
#[derive(AsRef, Hash, From, Into, Clone, PartialEq, Eq, Educe, Deref, Display)]
#[display("{}", _0)]
#[educe(Debug(name = false, named_field = false))]
pub struct ImportPathPart(SmolStr);

impl ImportPathPart {
  pub fn new(value: impl Into<SmolStr>) -> Self {
    Self(value.into())
  }
}

#[derive(AsRef, Hash, From, Into, Clone, PartialEq, Eq, Educe, Deref, Display)]
#[display("{}", _0)]
#[educe(Debug(name = false, named_field = false))]
pub struct SourceModuleName(SmolStr);

impl SourceModuleName {
  pub fn new(value: impl Into<SmolStr>) -> Self {
    Self(value.into())
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
  /// 1-based line number.
  pub line_number: usize,
  /// 1-based column of the start of this span
  pub line_position: usize,
  /// 0-based Offset in code units (in bytes) of the start of the span.
  pub offset: usize,
  /// Length in code units (in bytes) of the span.
  pub length: usize,
}

impl From<&SourceLocation> for miette::SourceSpan {
  fn from(value: &SourceLocation) -> miette::SourceSpan {
    miette::SourceSpan::new(value.offset.into(), value.length)
  }
}

/// Value for a shader preprocessor definition used in conditional compilation.
///
/// This mirrors the naga-oil `ShaderDefValue` type but is defined in wgsl_bindgen
/// so that users building with only WESL (`EmbedWithWesl`) do not need to add
/// naga-oil to their build dependencies.
///
/// When using [`crate::WgslShaderSourceType::EmbedWithWesl`] only `Bool` values
/// are consumed by the WESL compiler; `Int` and `UInt` values are silently
/// ignored because WESL's conditional translation only supports boolean feature
/// flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderDefValue {
  /// A boolean shader definition.
  Bool(bool),
  /// A signed 32-bit integer shader definition.
  ///
  /// Note: not supported by WESL conditional translation.
  Int(i32),
  /// An unsigned 32-bit integer shader definition.
  ///
  /// Note: not supported by WESL conditional translation.
  UInt(u32),
}

impl From<ShaderDefValue> for naga_oil::compose::ShaderDefValue {
  fn from(v: ShaderDefValue) -> Self {
    match v {
      ShaderDefValue::Bool(b) => Self::Bool(b),
      ShaderDefValue::Int(i) => Self::Int(i),
      ShaderDefValue::UInt(u) => Self::UInt(u),
    }
  }
}

impl From<naga_oil::compose::ShaderDefValue> for ShaderDefValue {
  fn from(v: naga_oil::compose::ShaderDefValue) -> Self {
    match v {
      naga_oil::compose::ShaderDefValue::Bool(b) => Self::Bool(b),
      naga_oil::compose::ShaderDefValue::Int(i) => Self::Int(i),
      naga_oil::compose::ShaderDefValue::UInt(u) => Self::UInt(u),
    }
  }
}
