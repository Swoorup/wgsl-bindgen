use miette::Diagnostic;
use thiserror::Error;

use crate::bevy_util::DependencyTreeError;
use crate::{CreateModuleError, WgslBindgenOptionBuilderError};

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
