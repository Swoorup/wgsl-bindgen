#![allow(unused)]

use enumflags2::{BitFlag, BitFlags};
use miette::Diagnostic;
use proc_macro2::TokenStream;
use quote::quote;
use smallvec::SmallVec;
use syn::Ident;
use thiserror::Error;

use super::constants::MOD_REFERENCE_ROOT;
use super::{RustSourceItem, RustSourceItemCategory};
use crate::quote_gen::constants::mod_reference_root;
use crate::{pretty_print, FastIndexMap};

#[derive(Debug, Error, Diagnostic)]
pub enum RustModuleBuilderError {
  #[error("Different Content for unique id {id}, \nExisting: \n{existing}\n\nReceived: \n{received}")]
  DuplicateContentError {
    id: String,
    existing: String,
    received: String,
  },
}

struct UniqueItemInfo {
  index: usize,
  types: BitFlags<RustSourceItemCategory>,
}

#[derive(Default)]
struct RustModule {
  name: String,
  is_public: bool,
  module_attributes: TokenStream,
  initial_contents: TokenStream,
  content: Vec<TokenStream>,
  unique_content_info: FastIndexMap<String, UniqueItemInfo>,
  submodules: FastIndexMap<String, RustModule>,
}

impl RustModule {
  fn new(name: &str, is_public_visibility: bool, initial_contents: TokenStream) -> Self {
    Self {
      module_attributes: quote!(),
      name: name.to_owned(),
      is_public: is_public_visibility,
      initial_contents,
      content: Vec::new(),
      unique_content_info: FastIndexMap::default(),
      submodules: FastIndexMap::default(),
    }
  }

  fn add_content(&mut self, content: TokenStream) {
    self.content.push(content);
  }

  /// Adds unique content to the `RustModule`.
  ///
  /// This function checks if the provided `id` already exists in the `unique_content_info` map.
  /// If it does, it compares the existing content with the new content.
  /// If the content is the same, it does nothing.
  /// If the content is different but the `RustItemKind` is the same, it appends the new content to the existing content.
  /// If the `RustItemKind` is different, it returns an error.
  fn add_unique(
    &mut self,
    id: &str,
    types: BitFlags<RustSourceItemCategory>,
    content: TokenStream,
  ) -> Result<(), RustModuleBuilderError> {
    if let Some((previous_info, existing_content)) =
      self.unique_content_info.get_mut(id).and_then(|info| {
        let content = self.content.get_mut(info.index)?;
        Some((info, content))
      })
    {
      if previous_info.types == types {
        let existing = existing_content.to_string();
        let received = content.to_string();
        if existing != received {
          return Err(RustModuleBuilderError::DuplicateContentError {
            id: id.to_string(),
            existing: pretty_print(&existing_content),
            received: pretty_print(&content),
          });
        }
      } else {
        existing_content.extend(content);
        previous_info.types |= types;
      }
    } else {
      self.unique_content_info.insert(
        id.to_string(),
        UniqueItemInfo {
          index: self.content.len(),
          types,
        },
      );
      self.content.push(content);
    }

    Ok(())
  }

  fn get_or_create_submodule(&mut self, name: &str) -> &mut RustModule {
    self
      .submodules
      .entry(name.to_owned())
      .or_insert_with(|| RustModule::new(name, true, self.initial_contents.clone()))
  }

  fn merge(&mut self, other: Self) {
    self.content.extend(other.content);
    self.unique_content_info.extend(other.unique_content_info);
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
  generate_relative_root: bool,
}

impl RustModBuilderConfig {
  fn build_module(&self, mod_name: &str) -> RustModule {
    if self.use_relative_root {
      // this helps import relative items for nested mods under this root
      // https://discord.com/channels/442252698964721669/448238009733742612/1207323647203868712
      let root = mod_reference_root();
      if mod_name == MOD_REFERENCE_ROOT {
        RustModule {
          name: mod_name.into(),
          is_public: false,
          module_attributes: quote!(),
          initial_contents: quote! {pub use super::*;},
          ..Default::default()
        }
      } else {
        RustModule {
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
      RustModule::new(mod_name, true, quote!())
    }
  }

  fn initial_modules(&self) -> FastIndexMap<String, RustModule> {
    if self.use_relative_root && self.generate_relative_root {
      let name = MOD_REFERENCE_ROOT.to_owned();
      let root_mod = self.build_module(name.as_str());
      FastIndexMap::from_iter([(name, root_mod)])
    } else {
      Default::default()
    }
  }
}

pub(crate) struct RustModBuilder {
  modules: FastIndexMap<String, RustModule>,
  config: RustModBuilderConfig,
}

impl RustModBuilder {
  pub fn new(use_relative_root: bool, generate_relative_root: bool) -> Self {
    let config = RustModBuilderConfig {
      use_relative_root,
      generate_relative_root,
    };

    Self {
      modules: config.initial_modules(),
      config,
    }
  }

  fn get_or_create_module(&mut self, path: &str) -> &mut RustModule {
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
  ) -> Result<(), RustModuleBuilderError> {
    for item in items {
      let module_path = item.path.module;
      let name = item.path.name;

      let mut m = self.get_or_create_module(&module_path);
      m.add_unique(&name, item.catagories, item.tokenstream)?;
    }

    Ok(())
  }

  pub fn add(&mut self, path: &str, content: TokenStream) {
    self.get_or_create_module(path).add_content(content);
  }

  fn add_unique(
    &mut self,
    path: &str,
    id: &str,
    content: TokenStream,
  ) -> Result<(), RustModuleBuilderError> {
    self
      .get_or_create_module(path)
      .add_unique(id, RustSourceItemCategory::all(), content)
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

  use super::{RustModBuilder, RustModuleBuilderError};
  use crate::assert_tokens_snapshot;

  #[test]
  fn test_module_generation_works() {
    let mut mod_builder = RustModBuilder::new(false, false);
    mod_builder.add("a::b::c::d", quote! {struct A;});
    mod_builder.add("a::b::c", quote! {struct B;});
    mod_builder.add("a::b::c", quote! {struct C;});

    let actual = mod_builder.generate();

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn test_relative_root_feature() {
    let mut mod_builder = RustModBuilder::new(true, true);
    mod_builder.add("a::b", quote! {struct A;});
    mod_builder.add(
      "a",
      quote! {struct B{
        a: a::b::A
      }},
    );

    let actual = mod_builder.generate();

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn test_include_relative_root_but_dont_generate_it() {
    let mut mod_builder = RustModBuilder::new(true, false);
    mod_builder.add("a::b", quote! {struct A;});
    mod_builder.add(
      "a",
      quote! {struct B{
        a: a::b::A
      }},
    );

    let actual = mod_builder.generate();

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn test_module_add_duplicates() -> Result<(), RustModuleBuilderError> {
    let mut mod_builder = RustModBuilder::new(false, false);
    mod_builder.add_unique("a::b", "A", quote! {struct A;})?;
    mod_builder.add_unique("a", "A", quote! {struct B;})?;
    mod_builder.add_unique("a::b", "A", quote! {struct A;})?;

    let tokens = mod_builder.generate();

    assert_tokens_snapshot!(tokens);
    Ok(())
  }

  #[test]
  fn test_module_add_duplicates_different_contents() {
    let mut mod_builder = RustModBuilder::new(false, false);
    mod_builder
      .add_unique("a::b", "A", quote! {struct A;})
      .unwrap();

    let error = mod_builder.add_unique("a::b", "A", quote! {struct B;});

    assert_eq!(error.is_err(), true);
  }

  #[test]
  fn test_merge() {
    let mut builder1 = RustModBuilder::new(false, false);
    builder1.add("a::b::c", quote! {struct A;});
    builder1.add("a::b::d", quote! {struct B;});

    let mut builder2 = RustModBuilder::new(false, false);
    builder2.add("a::b::c", quote! {struct C;});
    builder2.add("a::b::e", quote! {struct D;});

    let actual = builder1.merge(builder2).generate();

    assert_tokens_snapshot!(actual);
  }
}
