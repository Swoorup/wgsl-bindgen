mod constants;
mod rust_item;
mod rust_mod_builder;
mod rust_struct_builder;
mod rust_type_info;

pub(crate) use constants::*;
use proc_macro2::TokenStream;
pub(crate) use rust_item::*;
pub(crate) use rust_mod_builder::*;
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
///
/// # Returns
///
/// The demangled and qualified token stream.
pub(crate) fn demangle_and_qualify(string: &str) -> TokenStream {
  let demangled = demangle_str(string);

  match demangled.contains("::") {
    true => {
      let fully_qualified = format!("{}::{}", MOD_REFERENCE_ROOT, demangled);
      syn::parse_str(&fully_qualified).unwrap()
    }
    false => syn::parse_str(&demangled).unwrap(),
  }
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;

  use super::demangle_and_qualify;

  #[test]
  fn should_fully_qualify_mangled_string() {
    let string = "UniformsX_naga_oil_mod_XOR4XAZLTX";
    let actual = demangle_and_qualify(string);
    assert_eq!(actual.to_string(), "_root :: types :: Uniforms");
  }

  #[test]
  fn should_not_fully_qualify_non_mangled_string() {
    let string = "MatricesF64";
    let actual = demangle_and_qualify(string);
    assert_eq!(actual.to_string(), "MatricesF64");
  }
}
