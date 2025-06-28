use derive_more::IsVariant;
use naga::common::wgsl::TypeContext;
use naga::StructMember;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use smol_str::SmolStr;
use syn::{Ident, Index};

use super::{rust_type, RustSourceItem, RustSourceItemPath, RustTypeInfo};
use crate::bevy_util::demangle_str;
use crate::quote_gen::{
  generate_derive_attributes, generate_doc_comment, generate_impl_block,
  generate_struct_definition, generate_struct_field, RustSourceItemCategory,
  MOD_BYTEMUCK_IMPLS, MOD_STRUCT_ASSERTIONS,
};
use crate::{
  sanitized_upper_snake_case, WgslBindgenOption, WgslTypeSerializeStrategy,
  WgslTypeVisibility,
};

impl WgslTypeVisibility {
  fn generate_quote(&self) -> TokenStream {
    match self {
      WgslTypeVisibility::Public => quote!(pub),
      WgslTypeVisibility::RestrictedCrate => quote!(pub(crate)),
      WgslTypeVisibility::RestrictedSuper => quote!(pub(super)),
    }
  }
}

#[derive(Clone)]
pub struct Padding {
  pub pad_name: Ident,
  pub pad_size_tokens: TokenStream,
}

impl Padding {
  fn generate_member_instantiate(&self) -> TokenStream {
    let pad_name = &self.pad_name;
    let pad_size = &self.pad_size_tokens;
    quote!(#pad_name: [0; #pad_size])
  }

  fn generate_member_definition(&self) -> TokenStream {
    let pad_name = &self.pad_name;
    let pad_size = &self.pad_size_tokens;
    quote!(pub #pad_name: [u8; #pad_size])
  }
}

#[derive(Default)]
struct NagaToRustStructState<'a> {
  index: usize,
  members: Vec<RustStructMemberEntry<'a>>,
}

impl<'a> NagaToRustStructState<'a> {
  /// This replaces the `rust_type` with a custom field map if necessary
  fn get_rust_type(
    options: &WgslBindgenOption,
    fully_qualified_name: &SmolStr,
    rust_type: RustTypeInfo,
    member_name: &str,
  ) -> proc_macro2::TokenStream {
    let fully_qualified_name = fully_qualified_name.as_str();
    options
      .override_struct_field_type
      .iter()
      .find_map(|o| {
        let struct_matches = o.struct_regex.is_match(fully_qualified_name);
        let field_matches = o.field_regex.is_match(member_name);
        (struct_matches && field_matches).then_some(o.override_type.clone())
      })
      .unwrap_or(rust_type.tokens)
  }

  /// Creates a fold function for processing struct members into Rust equivalents
  fn create_fold(
    options: &'a WgslBindgenOption,
    fully_qualified_name: SmolStr,
    naga_members: &'a [StructMember],
    naga_module: &'a naga::Module,
    naga_context: naga::proc::GlobalCtx<'a>,
    layout_size: usize,
    is_directly_sharable: bool,
  ) -> impl FnMut(NagaToRustStructState<'a>, &'a StructMember) -> NagaToRustStructState<'a>
  {
    let member_processor = move |mut state: NagaToRustStructState<'a>,
                                 naga_member: &'a StructMember|
          -> NagaToRustStructState<'a> {
      let member_name = naga_member.name.as_ref().unwrap();
      let name_ident = Ident::new(member_name, Span::call_site());
      let member_naga_type = &naga_module.types[naga_member.ty];

      let rust_type_info = rust_type(None, naga_module, member_naga_type, options);
      let is_runtime_sized_array = rust_type_info.size.is_none();

      // Runtime-sized arrays can only be the last field in a struct
      if is_runtime_sized_array && state.index != naga_members.len() - 1 {
        panic!("Only the last field of a struct can be a runtime-sized array");
      }

      // Calculate padding needed between this field and the next
      let padding = if is_runtime_sized_array || !is_directly_sharable {
        None
      } else {
        let current_offset = naga_member.offset as usize;
        let next_offset = if state.index + 1 < naga_members.len() {
          naga_members[state.index + 1].offset as usize
        } else {
          layout_size
        };
        let rust_type = &rust_type_info;

        let pad_name = format!("_pad_{member_name}");
        let required_member_size = next_offset - current_offset;

        match rust_type.aligned_size() {
          Some(rust_type_size) if required_member_size == rust_type_size => None,
          _ => {
            let required_member_size = format!("0x{required_member_size:X}");
            let member_size =
              syn::parse_str::<TokenStream>(&required_member_size).unwrap();

            let pad_name = Ident::new(&pad_name, Span::call_site());
            let pad_size_tokens =
              quote!(#member_size - ::core::mem::size_of::<#rust_type>());

            let padding = Padding {
              pad_name,
              pad_size_tokens,
            };

            Some(padding)
          }
        }
      };

      let is_current_field_padding = options
        .custom_padding_field_regexps
        .iter()
        .any(|pad_expr| pad_expr.is_match(member_name));

      // Handle builtin and padding fields
      let is_builtin_field =
        matches!(naga_member.binding, Some(naga::Binding::BuiltIn(_)));

      // Skip builtin fields entirely for all serialization strategies
      // Builtin fields are GPU-provided values that are never part of user vertex buffer data
      if is_builtin_field {
        // Skip this member entirely - don't add to state.members
        state.index += 1;
        return state;
      }

      let entry = if is_current_field_padding {
        let size = member_naga_type.inner.size(naga_context);
        let size = format!("0x{size:X}");
        let pad_size_tokens = syn::parse_str::<TokenStream>(&size).unwrap();

        RustStructMemberEntry::Padding(Padding {
          pad_name: name_ident,
          pad_size_tokens,
        })
      } else {
        let rust_type = Self::get_rust_type(
          options,
          &fully_qualified_name,
          rust_type_info,
          member_name,
        );

        RustStructMemberEntry::Field(Field {
          name_ident: name_ident.clone(),
          naga_member,
          naga_type: member_naga_type,
          naga_ty_handle: naga_member.ty,
          rust_type: syn::Type::Verbatim(rust_type),
          is_rsa: is_runtime_sized_array,
        })
      };

      state.index += 1;
      state.members.push(entry);

      if let Some(padding) = padding {
        state.members.push(RustStructMemberEntry::Padding(padding));
      }
      state
    };

    member_processor
  }
}

pub struct Field<'a> {
  pub name_ident: Ident,
  pub naga_member: &'a naga::StructMember,
  pub naga_type: &'a naga::Type,
  pub naga_ty_handle: naga::Handle<naga::Type>,
  pub rust_type: syn::Type,
  pub is_rsa: bool,
}

impl<'a> Field<'a> {
  fn generate_member_instantiate(&self, other_struct_var_name: &Ident) -> TokenStream {
    let name = &self.name_ident;
    quote!(#name: #other_struct_var_name.#name)
  }

  fn generate_member_definition(&self) -> TokenStream {
    let name = &self.name_ident;
    let ty = &self.rust_type;
    quote!(pub #name: #ty)
  }

  fn generate_fn_new_param(&self) -> TokenStream {
    let name = &self.name_ident;
    let ty = &self.rust_type;
    quote!(#name: #ty)
  }
}

#[derive(IsVariant)]
pub enum RustStructMemberEntry<'a> {
  Field(Field<'a>),
  Padding(Padding),
}

impl<'a> RustStructMemberEntry<'a> {
  fn from_naga(
    options: &'a WgslBindgenOption,
    item_path: &'a RustSourceItemPath,
    naga_members: &'a [naga::StructMember],
    naga_module: &'a naga::Module,
    layout_size: usize,
    is_directly_sharable: bool,
  ) -> Vec<Self> {
    let naga_context = naga_module.to_ctx();
    let fully_qualified_name = item_path.get_fully_qualified_name();

    let state = naga_members.iter().fold(
      NagaToRustStructState::default(),
      NagaToRustStructState::create_fold(
        options,
        fully_qualified_name,
        naga_members,
        naga_module,
        naga_context,
        layout_size,
        is_directly_sharable,
      ),
    );
    state.members
  }
}

pub struct RustStructBuilder<'a> {
  item_path: &'a RustSourceItemPath,
  members: Vec<RustStructMemberEntry<'a>>,
  is_host_sharable: bool,
  has_rts_array: bool,
  naga_module: &'a naga::Module,
  layout: naga::proc::TypeLayout,
  options: &'a WgslBindgenOption,
}

impl<'a> RustStructBuilder<'a> {
  fn name_ident(&self) -> Ident {
    Ident::new(self.item_path.name.as_ref(), Span::call_site())
  }

  fn is_directly_shareable(&self) -> bool {
    self.options.serialization_strategy == WgslTypeSerializeStrategy::Bytemuck
      && self.is_host_sharable
  }

  fn uses_generics_for_rts(&self) -> bool {
    self.has_rts_array
      && self.options.serialization_strategy == WgslTypeSerializeStrategy::Bytemuck
  }

  fn uses_padding(&self) -> bool {
    self.members.iter().any(|m| m.is_padding())
  }

  fn ty_param_use(&self) -> TokenStream {
    if self.uses_generics_for_rts() {
      quote!(<N>)
    } else {
      quote!()
    }
  }

  fn ty_param_def(&self) -> TokenStream {
    if self.uses_generics_for_rts() {
      quote!(<const N: usize>)
    } else {
      quote!()
    }
  }

  fn struct_name_in_usage_fragment(&self) -> TokenStream {
    let ident = self.name_ident();
    let ty_param_use = self.ty_param_use();
    quote!(#ident #ty_param_use)
  }

  fn fully_qualified_struct_name_in_usage_fragment(&self) -> TokenStream {
    let fully_qualified_name_str = self.item_path.get_fully_qualified_name();
    let fully_qualified_name =
      syn::parse_str::<TokenStream>(&fully_qualified_name_str).unwrap();
    let ty_param_use = self.ty_param_use();
    quote!(#fully_qualified_name #ty_param_use)
  }

  fn struct_name_in_definition_fragment(&self) -> TokenStream {
    let ident = self.name_ident();
    let ty_param_def = self.ty_param_def();
    quote!(#ident #ty_param_def)
  }

  fn init_struct_name_in_usage_fragment(&self) -> TokenStream {
    let name = format!("{}Init", self.item_path.name);
    let ident = Ident::new(&name, Span::call_site());
    let ty_param_use = self.ty_param_use();
    quote!(#ident #ty_param_use)
  }

  fn init_struct_name_in_definition_fragment(&self) -> TokenStream {
    let name = format!("{}Init", self.item_path.name);
    let ident = Ident::new(&name, Span::call_site());
    let ty_param_def = self.ty_param_def();
    quote!(#ident #ty_param_def)
  }

  fn impl_trait_for_fragment(&self) -> TokenStream {
    let ty_param_def = self.ty_param_def();
    quote!(impl #ty_param_def)
  }

  fn build_init_struct(&self) -> TokenStream {
    if !self.is_directly_shareable()
      || (!self.uses_padding() && !self.options.always_generate_init_struct)
    {
      return quote!();
    }

    let impl_fragment = self.impl_trait_for_fragment();
    let struct_name_in_usage = self.struct_name_in_usage_fragment();
    let struct_name = self.name_ident();
    let init_struct_name_def = self.init_struct_name_in_definition_fragment();
    let init_struct_name_in_usage = self.init_struct_name_in_usage_fragment();
    let visibility = self.options.type_visibility.generate_quote();

    let mut init_struct_members = vec![];
    let mut mem_assignments = vec![];

    let init_var_name = Ident::new("self", Span::call_site());

    for entry in self.members.iter() {
      match entry {
        RustStructMemberEntry::Field(field) => {
          init_struct_members.push(field.generate_member_definition());
          mem_assignments.push(field.generate_member_instantiate(&init_var_name));
        }
        RustStructMemberEntry::Padding(padding) => {
          mem_assignments.push(padding.generate_member_instantiate())
        }
      }
    }

    let init_derives =
      generate_derive_attributes(&["Debug", "PartialEq", "Clone", "Copy"]);
    let build_method = quote! {
      pub const fn build(&self) -> #struct_name_in_usage {
        #struct_name {
          #(#mem_assignments),*
        }
      }
    };
    let from_impl = quote! {
      #impl_fragment From<#init_struct_name_in_usage> for #struct_name_in_usage {
        fn from(data: #init_struct_name_in_usage) -> Self {
          data.build()
        }
      }
    };

    quote! {
      #[repr(C)]
      #init_derives
      #visibility struct #init_struct_name_def {
        #(#init_struct_members),*
      }

      #impl_fragment #init_struct_name_in_usage {
        #build_method
      }

      #from_impl
    }
  }

  fn build_fn_new(&self) -> TokenStream {
    let struct_name_in_usage = self.struct_name_in_usage_fragment();
    let impl_fragment = self.impl_trait_for_fragment();

    let mut non_padding_members = Vec::new();
    let mut member_assignments = Vec::new();

    for entry in &self.members {
      match entry {
        RustStructMemberEntry::Field(field) => {
          let name = &field.name_ident;
          non_padding_members.push(field.generate_fn_new_param());
          member_assignments.push(quote!(#name));
        }
        RustStructMemberEntry::Padding(padding) => {
          member_assignments.push(padding.generate_member_instantiate())
        }
      }
    }

    match self.options.short_constructor {
      Some(max_param_length) if self.members.len() <= max_param_length as usize => {
        let struct_name = self.name_ident();
        let ty_param_def = self.ty_param_def();
        quote! {
          pub const fn #struct_name #ty_param_def(#(#non_padding_members),*) -> #struct_name_in_usage {
            #struct_name {
              #(#member_assignments),*
            }
          }
        }
      }
      _ => quote! {
        #impl_fragment #struct_name_in_usage {
          pub const fn new(
            #(#non_padding_members),*
          ) -> Self {
            Self {
              #(#member_assignments),*
            }
          }
        }
      },
    }
  }

  fn build_fields(&self) -> Vec<TokenStream> {
    let naga_context = self.naga_module.to_ctx();
    let members = self
      .members
      .iter()
      .map(|entry| match entry {
        RustStructMemberEntry::Field(field) => {
          let Field {
            name_ident: name,
            rust_type,
            is_rsa: is_rts,
            naga_member: member,
            naga_type,
            naga_ty_handle,
          } = field;

          let doc_comment = if self.is_directly_shareable() {
            let offset = member.offset;
            let size = naga_type.inner.size(naga_context);
            let ty_name = naga_context.type_to_string(*naga_ty_handle);
            let ty_name = demangle_str(&ty_name);
            let doc = format!(" size: {size}, offset: 0x{offset:X}, type: `{ty_name}`");

            generate_doc_comment(&doc)
          } else {
            quote!()
          };

          let runtime_size_attribute = if *is_rts
            && matches!(
              self.options.serialization_strategy,
              WgslTypeSerializeStrategy::Encase
            ) {
            quote!(#[size(runtime)])
          } else {
            quote!()
          };

          quote! {
            #doc_comment
            #runtime_size_attribute
            pub #name: #rust_type
          }
        }
        RustStructMemberEntry::Padding(padding) => padding.generate_member_definition(),
      })
      .collect::<Vec<_>>();

    members
  }

  fn build_derives(&self) -> Vec<&str> {
    let mut derives = vec!["Debug", "PartialEq", "Clone"];

    match self.options.serialization_strategy {
      WgslTypeSerializeStrategy::Bytemuck => {
        derives.push("Copy");
      }
      WgslTypeSerializeStrategy::Encase => {
        if !self.has_rts_array {
          derives.push("Copy");
        }
        derives.push("encase::ShaderType");
      }
    }
    if self.options.derive_serde {
      derives.push("serde::Serialize");
      derives.push("serde::Deserialize");
    }
    derives
  }

  fn build_layout_assertion(
    &self,
    custom_alignment: Option<naga::proc::Alignment>,
  ) -> TokenStream {
    let fully_qualified_name_str = self.item_path.get_fully_qualified_name();

    let fully_qualified_name =
      syn::parse_str::<TokenStream>(&fully_qualified_name_str).unwrap();
    let struct_name = if self.uses_generics_for_rts() {
      quote!(#fully_qualified_name<1>) // test RTS with 1 element
    } else {
      quote!(#fully_qualified_name)
    };

    let assert_member_offsets: Vec<_> = self
      .members
      .iter()
      .filter_map(|m| match m {
        RustStructMemberEntry::Field(field) => Some(field),
        RustStructMemberEntry::Padding(_) => None,
      })
      .map(|m| {
        let m = m.naga_member;
        let name = Ident::new(m.name.as_ref().unwrap(), Span::call_site());
        let rust_offset = quote!(std::mem::offset_of!(#struct_name, #name));
        let wgsl_offset = Index::from(m.offset as usize);
        quote!(assert!(#rust_offset == #wgsl_offset);)
      })
      .collect();

    if self.is_directly_shareable() {
      // Assert that the Rust layout matches the WGSL layout.
      // Enable for bytemuck since it uses the Rust struct's memory layout.
      let struct_size = custom_alignment
        .map(|alignment| alignment.round_up(self.layout.size))
        .unwrap_or(self.layout.size) as usize;

      let struct_size = Index::from(struct_size);

      let assertion_name = format_ident!(
        "{}_ASSERTS",
        sanitized_upper_snake_case(&fully_qualified_name_str)
      );

      quote! {
        const #assertion_name: () = {
          #(#assert_member_offsets)*
          assert!(std::mem::size_of::<#struct_name>() == #struct_size);
        };
      }
    } else {
      quote!()
    }
  }

  pub fn build_bytemuck_impls(&self) -> TokenStream {
    let struct_name_in_usage = self.fully_qualified_struct_name_in_usage_fragment();
    let impl_fragment = self.impl_trait_for_fragment();

    if self.options.serialization_strategy == WgslTypeSerializeStrategy::Bytemuck {
      quote! {
        unsafe #impl_fragment bytemuck::Zeroable for #struct_name_in_usage {}
        unsafe #impl_fragment bytemuck::Pod for #struct_name_in_usage {}
      }
    } else {
      quote!()
    }
  }

  pub fn build(&self) -> Vec<RustSourceItem> {
    let struct_name_def = self.struct_name_in_definition_fragment();

    // Assume types used in global variables are host shareable and require validation.
    // This includes storage, uniform, and workgroup variables.
    // This also means types that are never used will not be validated.
    // Structs used only for vertex inputs do not require validation on desktop platforms.
    // Vertex input layout is handled already by setting the attribute offsets and types.
    // This allows vertex input field types without padding like vec3 for positions.
    let is_host_shareable = self.is_host_sharable;

    let has_rts_array = self.has_rts_array;
    let should_generate_padding = is_host_shareable
      && self.options.serialization_strategy == WgslTypeSerializeStrategy::Bytemuck;

    let derives = self.build_derives();

    let fully_qualified_name = self.item_path.get_fully_qualified_name();
    let fully_qualified_name = fully_qualified_name.as_str();
    let custom_alignment = self
      .options
      .override_struct_alignment
      .iter()
      .find_map(|struct_align| {
        struct_align
          .struct_regex
          .is_match(fully_qualified_name)
          .then_some(struct_align.alignment as u32)
      })
      .and_then(naga::proc::Alignment::new);

    let alignment = custom_alignment.unwrap_or(self.layout.alignment) * 1u32;
    let alignment = Index::from(alignment as usize);
    let repr_c = if !has_rts_array {
      if should_generate_padding {
        Some(quote!(#[repr(C, align(#alignment))]))
      } else {
        Some(quote!(#[repr(C)]))
      }
    } else {
      None
    };

    let fields = self.build_fields();
    let struct_new_fn = self.build_fn_new();
    let init_struct = self.build_init_struct();
    let assert_layout = self.build_layout_assertion(custom_alignment);
    let unsafe_bytemuck_pod_impl = self.build_bytemuck_impls();
    let fully_qualified_name = self.item_path.get_fully_qualified_name();
    let visibility = self.options.type_visibility.generate_quote();

    // For now, keep the original complex struct definition due to generics handling
    let struct_name_def = self.struct_name_in_definition_fragment();
    let derive_attrs = generate_derive_attributes(&derives);
    let struct_definition = quote! {
      #repr_c
      #derive_attrs
      #visibility struct #struct_name_def {
          #(#fields),*
      }
    };

    vec![
      RustSourceItem::new(
        RustSourceItemCategory::TypeDefs | RustSourceItemCategory::TypeImpls,
        self.item_path.clone(),
        quote! {
          #struct_definition

          #struct_new_fn
          #init_struct
        },
      ),
      RustSourceItem::new(
        RustSourceItemCategory::ConstVarDecls.into(),
        RustSourceItemPath::new(
          MOD_STRUCT_ASSERTIONS.into(),
          fully_qualified_name.clone(),
        ),
        assert_layout,
      ),
      RustSourceItem::new(
        RustSourceItemCategory::TraitImpls.into(),
        RustSourceItemPath::new(MOD_BYTEMUCK_IMPLS.into(), fully_qualified_name.clone()),
        unsafe_bytemuck_pod_impl,
      ),
    ]
  }

  pub fn from_naga(
    item_path: &'a RustSourceItemPath,
    naga_members: &'a [naga::StructMember],
    naga_module: &'a naga::Module,
    options: &'a WgslBindgenOption,
    layout: naga::proc::TypeLayout,
    is_directly_sharable: bool,
    is_host_sharable: bool,
    has_rts_array: bool,
  ) -> Self {
    let members = RustStructMemberEntry::from_naga(
      options,
      item_path,
      naga_members,
      naga_module,
      layout.size as usize,
      is_directly_sharable,
    );

    RustStructBuilder {
      item_path,
      members,
      is_host_sharable,
      naga_module,
      options,
      has_rts_array,
      layout,
    }
  }
}
