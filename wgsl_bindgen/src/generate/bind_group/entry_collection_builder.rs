use derive_more::Constructor;

use self::quote_gen::RustItemPath;
use super::*;

#[derive(Constructor)]
pub(super) struct BindGroupEntryCollectionBuilder<'a> {
  invoking_entry_module: &'a str,
  group_no: u32,
  data: &'a GroupData<'a>,
  generator: &'a BindGroupLayoutGenerator,
}

impl<'a> BindGroupEntryCollectionBuilder<'a> {
  /// Generates a binding entry from a parameter variable and a group binding.
  fn create_entry_from_parameter(
    &self,
    binding_var_name: &Ident,
    binding: &GroupBinding,
  ) -> TokenStream {
    let entry_cons = self.generator.entry_constructor;
    let binding_index = binding.binding_index as usize;
    let demangled_name = RustItemPath::from_mangled(
      binding.name.as_ref().unwrap(),
      self.invoking_entry_module,
    );
    let binding_name = Ident::new(&demangled_name.item_name, Span::call_site());
    let binding_var = quote!(#binding_var_name.#binding_name);

    match binding.binding_type.inner {
      naga::TypeInner::Scalar(_)
      | naga::TypeInner::Struct { .. }
      | naga::TypeInner::Array { .. } => {
        entry_cons(binding_index, binding_var, BindResourceType::Buffer)
      }
      naga::TypeInner::Image { .. } => {
        entry_cons(binding_index, binding_var, BindResourceType::Texture)
      }
      naga::TypeInner::Sampler { .. } => {
        entry_cons(binding_index, binding_var, BindResourceType::Sampler)
      }
      // TODO: Better error handling.
      _ => panic!("Failed to generate BindingType."),
    }
  }

  /// Assigns entries for the bind group from the provided parameters.
  fn assign_entries_from_parameters(&self, param_var_name: Ident) -> Vec<TokenStream> {
    self
      .data
      .bindings
      .iter()
      .map(|binding| {
        let demangled_name = RustItemPath::from_mangled(
          binding.name.as_ref().unwrap(),
          self.invoking_entry_module,
        );
        let binding_name = Ident::new(&demangled_name.item_name, Span::call_site());
        let create_entry = self.create_entry_from_parameter(&param_var_name, binding);

        quote! {
          #binding_name: #create_entry
        }
      })
      .collect()
  }

  /// Generates a tuple of parameter field and entry field for a binding.
  fn binding_field_tuple(&self, binding: &GroupBinding) -> (TokenStream, TokenStream) {
    let rust_item_path = RustItemPath::from_mangled(
      binding.name.as_ref().unwrap(),
      self.invoking_entry_module,
    );
    let field_name = format_ident!("{}", &rust_item_path.item_name.as_str());

    // TODO: Support more types.
    let resource_type = match binding.binding_type.inner {
      naga::TypeInner::Struct { .. } => BindResourceType::Buffer,
      naga::TypeInner::Image { .. } => BindResourceType::Texture,
      naga::TypeInner::Sampler { .. } => BindResourceType::Sampler,
      naga::TypeInner::Array { .. } => BindResourceType::Buffer,
      naga::TypeInner::Scalar(_) => BindResourceType::Buffer,
      _ => panic!("Unsupported type for binding fields."),
    };

    let param_field_type = self.generator.binding_type_map[&resource_type].clone();
    let field_type = self.generator.entry_struct_type.clone();

    let param_field = quote!(pub #field_name: #param_field_type);
    let entry_field = quote!(pub #field_name: #field_type);

    (param_field, entry_field)
  }

  fn all_entries(&self, binding_var_name: Ident) -> Vec<TokenStream> {
    self
      .data
      .bindings
      .iter()
      .map(|binding| {
        let demangled_name = RustItemPath::from_mangled(
          binding.name.as_ref().unwrap(),
          self.invoking_entry_module,
        );
        let binding_name = Ident::new(&demangled_name.item_name, Span::call_site());
        quote! (#binding_var_name.#binding_name)
      })
      .collect()
  }

  pub(super) fn build(&self) -> TokenStream {
    let (entries_param_fields, entries_fields): (Vec<_>, Vec<_>) = self
      .data
      .bindings
      .iter()
      .map(|binding| self.binding_field_tuple(binding))
      .collect();

    let entry_collection_name = self
      .generator
      .bind_group_entry_collection_struct_name_ident(self.group_no);
    let entry_collection_param_name = format_ident!(
      "{}Params",
      self
        .generator
        .bind_group_entry_collection_struct_name_ident(self.group_no)
    );
    let entry_struct_type = self.generator.entry_struct_type.clone();

    let lifetime = if self.generator.uses_lifetime {
      quote!(<'a>)
    } else {
      quote!()
    };

    let entries_from_params =
      self.assign_entries_from_parameters(format_ident!("params"));
    let entries_length = Index::from(entries_from_params.len() as usize);
    let all_entries = self.all_entries(format_ident!("self"));

    quote! {
        #[derive(Debug)]
        pub struct #entry_collection_param_name #lifetime {
            #(#entries_param_fields),*
        }

        #[derive(Clone, Debug)]
        pub struct #entry_collection_name #lifetime {
            #(#entries_fields),*
        }

        impl #lifetime #entry_collection_name #lifetime {
          pub fn new(params: #entry_collection_param_name #lifetime) -> Self {
            Self {
              #(#entries_from_params),*
            }
          }

          pub fn entries(self) -> [#entry_struct_type; #entries_length] {
            [ #(#all_entries),* ]
          }
        }
    }
  }
}
