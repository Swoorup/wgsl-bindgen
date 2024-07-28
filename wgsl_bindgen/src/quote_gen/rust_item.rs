#![allow(dead_code)]
use derive_more::Constructor;
use enumflags2::{bitflags, BitFlags};
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use smol_str::SmolStr;

/// `RustItemPath` represents the path to a Rust item within a module.
#[derive(Constructor, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RustItemPath {
  /// The path to the parent module.
  pub module: SmolStr,
  /// name of the item, without the module path.
  pub name: SmolStr,
}

impl ToTokens for RustItemPath {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let fq_name = self.get_fully_qualified_name();
    let current = syn::parse_str::<TokenStream>(&fq_name).unwrap();
    tokens.extend(current)
  }
}

impl RustItemPath {
  pub fn get_fully_qualified_name(&self) -> SmolStr {
    if self.module.is_empty() {
      SmolStr::new(self.name.as_str())
    } else {
      SmolStr::new(format!("{}::{}", self.module, self.name).as_str())
    }
  }

  /// Returns a shortened `TokenStream`,
  /// If the module of the item is the same as given `target_module`, it will return just the `name` part of the path.
  /// Otherwise, it will return the full path.
  pub fn short_token_stream(&self, target_module: &str) -> TokenStream {
    if self.module == target_module {
      let ident = syn::Ident::new(&self.name, Span::call_site());
      quote::quote!(#ident)
    } else {
      self.to_token_stream()
    }
  }
}

#[bitflags]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum RustItemType {
  /// like `const VAR_NAME: Type = value;`
  ConstVarDecls,

  /// like `impl Trait for Struct {}`
  TraitImpls,

  /// like `impl Struct {}`
  TypeImpls,

  /// like `struct Struct {}`
  TypeDefs,
}

/// Represents a Rust source item, that is either a ConstVar, TraitImpls or others.
#[derive(Constructor)]
pub(crate) struct RustItem {
  pub types: BitFlags<RustItemType>,
  pub path: RustItemPath,
  pub item: TokenStream,
}
