use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::bevy_util::demangle_splitting_mod_path_and_item;
use crate::quote_gen::RustSourceItem;

pub fn consts_items(module: &naga::Module) -> Vec<RustSourceItem> {
  // Create matching Rust constants for WGSl constants.
  module
    .constants
    .iter()
    .filter_map(|(_, t)| -> Option<RustSourceItem> {
      let name_str = t.name.as_ref()?;

      // we don't need full qualification here
      let (_, demangled_name) = demangle_splitting_mod_path_and_item(name_str);

      let name = Ident::new(&demangled_name, Span::call_site());

      // TODO: Add support for f64 and f16 once naga supports them.
      let type_and_value = match &module.const_expressions[t.init] {
        naga::Expression::Literal(literal) => match literal {
          naga::Literal::F64(v) => Some(quote!(f32 = #v)),
          naga::Literal::F32(v) => Some(quote!(f32 = #v)),
          naga::Literal::U32(v) => Some(quote!(u32 = #v)),
          naga::Literal::I32(v) => Some(quote!(i32 = #v)),
          naga::Literal::Bool(v) => Some(quote!(bool = #v)),
          naga::Literal::I64(v) => Some(quote!(i64 = #v)),
          naga::Literal::AbstractInt(v) => Some(quote!(i64 = #v)),
          naga::Literal::AbstractFloat(v) => Some(quote!(f64 = #v)),
        },
        _ => None,
      }?;

      Some(RustSourceItem::from_mangled(
        &name_str,
        quote! { pub const #name: #type_and_value;},
      ))
    })
    .collect()
}

#[allow(unused)]
pub fn consts(module: &naga::Module) -> Vec<TokenStream> {
  consts_items(module).into_iter().map(|i| i.item).collect()
}

#[cfg(test)]
mod tests {
  use indoc::indoc;

  use super::*;
  use crate::assert_tokens_eq;

  #[test]
  fn write_global_consts() {
    let source = indoc! {r#"
            const INT_CONST = 12;
            const UNSIGNED_CONST = 34u;
            const FLOAT_CONST = 0.1;
            // TODO: Naga doesn't implement f16, even though it's in the WGSL spec
            // const SMALL_FLOAT_CONST:f16 = 0.1h;
            const BOOL_CONST = true;

            @fragment
            fn main() {
                // TODO: This is valid WGSL syntax, but naga doesn't support it apparently.
                // const C_INNER = 456;
            }
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let consts = consts(&module);
    let actual = quote!(#(#consts)*);
    eprintln!("{actual}");

    assert_tokens_eq!(
      quote! {
          pub const INT_CONST: i32 = 12i32;
          pub const UNSIGNED_CONST: u32 = 34u32;
          pub const FLOAT_CONST: f32 = 0.1f32;
          pub const BOOL_CONST: bool = true;
      },
      actual
    );
  }
}
