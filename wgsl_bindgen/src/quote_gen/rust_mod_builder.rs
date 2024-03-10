#![allow(unused)]

use miette::Diagnostic;
use proc_macro2::TokenStream;
use quote::quote;
use smallvec::SmallVec;
use syn::Ident;
use thiserror::Error;

use super::constants::MOD_REFERENCE_ROOT;
use super::RustSourceItem;
use crate::quote_gen::constants::mod_reference_root;
use crate::FastIndexMap;

#[derive(Debug, Error, Diagnostic)]
pub enum RustModBuilderError {
  #[error("Different Content for unique id {id}")]
  DuplicateContentError {
    id: String,
    existing: String,
    received: String,
  },
}

#[derive(Default)]
struct RustMod {
  name: String,
  is_public: bool,
  module_attributes: TokenStream,
  initial_contents: TokenStream,
  content: Vec<TokenStream>,
  unique_content: FastIndexMap<String, usize>,
  submodules: FastIndexMap<String, RustMod>,
}

impl RustMod {
  fn new(name: &str, is_public_visibility: bool, initial_contents: TokenStream) -> Self {
    Self {
      module_attributes: quote!(),
      name: name.to_owned(),
      is_public: is_public_visibility,
      initial_contents,
      content: Vec::new(),
      unique_content: FastIndexMap::default(),
      submodules: FastIndexMap::default(),
    }
  }

  fn add_content(&mut self, content: TokenStream) {
    self.content.push(content);
  }

  fn add_unique(
    &mut self,
    id: &str,
    content: TokenStream,
  ) -> Result<(), RustModBuilderError> {
    if let Some(existing_content) = self
      .unique_content
      .get(id)
      .and_then(|&index| self.content.get(index))
    {
      let existing = existing_content.to_string();
      let received = content.to_string();

      if existing != received {
        return Err(RustModBuilderError::DuplicateContentError {
          id: id.to_string(),
          existing,
          received,
        });
      }
    } else {
      self
        .unique_content
        .insert(id.to_string(), self.content.len());
      self.content.push(content);
    }

    Ok(())
  }

  fn get_or_create_submodule(&mut self, name: &str) -> &mut RustMod {
    self
      .submodules
      .entry(name.to_owned())
      .or_insert_with(|| RustMod::new(name, true, self.initial_contents.clone()))
  }

  fn merge(&mut self, other: Self) {
    self.content.extend(other.content);
    self.unique_content.extend(other.unique_content);
    for (name, other_submodule) in other.submodules {
      let self_submodule = self.get_or_create_submodule(&name);
      self_submodule.merge(other_submodule);
    }
  }

  fn generate(&self) -> TokenStream {
    let name = Ident::new(&self.name, proc_macro2::Span::call_site());

    let initial_contents = &self.initial_contents;
    let content = &self.content;

    let visibility = if self.is_public {
      quote!(pub)
    } else {
      quote!()
    };

    let submodules = self
      .submodules
      .values()
      .map(|m| m.generate())
      .collect::<Vec<_>>();

    let mod_attr = &self.module_attributes;

    quote! {
      #mod_attr
      #visibility mod #name {
          #initial_contents
          #( #content )*
          #( #submodules )*
      }
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RustModBuilderConfig {
  use_relative_root: bool,
}

impl RustModBuilderConfig {
  fn build_module(&self, mod_name: &str) -> RustMod {
    if self.use_relative_root {
      // this helps import relative items for nested mods under this root
      // https://discord.com/channels/442252698964721669/448238009733742612/1207323647203868712
      let root = mod_reference_root();
      if mod_name == MOD_REFERENCE_ROOT {
        RustMod {
          name: mod_name.into(),
          is_public: false,
          module_attributes: quote!(),
          initial_contents: quote! {pub use super::*;},
          ..Default::default()
        }
      } else {
        RustMod {
          name: mod_name.into(),
          is_public: true,
          module_attributes: quote!(),
          initial_contents: quote! {
            use super::{#root, #root::*};
          },
          ..Default::default()
        }
      }
    } else {
      RustMod::new(mod_name, true, quote!())
    }
  }

  fn initial_modules(&self) -> FastIndexMap<String, RustMod> {
    if self.use_relative_root {
      let name = MOD_REFERENCE_ROOT.to_owned();
      let root_mod = self.build_module(name.as_str());
      FastIndexMap::from_iter([(name, root_mod)])
    } else {
      Default::default()
    }
  }
}

pub(crate) struct RustModBuilder {
  modules: FastIndexMap<String, RustMod>,
  config: RustModBuilderConfig,
}

impl RustModBuilder {
  pub fn new(use_relative_root: bool) -> Self {
    let config = RustModBuilderConfig { use_relative_root };

    Self {
      modules: config.initial_modules(),
      config,
    }
  }

  fn get_or_create_module(&mut self, path: &str) -> &mut RustMod {
    if path.is_empty() {
      panic!("path cannot be empty");
    }

    let modules = path.split("::").collect::<SmallVec<[_; 8]>>();

    let mut current_module = self
      .modules
      .entry(modules[0].to_owned())
      .or_insert_with(|| self.config.build_module(modules[0]));

    for name in &modules[1..] {
      current_module = current_module.get_or_create_submodule(name);
    }
    current_module
  }

  pub fn add_items(
    &mut self,
    items: Vec<RustSourceItem>,
  ) -> Result<(), RustModBuilderError> {
    for item in items {
      let module_path = item.path.parent_module_path;
      let name = item.path.item_name;

      self.add_unique(module_path.as_str(), &name, item.item)?;
    }

    Ok(())
  }

  pub fn add(&mut self, path: &str, content: TokenStream) {
    self.get_or_create_module(path).add_content(content);
  }

  pub fn add_unique(
    &mut self,
    path: &str,
    id: &str,
    content: TokenStream,
  ) -> Result<(), RustModBuilderError> {
    self.get_or_create_module(path).add_unique(id, content)
  }

  pub fn merge(mut self, other: Self) -> Self {
    assert_eq!(self.config, other.config);
    for (name, other_module) in other.modules {
      let self_module = self.get_or_create_module(&name);
      self_module.merge(other_module);
    }
    self
  }

  /// Generates the top level root module that includes other modules
  pub fn generate(&self) -> TokenStream {
    let modules: Vec<TokenStream> = self.modules.values().map(|m| m.generate()).collect();
    quote! {
      #( #modules )*
    }
  }
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;
  use quote::quote;

  use super::{RustModBuilder, RustModBuilderError};
  use crate::assert_tokens_eq;

  #[test]
  fn test_module_generation_works() {
    let mut mod_builder = RustModBuilder::new(false);
    mod_builder.add("a::b::c::d", quote! {struct A;});
    mod_builder.add("a::b::c", quote! {struct B;});
    mod_builder.add("a::b::c", quote! {struct C;});

    let actual = mod_builder.generate();

    assert_tokens_eq!(
      actual,
      quote! {
        pub mod a {
          pub mod b {
            pub mod c {
              struct B ;
              struct C ;
              pub mod d {
                struct A ;
              }
            }
          }
        }
      }
    );
  }

  #[test]
  fn test_relative_root_feature() {
    let mut mod_builder = RustModBuilder::new(true);
    mod_builder.add("a::b", quote! {struct A;});
    mod_builder.add(
      "a",
      quote! {struct B{
        a: a::b::A
      }},
    );

    let actual = mod_builder.generate();

    assert_tokens_eq!(
      actual,
      quote! {
        mod _root {
          pub use super::*;
        }
        pub mod a {
          use super::{_root, _root::*};
          struct B {
              a: a::b::A,
          }
          pub mod b {
              use super::{_root, _root::*};
              struct A;
          }
        }
      }
    );
  }

  #[test]
  fn test_module_add_duplicates() -> Result<(), RustModBuilderError> {
    let mut mod_builder = RustModBuilder::new(false);
    mod_builder.add_unique("a::b", "A", quote! {struct A;})?;
    mod_builder.add_unique("a", "A", quote! {struct B;})?;
    mod_builder.add_unique("a::b", "A", quote! {struct A;})?;

    let tokens = mod_builder.generate();

    assert_tokens_eq!(
      tokens,
      quote! {
        pub mod a {
          struct B;
          pub mod b {
            struct A ;
          }
        }
      }
    );
    Ok(())
  }

  #[test]
  fn test_module_add_duplicates_different_contents() {
    let mut mod_builder = RustModBuilder::new(false);
    mod_builder
      .add_unique("a::b", "A", quote! {struct A;})
      .unwrap();

    let error = mod_builder.add_unique("a::b", "A", quote! {struct B;});

    assert_eq!(error.is_err(), true);
  }

  #[test]
  fn test_merge() {
    let mut builder1 = RustModBuilder::new(false);
    builder1.add("a::b::c", quote! {struct A;});
    builder1.add("a::b::d", quote! {struct B;});

    let mut builder2 = RustModBuilder::new(false);
    builder2.add("a::b::c", quote! {struct C;});
    builder2.add("a::b::e", quote! {struct D;});

    let actual = builder1.merge(builder2).generate();

    assert_tokens_eq!(
      actual,
      quote! {
        pub mod a {
          pub mod b {
            pub mod c {
              struct A;
              struct C;
            }
            pub mod d {
              struct B;
            }
            pub mod e {
              struct D;
            }
          }
        }
      }
    );
  }
}
