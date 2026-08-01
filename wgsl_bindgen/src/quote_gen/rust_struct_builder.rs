// See also: https://webgpufundamentals.org/webgpu/lessons/resources/wgsl-offset-computer.html

use derive_more::IsVariant;
use naga::common::wgsl::TypeContext;
use naga::StructMember;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use smol_str::SmolStr;
use syn::{Ident, Index};

use super::{
  rust_type, RustSourceItem, RustSourceItemPath, RustTypeInfo, RustTypeInitConversion,
};
use crate::bevy_util::demangle_str;
use crate::quote_gen::{
  generate_derive_attributes, generate_doc_comment, generate_impl_block,
  generate_struct_definition, generate_struct_field, RustSourceItemCategory,
  MOD_BYTEMUCK_IMPLS, MOD_STRUCT_ASSERTIONS,
};
use crate::{
  sanitized_upper_snake_case, WgslBindgenOption, WgslType, WgslTypeSerializeStrategy,
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
    rust_type: &RustTypeInfo,
    member_name: &str,
  ) -> (proc_macro2::TokenStream, bool) {
    let fully_qualified_name = fully_qualified_name.as_str();
    let override_type = options.override_struct_field_type.iter().find_map(|o| {
      let struct_matches = o.struct_regex.is_match(fully_qualified_name);
      let field_matches = o.field_regex.is_match(member_name);
      (struct_matches && field_matches).then_some(o.override_type.clone())
    });

    match override_type {
      Some(override_type) => (override_type, true),
      None => (rust_type.tokens.clone(), false),
    }
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
      let (resolved_rust_type, rust_type_is_overridden) =
        Self::get_rust_type(options, &fully_qualified_name, &rust_type_info, member_name);

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
        let pad_name = format!("_pad_{member_name}");
        let required_member_size = next_offset - current_offset;

        // Padding is determined by the emitted Rust storage representation, not
        // the WGSL value size. A vec3 may already be widened to four scalars and
        // therefore occupy the complete WGSL member slot.
        match rust_type_info.aligned_size() {
          Some(rust_type_size)
            if !rust_type_is_overridden && rust_type_size == required_member_size =>
          {
            None
          }
          _ => {
            let required_member_size = Index::from(required_member_size);
            let pad_name = Ident::new(&pad_name, Span::call_site());
            let pad_size_tokens = quote!(
              #required_member_size - ::core::mem::size_of::<#resolved_rust_type>()
            );

            Some(Padding {
              pad_name,
              pad_size_tokens,
            })
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

      // For builtin fields, we need to add padding instead of the field itself
      if is_builtin_field {
        // For builtin fields, calculate padding for the space the builtin field would occupy
        if is_directly_sharable {
          let current_offset = naga_member.offset as usize;
          let next_offset = if state.index + 1 < naga_members.len() {
            naga_members[state.index + 1].offset as usize
          } else {
            layout_size
          };
          let builtin_field_space = next_offset - current_offset;

          if builtin_field_space > 0 {
            let pad_name = format!("_pad_{member_name}");
            let pad_name = Ident::new(&pad_name, Span::call_site());
            let padding_size_hex = format!("0x{builtin_field_space:X}");
            let pad_size_tokens =
              syn::parse_str::<TokenStream>(&padding_size_hex).unwrap();

            let padding = Padding {
              pad_name,
              pad_size_tokens,
            };
            state.members.push(RustStructMemberEntry::Padding(padding));
          }
        }

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
        let init_type = rust_type_info.init_type.clone().map(syn::Type::Verbatim);
        let init_conversion = rust_type_info.init_conversion.clone();

        RustStructMemberEntry::Field(Field {
          name_ident: name_ident.clone(),
          naga_member,
          naga_type: member_naga_type,
          naga_ty_handle: naga_member.ty,
          rust_type: syn::Type::Verbatim(resolved_rust_type),
          init_type,
          init_conversion,
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
  pub init_type: Option<syn::Type>,
  pub init_conversion: Option<RustTypeInitConversion>,
  pub is_rsa: bool,
}

impl<'a> Field<'a> {
  fn input_type(&self) -> &syn::Type {
    self.init_type.as_ref().unwrap_or(&self.rust_type)
  }

  fn generate_member_instantiate(&self, other_struct_var_name: &Ident) -> TokenStream {
    let name = &self.name_ident;
    if self.has_init_type() {
      self.generate_init_to_target_conversion(other_struct_var_name)
    } else {
      quote!(#name: #other_struct_var_name.#name)
    }
  }

  fn generate_member_definition(&self) -> TokenStream {
    let name = &self.name_ident;
    let ty = &self.rust_type;
    quote!(pub #name: #ty)
  }

  fn generate_init_member_definition(&self, runtime_tail_as_array: bool) -> TokenStream {
    let name = &self.name_ident;
    if self.is_rsa && runtime_tail_as_array {
      let ty = self.input_type();
      quote!(pub #name: [#ty; N])
    } else if let Some(init_ty) = &self.init_type {
      quote!(pub #name: #init_ty)
    } else {
      let ty = &self.rust_type;
      quote!(pub #name: #ty)
    }
  }

  fn generate_fn_new_param(&self, runtime_tail_as_array: bool) -> TokenStream {
    let name = &self.name_ident;
    if self.is_rsa && runtime_tail_as_array {
      let ty = self.input_type();
      quote!(#name: [#ty; N])
    } else {
      let ty = &self.rust_type;
      quote!(#name: #ty)
    }
  }

  fn has_init_type(&self) -> bool {
    self.init_type.is_some()
  }

  fn generate_init_to_target_conversion(
    &self,
    other_struct_var_name: &Ident,
  ) -> TokenStream {
    let name = &self.name_ident;
    let conversion = self
      .init_conversion
      .as_ref()
      .expect("init-friendly field must have a conversion");
    let converted = if self.is_rsa {
      let converted_element = conversion.generate(quote!(value));
      quote!(#other_struct_var_name.#name.map(|value| #converted_element))
    } else {
      conversion.generate(quote!(#other_struct_var_name.#name))
    };
    quote!(#name: #converted)
  }

  fn uses_array_element_helper(&self) -> bool {
    self
      .init_conversion
      .as_ref()
      .is_some_and(RustTypeInitConversion::uses_array_element_helper)
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

  fn runtime_array_field(&self) -> Option<&Field<'a>> {
    if !self.uses_generics_for_rts() {
      return None;
    }

    self.members.iter().find_map(|member| match member {
      RustStructMemberEntry::Field(field) if field.is_rsa => Some(field),
      _ => None,
    })
  }

  fn sized_name_ident(&self) -> Ident {
    if self.uses_generics_for_rts() {
      format_ident!("{}Sized", self.item_path.name.as_str())
    } else {
      self.name_ident()
    }
  }

  fn uses_padding(&self) -> bool {
    self.members.iter().any(|m| match m {
      RustStructMemberEntry::Padding(_) => true,
      RustStructMemberEntry::Field(field) => field.has_init_type(),
    })
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
    let ident = self.sized_name_ident();
    let ty_param_use = self.ty_param_use();
    quote!(#ident #ty_param_use)
  }

  fn fully_qualified_struct_name_in_usage_fragment(&self) -> TokenStream {
    let fully_qualified_name_str = if self.uses_generics_for_rts() {
      format!("{}Sized", self.item_path.get_fully_qualified_name())
    } else {
      self.item_path.get_fully_qualified_name().to_string()
    };
    let fully_qualified_name =
      syn::parse_str::<TokenStream>(&fully_qualified_name_str).unwrap();
    let ty_param_use = self.ty_param_use();
    quote!(#fully_qualified_name #ty_param_use)
  }

  fn struct_name_in_definition_fragment(&self) -> TokenStream {
    let ident = self.name_ident();
    if let Some(field) = self.runtime_array_field() {
      let element_type = &field.rust_type;
      quote! {
        #ident<Tail = [#element_type]>
        where
          Tail: WgslBindgenRuntimeArray<#element_type> + ?Sized
      }
    } else {
      quote!(#ident)
    }
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
    let struct_name = self.sized_name_ident();
    let init_struct_name_def = self.init_struct_name_in_definition_fragment();
    let init_struct_name_in_usage = self.init_struct_name_in_usage_fragment();
    let visibility = self.options.type_visibility.generate_quote();

    let mut init_struct_members = vec![];
    let mut mem_assignments = vec![];

    let init_var_name = Ident::new("self", Span::call_site());

    for entry in self.members.iter() {
      match entry {
        RustStructMemberEntry::Field(field) => {
          init_struct_members
            .push(field.generate_init_member_definition(self.uses_generics_for_rts()));
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
      pub fn build(&self) -> #struct_name_in_usage {
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
          non_padding_members
            .push(field.generate_fn_new_param(self.uses_generics_for_rts()));
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

  fn build_fn_new_runtime(&self) -> TokenStream {
    let struct_name = self.name_ident();
    let sized_name = self.sized_name_ident();
    let visibility = self.options.type_visibility.generate_quote();
    let tail = self
      .runtime_array_field()
      .expect("runtime-sized struct must have a runtime array field");
    let tail_name = &tail.name_ident;
    let element_type = &tail.rust_type;
    let input_element_type = tail.input_type();
    let sized_new_const = if tail.has_init_type() {
      quote!()
    } else {
      quote!(const)
    };
    let sized_new_bounds = if tail.has_init_type() {
      quote!(where #input_element_type: bytemuck::Pod,)
    } else {
      quote!()
    };
    let validate_tail_element_size = if tail.has_init_type() {
      quote! {
        assert!(
          mem::size_of::<#input_element_type>() <= mem::size_of::<#element_type>(),
          "Rust array element exceeds its WGSL stride",
        );
      }
    } else {
      quote!()
    };
    let raw_tail_write = if tail.has_init_type() {
      let conversion = tail
        .init_conversion
        .as_ref()
        .expect("init-friendly runtime tail must have a conversion");
      let converted = conversion.generate(quote!(value));
      quote! {
        let __wgsl_bindgen_tail_destination =
          ptr::addr_of_mut!((*__wgsl_bindgen_this).#tail_name)
            .cast::<#element_type>();
        for (index, value) in #tail_name.iter().copied().enumerate() {
          __wgsl_bindgen_tail_destination
            .add(index)
            .write(#converted);
        }
      }
    } else {
      quote! {
        ptr::copy_nonoverlapping(
          #tail_name.as_ptr(),
          ptr::addr_of_mut!((*__wgsl_bindgen_this).#tail_name)
            .cast::<#element_type>(),
          #tail_name.len(),
        );
      }
    };

    let mut fixed_params = Vec::new();
    let mut sized_params = Vec::new();
    let mut sized_assignments = Vec::new();
    let mut raw_writes = Vec::new();
    let mut byte_types = Vec::new();

    for entry in &self.members {
      match entry {
        RustStructMemberEntry::Field(field) => {
          let name = &field.name_ident;
          byte_types.push(&field.rust_type);
          sized_params.push(field.generate_fn_new_param(true));
          if field.is_rsa && field.has_init_type() {
            let conversion = field
              .init_conversion
              .as_ref()
              .expect("init-friendly runtime tail must have a conversion");
            let converted = conversion.generate(quote!(value));
            sized_assignments.push(quote!(#name: #name.map(|value| #converted)));
          } else {
            sized_assignments.push(quote!(#name));
          }

          if !field.is_rsa {
            fixed_params.push(field.generate_fn_new_param(true));
            raw_writes.push(quote! {
              ::core::ptr::addr_of_mut!((*__wgsl_bindgen_this).#name).write(#name);
            });
          }
        }
        RustStructMemberEntry::Padding(padding) => {
          sized_assignments.push(padding.generate_member_instantiate());
          let pad_name = &padding.pad_name;
          let pad_size = &padding.pad_size_tokens;
          raw_writes.push(quote! {
            ::core::ptr::addr_of_mut!((*__wgsl_bindgen_this).#pad_name)
              .write([0; #pad_size]);
          });
        }
      }
    }

    quote! {
      #visibility type #sized_name<const N: usize> = #struct_name<[#element_type; N]>;

      impl<const N: usize> #sized_name<N> {
        pub #sized_new_const fn new_sized(#(#sized_params),*) -> Self
        #sized_new_bounds
        {
          Self {
            #(#sized_assignments),*
          }
        }

        pub fn as_bytes(&self) -> &[u8]
        where
          #(#byte_types: bytemuck::Pod,)*
        {
          let __wgsl_bindgen_unsized: &#struct_name = self;
          __wgsl_bindgen_unsized.as_bytes()
        }
      }

      impl #struct_name {
        pub fn new(
          #(#fixed_params,)*
          #tail_name: &[#input_element_type],
        ) -> Box<Self>
        where
          #input_element_type: bytemuck::Pod,
        {
          use std::{alloc, mem, ptr};

          let __wgsl_bindgen_tail_offset =
            mem::offset_of!(#sized_name<0>, #tail_name);
          let __wgsl_bindgen_tail_size = #tail_name
            .len()
            .checked_mul(mem::size_of::<#element_type>())
            .expect("runtime-sized struct allocation is too large");
          let __wgsl_bindgen_unpadded_size = __wgsl_bindgen_tail_offset
            .checked_add(__wgsl_bindgen_tail_size)
            .expect("runtime-sized struct allocation is too large");
          let __wgsl_bindgen_alignment = mem::align_of::<#sized_name<0>>();
          let __wgsl_bindgen_allocation_size = __wgsl_bindgen_unpadded_size
            .checked_add(__wgsl_bindgen_alignment - 1)
            .map(|size| size & !(__wgsl_bindgen_alignment - 1))
            .expect("runtime-sized struct allocation is too large");
          let __wgsl_bindgen_layout = alloc::Layout::from_size_align(
            __wgsl_bindgen_allocation_size,
            __wgsl_bindgen_alignment,
          )
          .expect("runtime-sized struct has an invalid allocation layout");
          #validate_tail_element_size

          unsafe {
            let __wgsl_bindgen_allocation = if __wgsl_bindgen_allocation_size == 0 {
              ptr::NonNull::<#sized_name<0>>::dangling()
                .as_ptr()
                .cast::<u8>()
            } else {
              // Every field, explicit padding byte, and tail element is written
              // below. Any final allocation padding is excluded from as_bytes().
              let allocation = alloc::alloc(__wgsl_bindgen_layout);
              if allocation.is_null() {
                alloc::handle_alloc_error(__wgsl_bindgen_layout);
              }
              allocation
            };

            let __wgsl_bindgen_tail = ptr::slice_from_raw_parts_mut(
              __wgsl_bindgen_allocation.cast::<#element_type>(),
              #tail_name.len(),
            );
            let __wgsl_bindgen_this = __wgsl_bindgen_tail as *mut Self;

            #(#raw_writes)*
            #raw_tail_write

            Box::from_raw(__wgsl_bindgen_this)
          }
        }

        pub fn as_bytes(&self) -> &[u8]
        where
          #(#byte_types: bytemuck::Pod,)*
        {
          let __wgsl_bindgen_len =
            ::core::mem::offset_of!(#sized_name<0>, #tail_name)
              + ::core::mem::size_of_val(&self.#tail_name);
          unsafe {
            std::slice::from_raw_parts(
              self as *const Self as *const u8,
              __wgsl_bindgen_len,
            )
          }
        }
      }
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
            init_type: _,
            init_conversion: _,
          } = field;

          let doc_comment = if self.is_directly_shareable() {
            let offset = member.offset;
            let size = naga_type.inner.size(naga_context);
            let ty_name = naga_context.type_to_string(*naga_ty_handle);
            let ty_name = demangle_str(&ty_name);
            let doc = format!("offset: {offset}, size: {size}, type: `{ty_name}`");

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

          let field_type = if *is_rts && self.uses_generics_for_rts() {
            quote!(Tail)
          } else {
            quote!(#rust_type)
          };

          quote! {
            #doc_comment
            #runtime_size_attribute
            pub #name: #field_type
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

  fn calculate_actual_struct_size(&self) -> usize {
    let naga_context = self.naga_module.to_ctx();

    // Find the last field and calculate struct size
    let mut max_end = 0usize;

    for (idx, entry) in self.members.iter().enumerate() {
      match entry {
        RustStructMemberEntry::Field(field) => {
          let offset = field.naga_member.offset as usize;
          let size = field.naga_type.inner.size(naga_context) as usize;
          let field_end = offset + size;
          max_end = max_end.max(field_end);
        }
        RustStructMemberEntry::Padding(_) => {
          // Padding fields have already been calculated to fill gaps
          // We'll find their size from the next field or end of struct
          if idx + 1 < self.members.len() {
            if let RustStructMemberEntry::Field(next_field) = &self.members[idx + 1] {
              max_end = max_end.max(next_field.naga_member.offset as usize);
            }
          }
        }
      }
    }

    // If we didn't find any fields (shouldn't happen), use 0
    if max_end == 0 {
      return 0;
    }

    // Round up to struct alignment
    let struct_alignment = self.layout.alignment;
    struct_alignment.round_up(max_end as u32) as usize
  }

  fn build_layout_assertion(
    &self,
    custom_alignment: Option<naga::proc::Alignment>,
  ) -> TokenStream {
    let fully_qualified_name_str = self.item_path.get_fully_qualified_name();

    let fully_qualified_name =
      syn::parse_str::<TokenStream>(&fully_qualified_name_str).unwrap();
    let struct_name = if self.uses_generics_for_rts() {
      let sized_name = format!("{fully_qualified_name_str}Sized");
      let sized_name = syn::parse_str::<TokenStream>(&sized_name).unwrap();
      quote!(#sized_name<0>)
    } else {
      quote!(#fully_qualified_name)
    };

    // Calculate actual Rust struct offsets including padding fields
    let mut assert_member_offsets = Vec::new();
    let mut current_rust_offset = 0usize;

    for m in &self.members {
      match m {
        RustStructMemberEntry::Field(field) => {
          let name =
            Ident::new(field.naga_member.name.as_ref().unwrap(), Span::call_site());
          let rust_offset = quote!(std::mem::offset_of!(#struct_name, #name));

          // Use the WGSL offset from naga, which is the correct expected offset
          let expected_offset = Index::from(field.naga_member.offset as usize);

          assert_member_offsets.push(quote!(assert!(#rust_offset == #expected_offset);));

          // Don't need to track current_rust_offset since we use WGSL offsets directly
        }
        RustStructMemberEntry::Padding(_padding) => {
          // Padding doesn't have assertions
        }
      }
    }

    if self.is_directly_shareable() {
      let expected_alignment = custom_alignment.unwrap_or(self.layout.alignment) * 1u32;
      let expected_alignment = Index::from(expected_alignment as usize);

      let size_assertion = if let Some(tail) = self.runtime_array_field() {
        let stride = match &tail.naga_type.inner {
          naga::TypeInner::Array {
            size: naga::ArraySize::Dynamic,
            stride,
            ..
          } => *stride,
          _ => unreachable!("runtime array field must use a dynamic array type"),
        };
        let stride = Index::from(stride as usize);
        let element_type = if tail.has_init_type() {
          let helper_path = RustSourceItemPath::new(
            self.item_path.module.clone(),
            "WgslBindgenArrayElement".into(),
          );
          quote!(#helper_path<#stride>)
        } else {
          let rust_type = &tail.rust_type;
          quote!(#rust_type)
        };
        quote! {
          assert!(std::mem::size_of::<#element_type>() == #stride);
        }
      } else {
        // For fixed-size bytemuck types, the explicit padding must make the
        // complete Rust object match the WGSL struct size.
        let struct_size = if self.members.is_empty() {
          0
        } else {
          self.layout.size as usize
        };
        let struct_size = custom_alignment
          .map(|alignment| alignment.round_up(struct_size as u32) as usize)
          .unwrap_or(struct_size);
        let struct_size = Index::from(struct_size);
        quote! {
          assert!(std::mem::size_of::<#struct_name>() == #struct_size);
        }
      };

      let assertion_name = format_ident!(
        "{}_ASSERTS",
        sanitized_upper_snake_case(&fully_qualified_name_str)
      );

      quote! {
        const #assertion_name: () = {
          #(#assert_member_offsets)*
          assert!(std::mem::align_of::<#struct_name>() == #expected_alignment);
          #size_assertion
        };
      }
    } else {
      quote!()
    }
  }

  pub fn build_bytemuck_impls(&self) -> TokenStream {
    let struct_name_in_usage = self.fully_qualified_struct_name_in_usage_fragment();
    let impl_fragment = self.impl_trait_for_fragment();

    if self.options.serialization_strategy == WgslTypeSerializeStrategy::Bytemuck
      && !self.uses_generics_for_rts()
    {
      quote! {
        unsafe #impl_fragment bytemuck::Zeroable for #struct_name_in_usage {}
        unsafe #impl_fragment bytemuck::Pod for #struct_name_in_usage {}
      }
    } else {
      quote!()
    }
  }

  fn build_runtime_array_trait_helper(&self) -> Option<RustSourceItem> {
    if !self.uses_generics_for_rts() {
      return None;
    }

    Some(RustSourceItem::new(
      RustSourceItemCategory::TypeDefs | RustSourceItemCategory::TraitImpls,
      RustSourceItemPath::new(
        self.item_path.module.clone(),
        "WgslBindgenRuntimeArray".into(),
      ),
      quote! {
        #[doc(hidden)]
        mod __wgsl_bindgen_runtime_array_sealed {
          pub trait Sealed {}
          impl<T> Sealed for [T] {}
          impl<T, const N: usize> Sealed for [T; N] {}
        }

        #[doc(hidden)]
        pub trait WgslBindgenRuntimeArray<T>:
          __wgsl_bindgen_runtime_array_sealed::Sealed
        {}
        impl<T> WgslBindgenRuntimeArray<T> for [T] {}
        impl<T, const N: usize> WgslBindgenRuntimeArray<T> for [T; N] {}
      },
    ))
  }

  fn build_array_element_helper(&self) -> Option<RustSourceItem> {
    let uses_padded_array_element = self.members.iter().any(|member| match member {
      RustStructMemberEntry::Field(field) => field.uses_array_element_helper(),
      RustStructMemberEntry::Padding(_) => false,
    });
    if !uses_padded_array_element {
      return None;
    }

    Some(array_element_helper_item(self.item_path.module.as_str(), self.options))
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
    let repr_c = if should_generate_padding {
      Some(quote!(#[repr(C, align(#alignment))]))
    } else {
      Some(quote!(#[repr(C)]))
    };

    let fields = self.build_fields();
    let struct_new_fn = if self.uses_generics_for_rts() {
      self.build_fn_new_runtime()
    } else {
      self.build_fn_new()
    };
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

    let mut items = vec![
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
    ];

    if let Some(array_element_helper) = self.build_array_element_helper() {
      items.push(array_element_helper);
    }
    if let Some(runtime_array_trait_helper) = self.build_runtime_array_trait_helper() {
      items.push(runtime_array_trait_helper);
    }

    items
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

pub(crate) fn array_element_helper_item(
  module: &str,
  options: &WgslBindgenOption,
) -> RustSourceItem {
  let derives = generate_derive_attributes(&["Debug", "PartialEq", "Clone", "Copy"]);
  let serde_impls = options.derive_serde.then(|| {
    quote! {
      impl<const STRIDE: usize> serde::Serialize for WgslBindgenArrayElement<STRIDE> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
          S: serde::Serializer,
        {
          let mut tuple = serde::Serializer::serialize_tuple(serializer, STRIDE)?;
          for byte in &self.0 {
            serde::ser::SerializeTuple::serialize_element(&mut tuple, byte)?;
          }
          serde::ser::SerializeTuple::end(tuple)
        }
      }

      struct WgslBindgenArrayElementVisitor<const STRIDE: usize>;

      impl<'de, const STRIDE: usize> serde::de::Visitor<'de>
        for WgslBindgenArrayElementVisitor<STRIDE>
      {
        type Value = WgslBindgenArrayElement<STRIDE>;

        fn expecting(
          &self,
          formatter: &mut std::fmt::Formatter<'_>,
        ) -> std::fmt::Result {
          formatter.write_str("a byte array with the generated WGSL stride")
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
          A: serde::de::SeqAccess<'de>,
        {
          let mut bytes = [0u8; STRIDE];
          for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = sequence
              .next_element()?
              .ok_or_else(|| serde::de::Error::invalid_length(index, &self))?;
          }
          if sequence
            .next_element::<serde::de::IgnoredAny>()?
            .is_some()
          {
            return Err(serde::de::Error::invalid_length(STRIDE + 1, &self));
          }
          Ok(WgslBindgenArrayElement(bytes))
        }
      }

      impl<'de, const STRIDE: usize> serde::Deserialize<'de>
        for WgslBindgenArrayElement<STRIDE>
      {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
          D: serde::Deserializer<'de>,
        {
          serde::Deserializer::deserialize_tuple(
            deserializer,
            STRIDE,
            WgslBindgenArrayElementVisitor::<STRIDE>,
          )
        }
      }
    }
  });

  RustSourceItem::new(
    RustSourceItemCategory::TypeDefs | RustSourceItemCategory::TraitImpls,
    RustSourceItemPath::new(module.into(), "WgslBindgenArrayElement".into()),
    quote! {
      #[doc(hidden)]
      #[repr(transparent)]
      #derives
      pub struct WgslBindgenArrayElement<const STRIDE: usize>(pub [u8; STRIDE]);

      impl<const STRIDE: usize> WgslBindgenArrayElement<STRIDE> {
        pub fn new<T: bytemuck::Pod>(value: T) -> Self {
          assert!(
            ::core::mem::size_of::<T>() <= STRIDE,
            "Rust array element exceeds its WGSL stride",
          );
          let value_bytes = bytemuck::bytes_of(&value);
          let mut result = ::core::mem::MaybeUninit::<Self>::uninit();

          unsafe {
            let result_bytes = result.as_mut_ptr().cast::<u8>();
            ::core::ptr::copy_nonoverlapping(
              value_bytes.as_ptr(),
              result_bytes,
              value_bytes.len(),
            );
            ::core::ptr::write_bytes(
              result_bytes.add(value_bytes.len()),
              0,
              STRIDE - value_bytes.len(),
            );

            // The value bytes and every remaining stride byte were initialized.
            result.assume_init()
          }
        }
      }

      unsafe impl<const STRIDE: usize> bytemuck::Zeroable
        for WgslBindgenArrayElement<STRIDE>
      {}
      unsafe impl<const STRIDE: usize> bytemuck::Pod
        for WgslBindgenArrayElement<STRIDE>
      {}

      #serde_impls
    },
  )
}
