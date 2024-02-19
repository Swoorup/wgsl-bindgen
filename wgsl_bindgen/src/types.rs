use std::path::PathBuf;

use derivative::Derivative;
use derive_more::{AsRef, Deref, Display, From, Into};
use fxhash::FxBuildHasher;
use indexmap::{IndexMap, IndexSet};
use smol_str::SmolStr;

pub type FxIndexMap<K, V> = IndexMap<K, V, FxBuildHasher>;
pub type FxIndexSet<T> = IndexSet<T, FxBuildHasher>;

#[derive(AsRef, Hash, From, Into, Clone, PartialEq, Eq, Derivative, Deref, Display)]
#[display(fmt = "{}", "_0.to_str().unwrap()")]
#[derivative(Debug = "transparent")]
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
    self.0.file_prefix().unwrap().to_str().unwrap().to_string()
  }
}

#[derive(AsRef, Hash, From, Into, Clone, PartialEq, Eq, Derivative, Deref, Display)]
#[display(fmt = "{}", "_0.to_str().unwrap()")]
#[derivative(Debug = "transparent")]
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

#[derive(AsRef, Hash, From, Into, Clone, PartialEq, Eq, Derivative, Deref, Display)]
#[display(fmt = "{}", "_0")]
#[derivative(Debug = "transparent")]
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
