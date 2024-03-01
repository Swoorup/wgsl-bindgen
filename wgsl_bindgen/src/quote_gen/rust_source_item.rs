use proc_macro2::TokenStream;

use crate::bevy_util::demangle_splitting_mod_path_and_item;

/// Represents a Rust source item.
pub(crate) struct RustSourceItem {
  /// If not present this item belongs at the source root
  pub mod_path: Option<String>,
  pub name: String,
  pub item: TokenStream,
}

impl RustSourceItem {
  pub fn from_mangled(name: &str, item: TokenStream) -> Self {
    let (mod_path, name) = demangle_splitting_mod_path_and_item(name);

    Self {
      mod_path,
      name,
      item,
    }
  }
}
