use derive_more::Constructor;

use self::quote_gen::RustItemPath;
use super::*;

#[derive(Constructor)]
pub(super) struct BindGroupLayoutBuilder<'a> {
  invoking_entry_module: &'a str,
  group_no: u32,
  data: &'a GroupData<'a>,
  generator: &'a BindGroupLayoutGenerator,
}

impl<'a> BindGroupLayoutBuilder<'a> {
  fn entries(&self, binding_var_name: Ident) -> Vec<TokenStream> {
    let entry_cons = self.generator.entry_constructor;

    self
      .data
      .bindings
      .iter()
      .map(|binding| {
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
      })
      .collect()
  }

  pub(super) fn build(&self) -> TokenStream {
    let fields: Vec<_> = self
      .data
      .bindings
      .iter()
      .map(|binding| {
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

        let field_type = self.generator.binding_type_map[&resource_type].clone();

        quote!(pub #field_name: #field_type)
      })
      .collect();

    let name = indexed_name_ident(&self.generator.layout_prefix_name, self.group_no);
    let entries = self.entries(format_ident!("self"));
    let entries_length = Index::from(entries.len() as usize);
    let entry_struct_type = self.generator.entry_struct_type.clone();

    let lifetime = if self.generator.uses_lifetime {
      quote!(<'a>)
    } else {
      quote!()
    };

    quote! {
        #[derive(Debug)]
        pub struct #name #lifetime {
            #(#fields),*
        }

        impl #lifetime #name #lifetime {

          pub fn entries(self) -> [#entry_struct_type; #entries_length] {
            [ #(#entries),* ]
          }
        }
    }
  }
}
