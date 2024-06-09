#![allow(dead_code)]
use derive_more::Constructor;
use proc_macro2::TokenStream;
use smol_str::SmolStr;

/// `RustItemPath` represents the path to a Rust item within a module.
#[derive(Constructor, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RustItemPath {
  /// The path to the parent module.
  pub parent_module_path: SmolStr,
  /// name of the item, without the module path.
  pub item_name: SmolStr,
}

impl RustItemPath {
  pub fn get_fully_qualified_name(&self) -> SmolStr {
    if self.parent_module_path.is_empty() {
      SmolStr::new(self.item_name.as_str())
    } else {
      SmolStr::new(format!("{}::{}", self.parent_module_path, self.item_name).as_str())
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RustItemKind {
  ConstVarDecl,
  TraitImpls,
  Any,
}

/// Represents a Rust source item, that is either a ConstVar, TraitImpls or others.
#[derive(Constructor)]
pub(crate) struct RustItem {
  pub kind: RustItemKind,
  pub path: RustItemPath,
  pub item: TokenStream,
}
