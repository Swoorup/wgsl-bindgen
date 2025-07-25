use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::quote_gen::{
  rust_type, RustSourceItem, RustSourceItemCategory, RustSourceItemPath,
};
use crate::WgslBindgenOption;

pub fn consts_items(
  invoking_entry_module: &str,
  module: &naga::Module,
) -> Vec<RustSourceItem> {
  // Create matching Rust constants for WGSl constants.
  module
    .constants
    .iter()
    .filter_map(|(_, t)| -> Option<RustSourceItem> {
      let name_str = t.name.as_ref()?;

      // we don't need full qualification here
      let rust_item_path =
        RustSourceItemPath::from_mangled(name_str, invoking_entry_module);
      let name = Ident::new(&rust_item_path.name, Span::call_site());

      // TODO: Add support for f64 and f16 once naga supports them.
      let type_and_value = match &module.global_expressions[t.init] {
        naga::Expression::Literal(literal) => match literal {
          naga::Literal::F64(v) => Some(quote!(f32 = #v)),
          naga::Literal::F32(v) => Some(quote!(f32 = #v)),
          naga::Literal::U32(v) => Some(quote!(u32 = #v)),
          naga::Literal::U64(v) => Some(quote!(u64 = #v)),
          naga::Literal::I32(v) => Some(quote!(i32 = #v)),
          naga::Literal::Bool(v) => Some(quote!(bool = #v)),
          naga::Literal::I64(v) => Some(quote!(i64 = #v)),
          naga::Literal::AbstractInt(v) => Some(quote!(i64 = #v)),
          naga::Literal::AbstractFloat(v) => Some(quote!(f64 = #v)),
          naga::Literal::F16(v) => {
            let v = v.to_f32();
            Some(quote!(half::f16 = half::f16::from_f32_const(#v)))
          }
        },
        _ => None,
      }?;

      Some(RustSourceItem::new(
        RustSourceItemCategory::ConstVarDecls.into(),
        rust_item_path,
        quote! { pub const #name: #type_and_value;},
      ))
    })
    .collect()
}

pub fn pipeline_overridable_constants(
  module: &naga::Module,
  options: &WgslBindgenOption,
) -> TokenStream {
  let overrides: Vec<_> = module.overrides.iter().map(|(_, o)| o).collect();

  let fields: Vec<_> = overrides
    .iter()
    .map(|o| {
      let name = Ident::new(o.name.as_ref().unwrap(), Span::call_site());
      // TODO: Do we only need to handle scalar types here?
      let ty = rust_type(None, module, &module.types[o.ty], options);

      if o.init.is_some() {
        quote!(pub #name: Option<#ty>)
      } else {
        quote!(pub #name: #ty)
      }
    })
    .collect();

  let required_entries: Vec<_> = overrides
      .iter()
      .filter_map(|o| {
          if o.init.is_some() {
              None
          } else {
              let key = override_key(o);

              let name = Ident::new(o.name.as_ref().unwrap(), Span::call_site());

              // TODO: Do we only need to handle scalar types here?
              let ty = &module.types[o.ty];
              let value = if matches!(ty.inner, naga::TypeInner::Scalar(s) if s.kind == naga::ScalarKind::Bool) {
                  quote!(if self.#name { 1.0 } else { 0.0})
              } else {
                  quote!(self.#name as f64)
              };

              Some(quote!((#key.to_owned(), #value)))
          }
      })
      .collect();

  // Add code for optionally inserting the constants with defaults.
  // Omitted constants will be initialized using the values defined in WGSL.
  let insert_optional_entries: Vec<_> = overrides
      .iter()
      .filter_map(|o| {
          if o.init.is_some() {
              let key = override_key(o);

              // TODO: Do we only need to handle scalar types here?
              let ty = &module.types[o.ty];
              let value = if matches!(ty.inner, naga::TypeInner::Scalar(s) if s.kind == naga::ScalarKind::Bool) {
                  quote!(if value { 1.0 } else { 0.0})
              } else {
                  quote!(value as f64)
              };

              let name = Ident::new(o.name.as_ref().unwrap(), Span::call_site());

              Some(quote! {
                  if let Some(value) = self.#name {
                      entries.insert(#key.to_owned(), #value);
                  }
              })
          } else {
              None
          }
      })
      .collect();

  let init_entries = if insert_optional_entries.is_empty() {
    quote!(let entries = vec![#(#required_entries),*];)
  } else {
    quote!(let mut entries = vec![#(#required_entries),*];)
  };

  if !fields.is_empty() {
    // Create a Rust struct that can initialize the constants dictionary.
    quote! {
        pub struct OverrideConstants {
            #(#fields),*
        }

        impl OverrideConstants {
            pub fn constants(&self) -> Vec<(&'static str, f64)> {
                #init_entries
                #(#insert_optional_entries);*
                entries
            }
        }
    }
  } else {
    quote!()
  }
}

fn override_key(o: &naga::Override) -> String {
  // The @id(id) should be the name if present.
  o.id
    .map(|i| i.to_string())
    .unwrap_or(o.name.clone().unwrap())
}

#[cfg(test)]
mod tests {
  use indoc::indoc;
  use proc_macro2::TokenStream;

  use super::*;
  use crate::assert_tokens_snapshot;

  fn consts(module: &naga::Module) -> Vec<TokenStream> {
    consts_items("", module)
      .into_iter()
      .map(|i| i.tokenstream)
      .collect()
  }
  #[test]
  fn write_global_constants() {
    let source = indoc! {r#"
            enable f16;
            const INT_CONST = 12;
            const UNSIGNED_CONST = 34u;
            const FLOAT_CONST = 0.1;
            const SMALL_FLOAT_CONST:f16 = 0.1h;
            const BOOL_CONST = true;

            @fragment
            fn main() -> @location(0) vec4<f32> {
                return vec4<f32>(
                    f32(INT_CONST),
                    f32(UNSIGNED_CONST),
                    FLOAT_CONST,
                    SMALL_FLOAT_CONST
                );
            }
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let consts = consts(&module);
    let actual = quote!(#(#consts)*);
    eprintln!("{actual}");
    // TODO: Why INT_CONST and UNSIGNED_CONST are not generated?

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_pipeline_overrideable_constants() {
    let source = indoc! {r#"
          override b1: bool = true;
          override b2: bool = false;
          override b3: bool;
          override f1: f32 = 0.5;
          override f2: f32;
          // override f3: f64 = 0.6;
          // override f4: f64;
          override i1: i32 = 0;
          override i2: i32;
          override i3: i32 = i1 * i2;
          @id(0) override a: f32 = 1.0;
          @id(35) override b: f32 = 2.0;
          @fragment
          fn main() {}
      "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let actual = pipeline_overridable_constants(&module, &WgslBindgenOption::default());
    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_pipeline_overrideable_constants_empty() {
    let source = indoc! {r#"
          @fragment
          fn main() {}
      "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = pipeline_overridable_constants(&module, &WgslBindgenOption::default());
    assert_tokens_snapshot!(actual);
  }
}
