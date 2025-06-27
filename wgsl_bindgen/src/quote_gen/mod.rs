mod constants;
mod rust_module_builder;
mod rust_source_item;
mod rust_struct_builder;
mod rust_type_info;

use core::panic;

pub(crate) use constants::*;
use proc_macro2::TokenStream;
pub(crate) use rust_module_builder::*;
pub(crate) use rust_source_item::*;
pub(crate) use rust_struct_builder::*;
pub(crate) use rust_type_info::*;

use crate::bevy_util::demangle_str;

/// Creates a raw string literal from the given shader content.
///
/// # Arguments
///
/// * `shader_content` - The content of the shader as a string.
///
/// # Returns
///
/// The token stream representing the raw string literal.
pub(crate) fn create_shader_raw_string_literal(shader_content: &str) -> TokenStream {
  syn::parse_str::<TokenStream>(&format!("r#\"\n{}\"#", &shader_content)).unwrap()
}

/// Demangles the given string and qualifies it with the qualification root.
///
/// # Arguments
///
/// * `string` - The string to demangle and qualify.
/// * `default_mod_path` - The default module path to use if the string does not contain a module path.
///
/// # Returns
///
/// The demangled and qualified token stream.
pub(crate) fn demangle_and_fully_qualify_str(
  string: &str,
  default_mod_path: Option<&str>,
) -> String {
  let demangled = demangle_str(string);

  match (demangled.contains("::"), default_mod_path) {
    (true, _) => {
      format!("{MOD_REFERENCE_ROOT}::{demangled}")
    }
    (false, None) => demangled.to_string(),
    (false, Some(default_mod_path)) => {
      if default_mod_path.is_empty() {
        panic!("default_mod_path cannot be empty");
      }

      let default_mod_path = default_mod_path.to_lowercase();
      format!("{MOD_REFERENCE_ROOT}::{default_mod_path}::{demangled}")
    }
  }
}

pub(crate) fn demangle_and_fully_qualify(
  string: &str,
  default_mod_path: Option<&str>,
) -> TokenStream {
  let raw_path = demangle_and_fully_qualify_str(string, default_mod_path);
  syn::parse_str(&raw_path).unwrap()
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;

  use super::demangle_and_fully_qualify;

  #[test]
  fn should_fully_qualify_mangled_string() {
    let string = "UniformsX_naga_oil_mod_XOR4XAZLTX";
    let actual = demangle_and_fully_qualify(string, None);
    assert_eq!(actual.to_string(), "_root :: types :: Uniforms");
  }

  #[test]
  fn should_not_fully_qualify_non_mangled_string() {
    let string = "MatricesF64";
    let actual = demangle_and_fully_qualify(string, None);
    assert_eq!(actual.to_string(), "MatricesF64");
  }
}
