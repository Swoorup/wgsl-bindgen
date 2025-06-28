//! Utility functions for common token generation patterns
//!
//! This module contains helper functions to reduce duplication in quote! macro usage
//! and make token generation more readable and maintainable.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

/// Generates a derive attribute with the specified traits
pub fn generate_derive_attributes(traits: &[&str]) -> TokenStream {
  if traits.is_empty() {
    return quote!();
  }

  let trait_idents: Vec<TokenStream> = traits
    .iter()
    .map(|trait_name| {
      // Handle fully qualified trait names like "encase::ShaderType" or "serde::Serialize"
      if trait_name.contains("::") {
        syn::parse_str::<TokenStream>(trait_name).unwrap()
      } else {
        let ident = format_ident!("{}", trait_name);
        quote!(#ident)
      }
    })
    .collect();

  quote! {
      #[derive(#(#trait_idents),*)]
  }
}

/// Generates a basic struct definition with optional derives and visibility
pub fn generate_struct_definition(
  visibility: TokenStream,
  struct_name: &Ident,
  derives: &[&str],
  repr: Option<TokenStream>,
  fields: &[TokenStream],
) -> TokenStream {
  let derive_attr = generate_derive_attributes(derives);
  let repr_attr = repr.unwrap_or_else(|| quote!());

  quote! {
      #derive_attr
      #repr_attr
      #visibility struct #struct_name {
          #(#fields),*
      }
  }
}

/// Generates a constructor function for a struct
pub fn generate_constructor_function(
  visibility: TokenStream,
  function_name: &Ident,
  struct_name: &Ident,
  parameters: &[TokenStream],
  field_assignments: &[TokenStream],
) -> TokenStream {
  quote! {
      #visibility const fn #function_name(#(#parameters),*) -> #struct_name {
          #struct_name {
              #(#field_assignments),*
          }
      }
  }
}

/// Generates an impl block with methods
pub fn generate_impl_block(
  struct_name: &Ident,
  generic_params: Option<TokenStream>,
  methods: &[TokenStream],
) -> TokenStream {
  match generic_params {
    Some(generics) => quote! {
        impl #generics #struct_name #generics {
            #(#methods)*
        }
    },
    None => quote! {
        impl #struct_name {
            #(#methods)*
        }
    },
  }
}

/// Generates a public constant definition
pub fn generate_public_const(
  const_name: &Ident,
  const_type: TokenStream,
  const_value: TokenStream,
) -> TokenStream {
  quote! {
      pub const #const_name: #const_type = #const_value;
  }
}

/// Generates a documentation comment attribute
pub fn generate_doc_comment(doc_text: &str) -> TokenStream {
  quote! {
      #[doc = #doc_text]
  }
}

/// Generates a field with optional documentation
pub fn generate_struct_field(
  field_name: &Ident,
  field_type: TokenStream,
  doc_comment: Option<&str>,
  visibility: Option<TokenStream>,
) -> TokenStream {
  let vis = visibility.unwrap_or_else(|| quote!(pub));

  if let Some(doc) = doc_comment {
    let doc_attr = generate_doc_comment(doc);
    quote! {
        #doc_attr
        #vis #field_name: #field_type
    }
  } else {
    quote! {
        #vis #field_name: #field_type
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use quote::format_ident;

  #[test]
  fn test_generate_derive_attributes() {
    let result = generate_derive_attributes(&["Debug", "Clone", "PartialEq"]);
    let expected = quote! {
        #[derive(Debug, Clone, PartialEq)]
    };
    assert_eq!(result.to_string(), expected.to_string());
  }

  #[test]
  fn test_generate_struct_definition() {
    let name = format_ident!("TestStruct");
    let field = quote! { pub field: u32 };
    let result = generate_struct_definition(
      quote!(pub),
      &name,
      &["Debug"],
      Some(quote!(#[repr(C)])),
      &[field],
    );

    let expected = quote! {
        #[derive(Debug)]
        #[repr(C)]
        pub struct TestStruct {
            pub field: u32
        }
    };
    assert_eq!(result.to_string(), expected.to_string());
  }
}
