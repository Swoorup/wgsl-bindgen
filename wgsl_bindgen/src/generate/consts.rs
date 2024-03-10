use proc_macro2::Span;
use quote::quote;
use syn::Ident;

use crate::quote_gen::{RustItemPath, RustItem};

pub fn consts_items(
  invoking_entry_module: &str,
  module: &naga::Module,
) -> Vec<RustItem> {
  // Create matching Rust constants for WGSl constants.
  module
    .constants
    .iter()
    .filter_map(|(_, t)| -> Option<RustItem> {
      let name_str = t.name.as_ref()?;

      // we don't need full qualification here
      let rust_item_path = RustItemPath::from_mangled(name_str, invoking_entry_module);
      let name = Ident::new(&rust_item_path.item_name, Span::call_site());

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

      Some(RustItem::new(
        rust_item_path,
        quote! { pub const #name: #type_and_value;},
      ))
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use indoc::indoc;
  use proc_macro2::TokenStream;

  use super::*;
  use crate::assert_tokens_eq;

  pub fn consts(module: &naga::Module) -> Vec<TokenStream> {
    consts_items("", module)
      .into_iter()
      .map(|i| i.item)
      .collect()
  }

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
