use std::collections::HashSet;

use naga::{Handle, Type};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Index};

use crate::{wgsl::rust_type, ShaderSerializationStrategy, WriteOptions};

#[derive(Clone)]
struct StructMemberEntryPadding {
  pad_name: Ident,
  q_pad_size: TokenStream,
}

impl StructMemberEntryPadding {
  fn build_inst_quote(&self) -> TokenStream {
    let pad_name = &self.pad_name;
    let pad_size = &self.q_pad_size;
    quote!(#pad_name: [0; #pad_size])
  }

  fn build_def_quote(&self) -> TokenStream {
    let pad_name = &self.pad_name;
    let pad_size = &self.q_pad_size;
    quote!(pub #pad_name: [u8; #pad_size])
  }
}

struct StructMemberEntry<'a> {
  name: Ident,
  member: &'a naga::StructMember,
  ty: &'a naga::Type,
  q_ty: syn::Type,
  padding: Option<StructMemberEntryPadding>,
  is_rat: bool,
}

impl<'a> StructMemberEntry<'a> {
  fn build_inst_quote(&self, other_struct_var_name: &Ident) -> TokenStream {
    let name = &self.name;
    quote!(#name: #other_struct_var_name.#name)
  }

  fn build_def_quote(&self) -> TokenStream {
    let name = &self.name;
    let ty = &self.q_ty;
    quote!(pub #name: #ty)
  }

  fn build_new_fn_param_quote(&self) -> TokenStream {
    let name = &self.name;
    let ty = &self.q_ty;
    quote!(#name: #ty)
  }
}

struct StructBuilder<'a> {
  r#type: &'a naga::Type,
  name: Ident,
  members: Vec<StructMemberEntry<'a>>,
  is_host_shareable: bool,
  has_rts_array: bool,
  module: &'a naga::Module,
  layout: &'a naga::proc::TypeLayout,
  options: &'a WriteOptions
}

impl<'a> StructBuilder<'a> {

  fn is_directly_shareable(&self) -> bool {
    self.options.serialization_strategy == ShaderSerializationStrategy::Bytemuck
    && self.is_host_shareable
  }

  fn uses_generics_for_rts(&self) -> bool {
    self.has_rts_array && self.options.serialization_strategy == ShaderSerializationStrategy::Bytemuck
  }

  fn uses_padding(&self) -> bool {
    self.members.iter().any(|m| m.padding.is_some())
  }

  fn struct_name_in_usage_fragment(&self) -> TokenStream {
    let ident = self.name.clone();

    if self.uses_generics_for_rts() {
      quote!(#ident<N>)
    } else {
      quote!(#ident)
    }
  }

  fn struct_name_in_definition_fragment(&self) -> TokenStream {
    let ident = self.name.clone();

    if self.uses_generics_for_rts() {
      quote!(#ident<const N: usize>)
    } else {
      quote!(#ident)
    }
  }

  fn init_struct_name_in_usage_fragment(&self) -> TokenStream {
    let name = format!("{}Init", self.r#type.name.clone().unwrap());
    let ident = Ident::new(&name, Span::call_site());
    if self.uses_generics_for_rts() {
      quote!(#ident<N>)
    } else {
      quote!(#ident)
    }
  }

  fn init_struct_name_in_definition_fragment(&self) -> TokenStream {
    let name = format!("{}Init", self.r#type.name.clone().unwrap());
    let ident = Ident::new(&name, Span::call_site());
    if self.uses_generics_for_rts() {
      quote!(#ident<const N: usize>)
    } else {
      quote!(#ident)
    }
  }

  fn impl_trait_for_fragment(&self) -> TokenStream {
    if self.uses_generics_for_rts() {
      quote!(impl<const N:usize>)
    } else {
      quote!(impl)
    }
  }

  fn build_init_struct(&self) -> TokenStream {
    if !self.is_directly_shareable() || !self.uses_padding() {
      return quote!();
    }

    let impl_fragment = self.impl_trait_for_fragment();
    let struct_name_usage = self.struct_name_in_usage_fragment();
    let struct_name = self.name.clone();
    let init_struct_name_def = self.init_struct_name_in_definition_fragment();
    let init_struct_name_usage = self.init_struct_name_in_usage_fragment();

    let mut init_struct_members = vec![]; 
    let mut mem_assignments = vec![];
    
    let init_var_name = Ident::new("init", Span::call_site());
    
    for entry in self.members.iter() {
      init_struct_members.push(entry.build_def_quote());
      mem_assignments.push(entry.build_inst_quote(&init_var_name));

      for pad in entry.padding.iter() {
        mem_assignments.push(pad.build_inst_quote())
      }
    };

    quote! {
      #[repr(C)]
      #[derive(Debug, PartialEq, Clone, Copy)]
      pub struct #init_struct_name_def {
        #(#init_struct_members),*
      }

      #impl_fragment #init_struct_name_usage {
        pub const fn const_into(&self) -> #struct_name_usage {
          let init = self;
          #struct_name {
            #(#mem_assignments),*
          }
        }
      }

      #impl_fragment From<#init_struct_name_usage> for #struct_name_usage {
        fn from(init: #init_struct_name_usage) -> Self {
          Self {
            #(#mem_assignments),*
          }
        }
      }
    }
  }

  fn build_fn_new(&self) -> TokenStream {
    let struct_name_usage = self.struct_name_in_usage_fragment();
    let impl_fragment = self.impl_trait_for_fragment();

    let mut non_padding_members = Vec::new();
    let mut member_assignments = Vec::new();

    for entry in &self.members {
      let name = &entry.name;
      non_padding_members.push(entry.build_new_fn_param_quote());
      member_assignments.push(quote!(#name));

      for p in entry.padding.iter() {
        member_assignments.push(p.build_inst_quote())
      }
    }

    quote! {
      #impl_fragment #struct_name_usage {
        pub fn new(
          #(#non_padding_members),*
        ) -> Self {
          Self {
            #(#member_assignments),*
          }
        }
      }
    }
  }

  fn build_fields(&self) -> Vec<TokenStream> {
    let gctx = self.module.to_ctx();
    let members = self.members
      .iter()
      .map(|StructMemberEntry { name, q_ty, is_rat, member, ty, padding }| {
        let doc = if self.is_directly_shareable() {
          let offset = member.offset;
          let size = ty.inner.size(gctx);
          let ty_name = ty.inner.to_wgsl(&gctx);
          let doc = format!(" size: {}, offset: 0x{:X}, type: {}", size, offset, ty_name);

          quote!(#[doc = #doc])
        } else {
          quote!()
        };

        let runtime_size_attribute = if *is_rat && matches!(self.options.serialization_strategy, ShaderSerializationStrategy::Encase) {
          quote!(#[size(runtime)])
        } else {
          quote!()
        };

        let mut qs = vec![
          quote!{
            #doc
            #runtime_size_attribute
            pub #name: #q_ty
          },
        ];

        for padding in padding.iter() {
          qs.push(padding.build_def_quote());
        }

        quote!(#(#qs), *)
      })
      .collect::<Vec<_>>();

      members
  }

  fn build_derives(&self) -> Vec<TokenStream> {
    let mut derives = Vec::new();
    derives.push(quote!(Debug));
    derives.push(quote!(PartialEq));
    derives.push(quote!(Clone));

    match self.options.serialization_strategy {
      ShaderSerializationStrategy::Bytemuck => {
        derives.push(quote!(Copy));
      },
      ShaderSerializationStrategy::Encase => {
        if !self.has_rts_array {
            derives.push(quote!(Copy));
        }
        derives.push(quote!(encase::ShaderType));
      }
    }
    if self.options.derive_serde {
        derives.push(quote!(serde::Serialize));
        derives.push(quote!(serde::Deserialize));
    }
    derives
  }

  fn build_assert_layout(&self) -> TokenStream {
    let ident = self.name.clone(); 
    let struct_name = if self.uses_generics_for_rts() {
      quote!(#ident<1>) // test RTS with 1 element
    } else {
      quote!(#ident)
    };

    // TODO: Does the Rust alignment matter if it's copied to a buffer anyway?
    let struct_size = Index::from(self.layout.size as usize);

    let assert_size_text = format!("size of {} does not match WGSL", self.r#type.name.as_ref().unwrap());
    let assert_size = quote! {
        const _: () = assert!(std::mem::size_of::<#struct_name>() == #struct_size, #assert_size_text);
    };
    let assert_member_offsets: Vec<_> = self.members
        .iter()
        .map(|m| {
            let m = m.member;
            let name = Ident::new(m.name.as_ref().unwrap(), Span::call_site());
            let rust_offset = quote!(memoffset::offset_of!(#struct_name, #name));

            let wgsl_offset = Index::from(m.offset as usize);

            let assert_text = format!(
                "offset of {}.{} does not match WGSL",
                self.r#type.name.as_ref().unwrap(),
                m.name.as_ref().unwrap()
            );
            quote! {
                const _: () = assert!(#rust_offset == #wgsl_offset, #assert_text);
            }
        })
        .collect();

    if self.is_directly_shareable() {
        // Assert that the Rust layout matches the WGSL layout.
        // Enable for bytemuck since it uses the Rust struct's memory layout.
        quote! {
            #assert_size
            #(#assert_member_offsets)*
        }
    } else {
        quote!()
    }
  }

  fn build(&self) -> TokenStream {
    let struct_name_def = self.struct_name_in_definition_fragment();
    let struct_name_usage = self.struct_name_in_usage_fragment();
    let impl_fragment = self.impl_trait_for_fragment();

    // Assume types used in global variables are host shareable and require validation.
    // This includes storage, uniform, and workgroup variables.
    // This also means types that are never used will not be validated.
    // Structs used only for vertex inputs do not require validation on desktop platforms.
    // Vertex input layout is handled already by setting the attribute offsets and types.
    // This allows vertex input field types without padding like vec3 for positions.
    let is_host_shareable = self.is_host_shareable;

    let has_rts_array = self.has_rts_array;
    let should_generate_padding = is_host_shareable && self.options.serialization_strategy == ShaderSerializationStrategy::Bytemuck;

    let derives = self.build_derives();


    let alignment = Index::from((self.layout.alignment * 1u32) as usize);
    let repr_c = if !has_rts_array {
      if should_generate_padding {
        quote!(#[repr(C, align(#alignment))])
      }
      else {
        quote!(#[repr(C)])
      }
    } else {
        quote!()
    };

    let fields = self.build_fields();
    let struct_new_fn = self.build_fn_new();
    let init_struct = self.build_init_struct();
    let assert_layout = self.build_assert_layout();

    let unsafe_bytemuck_pod_impl = 
      if self.options.serialization_strategy == ShaderSerializationStrategy::Bytemuck {
      quote!{
        unsafe #impl_fragment bytemuck::Zeroable for #struct_name_usage {}
        unsafe #impl_fragment bytemuck::Pod for #struct_name_usage {}
      }
    } else {
      quote!()
    };

    quote! {
        #repr_c
        #[derive(#(#derives),*)]
        pub struct #struct_name_def {
            #(#fields),*
        }
        
        #struct_new_fn
        #unsafe_bytemuck_pod_impl
        #init_struct
        #assert_layout
    }
  }
}

pub fn structs(module: &naga::Module, options: WriteOptions) -> Vec<TokenStream> {
    // Initialize the layout calculator provided by naga.
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).unwrap();

    let mut global_variable_types = HashSet::new();
    for g in module.global_variables.iter() {
        add_types_recursive(&mut global_variable_types, module, g.1.ty);
    }

    // Create matching Rust structs for WGSL structs.
    // This is a UniqueArena, so each struct will only be generated once.
    module
        .types
        .iter()
        .filter(|(h, _)| {
            // Check if the struct will need to be used by the user from Rust.
            // This includes function inputs like vertex attributes and global variables.
            // Shader stage function outputs will not be accessible from Rust.
            // Skipping internal structs helps avoid issues deriving encase or bytemuck.
            !module
                .entry_points
                .iter()
                .any(|e| e.function.result.as_ref().map(|r| r.ty) == Some(*h))
                && module
                    .entry_points
                    .iter()
                    .any(|e| e.function.arguments.iter().any(|a| a.ty == *h))
                || global_variable_types.contains(h)
        })
        .filter_map(|(t_handle, t)| {
            if let naga::TypeInner::Struct { members, .. } = &t.inner {
                Some(rust_struct(
                    t,
                    members,
                    &layouter,
                    t_handle,
                    module,
                    options,
                    &global_variable_types,
                ))
            } else {
                None
            }
        })
        .collect()
}

fn rust_struct(
    t: &naga::Type,
    members: &[naga::StructMember],
    layouter: &naga::proc::Layouter,
    t_handle: naga::Handle<naga::Type>,
    module: &naga::Module,
    options: WriteOptions,
    global_variable_types: &HashSet<Handle<Type>>,
) -> TokenStream {
    let struct_name = Ident::new(t.name.as_ref().unwrap(), Span::call_site());

    let layout = layouter[t_handle];

    // Assume types used in global variables are host shareable and require validation.
    // This includes storage, uniform, and workgroup variables.
    // This also means types that are never used will not be validated.
    // Structs used only for vertex inputs do not require validation on desktop platforms.
    // Vertex input layout is handled already by setting the attribute offsets and types.
    // This allows vertex input field types without padding like vec3 for positions.
    let is_host_shareable = global_variable_types.contains(&t_handle);

    let has_rts_array = struct_has_rts_array_member(members, module);
    let is_directly_sharable = options.serialization_strategy == ShaderSerializationStrategy::Bytemuck && is_host_shareable;
    let struct_members_entries = struct_members(members, module, options, layout.size as usize, is_directly_sharable);

    let builder = StructBuilder {
        name: struct_name.clone(),
        members: struct_members_entries,
        is_host_shareable, 
        module,
        options: &options,
        has_rts_array,
        layout: &layout,
        r#type: t,
    };

    builder.build()
}

fn add_types_recursive(
    types: &mut HashSet<naga::Handle<naga::Type>>,
    module: &naga::Module,
    ty: Handle<Type>,
) {
    types.insert(ty);

    match &module.types[ty].inner {
        naga::TypeInner::Pointer { base, .. } => add_types_recursive(types, module, *base),
        naga::TypeInner::Array { base, .. } => add_types_recursive(types, module, *base),
        naga::TypeInner::Struct { members, .. } => {
            for member in members {
                add_types_recursive(types, module, member.ty);
            }
        }
        naga::TypeInner::BindingArray { base, .. } => add_types_recursive(types, module, *base),
        _ => (),
    }
}

fn struct_members<'a>(
    members: &'a [naga::StructMember],
    module: &'a naga::Module,
    options: WriteOptions,
    required_struct_size: usize,
    is_directly_sharable: bool
) -> Vec<StructMemberEntry<'a>> {
    let mut mems = members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            let member_name = Ident::new(member.name.as_ref().unwrap(), Span::call_site());
            let ty = &module.types[member.ty];

            if let naga::TypeInner::Array {
                base,
                size: naga::ArraySize::Dynamic,
                stride: _,
            } = &ty.inner
            {
                if index != members.len() - 1 {
                    panic!("Only the last field of a struct can be a runtime-sized array");
                }
                let element_type = rust_type(module, &module.types[*base], &options);
                let member_type = match options.serialization_strategy {
                    ShaderSerializationStrategy::Encase => 
                      syn::Type::Verbatim(quote!(Vec<#element_type>)),
                    ShaderSerializationStrategy::Bytemuck => 
                      syn::Type::Verbatim(quote!([#element_type; N])),
                };

                StructMemberEntry {
                  name: member_name.clone(), 
                  member,
                  ty,
                  q_ty: member_type.clone(), 
                  is_rat: true,
                  padding: None
                }
            } else {
              let member_type = syn::Type::Verbatim(rust_type(module, ty, &options));
              StructMemberEntry {
                name: member_name.clone(), 
                member,
                ty,
                q_ty: member_type, 
                is_rat: false,
                padding: None
              }
            }
        })
        .collect::<Vec<_>>();


    // start adding padding
    if is_directly_sharable {
      for (i, member) in mems.iter_mut().enumerate() {
        let current_offset = member.member.offset as usize;
        let next_offset = if i + 1 < members.len() {
          members[i + 1].offset as usize
        } else {
          required_struct_size
        };

        if member.is_rat {
          continue;
        }

        let pad_name = format!("_pad{}", member.member.name.clone().unwrap());
        let member_type = &member.q_ty;

        let current_offset = Index::from(current_offset);
        let next_offset = Index::from(next_offset);

        let pad_name = Ident::new(&pad_name, Span::call_site());
        let q_pad_size = quote!(#next_offset - #current_offset - core::mem::size_of::<#member_type>());

        let padding = StructMemberEntryPadding { 
          pad_name,
          q_pad_size
        };

        member.padding = Some(padding)
      }
    }

    mems
}

fn struct_has_rts_array_member(members: &[naga::StructMember], module: &naga::Module) -> bool {
    members.iter().any(|m| {
        matches!(
            module.types[m.ty].inner,
            naga::TypeInner::Array {
                size: naga::ArraySize::Dynamic,
                ..
            }
        )
    })
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::{assert_tokens_eq, MatrixVectorTypes, ShaderSerializationStrategy, WriteOptions};

    #[test]
    fn write_all_structs_rust() {
        let source = indoc! {r#"
            struct Scalars {
                a: u32,
                b: i32,
                c: f32,
            };
            var<uniform> a: Scalars;

            struct VectorsU32 {
                a: vec2<u32>,
                b: vec3<u32>,
                c: vec4<u32>,
            };
            var<uniform> b: VectorsU32;

            struct VectorsI32 {
                a: vec2<i32>,
                b: vec3<i32>,
                c: vec4<i32>,
            };
            var<uniform> c: VectorsI32;

            struct VectorsF32 {
                a: vec2<f32>,
                b: vec3<f32>,
                c: vec4<f32>,
            };
            var<uniform> d: VectorsF32;

            struct VectorsF64 {
                a: vec2<f64>,
                b: vec3<f64>,
                c: vec4<f64>,
            };
            var<uniform> e: VectorsF64;

            struct MatricesF32 {
                a: mat4x4<f32>,
                b: mat4x3<f32>,
                c: mat4x2<f32>,
                d: mat3x4<f32>,
                e: mat3x3<f32>,
                f: mat3x2<f32>,
                g: mat2x4<f32>,
                h: mat2x3<f32>,
                i: mat2x2<f32>,
            };
            var<uniform> f: MatricesF32;

            struct MatricesF64 {
                a: mat4x4<f64>,
                b: mat4x3<f64>,
                c: mat4x2<f64>,
                d: mat3x4<f64>,
                e: mat3x3<f64>,
                f: mat3x2<f64>,
                g: mat2x4<f64>,
                h: mat2x3<f64>,
                i: mat2x2<f64>,
            };
            var<uniform> g: MatricesF64;

            struct StaticArrays {
                a: array<u32, 5>,
                b: array<f32, 3>,
                c: array<mat4x4<f32>, 512>,
            };
            var<uniform> h: StaticArrays;

            struct Nested {
                a: MatricesF32,
                b: MatricesF64
            }
            var<uniform> i: Nested;

            @fragment
            fn main() {}
        "#};

        let module = naga::front::wgsl::parse_str(source).unwrap();

        let structs = structs(&module, WriteOptions::default());
        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
            quote! {
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct Scalars {
                    pub a: u32,
                    pub b: i32,
                    pub c: f32,
                }
                impl Scalars {
                  pub fn new(a: u32, b: i32, c: f32) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct VectorsU32 {
                    pub a: [u32; 2],
                    pub b: [u32; 3],
                    pub c: [u32; 4],
                }
                impl VectorsU32 {
                  pub fn new(a: [u32; 2], b: [u32; 3], c: [u32; 4]) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct VectorsI32 {
                    pub a: [i32; 2],
                    pub b: [i32; 3],
                    pub c: [i32; 4],
                }
                impl VectorsI32 {
                  pub fn new(a: [i32; 2], b: [i32; 3], c: [i32; 4]) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct VectorsF32 {
                    pub a: [f32; 2],
                    pub b: [f32; 3],
                    pub c: [f32; 4],
                }
                impl VectorsF32 {
                  pub fn new(a: [f32; 2], b: [f32; 3], c: [f32; 4]) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct VectorsF64 {
                    pub a: [f64; 2],
                    pub b: [f64; 3],
                    pub c: [f64; 4],
                }
                impl VectorsF64 {
                  pub fn new(a: [f64; 2], b: [f64; 3], c: [f64; 4]) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct MatricesF32 {
                    pub a: [[f32; 4]; 4],
                    pub b: [[f32; 4]; 3],
                    pub c: [[f32; 4]; 2],
                    pub d: [[f32; 3]; 4],
                    pub e: [[f32; 3]; 3],
                    pub f: [[f32; 3]; 2],
                    pub g: [[f32; 2]; 4],
                    pub h: [[f32; 2]; 3],
                    pub i: [[f32; 2]; 2],
                }
                impl MatricesF32 {
                  pub fn new(
                      a: [[f32; 4]; 4],
                      b: [[f32; 4]; 3],
                      c: [[f32; 4]; 2],
                      d: [[f32; 3]; 4],
                      e: [[f32; 3]; 3],
                      f: [[f32; 3]; 2],
                      g: [[f32; 2]; 4],
                      h: [[f32; 2]; 3],
                      i: [[f32; 2]; 2],
                  ) -> Self {
                      Self { a, b, c, d, e, f, g, h, i }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct MatricesF64 {
                    pub a: [[f64; 4]; 4],
                    pub b: [[f64; 4]; 3],
                    pub c: [[f64; 4]; 2],
                    pub d: [[f64; 3]; 4],
                    pub e: [[f64; 3]; 3],
                    pub f: [[f64; 3]; 2],
                    pub g: [[f64; 2]; 4],
                    pub h: [[f64; 2]; 3],
                    pub i: [[f64; 2]; 2],
                }
                impl MatricesF64 {
                  pub fn new(
                      a: [[f64; 4]; 4],
                      b: [[f64; 4]; 3],
                      c: [[f64; 4]; 2],
                      d: [[f64; 3]; 4],
                      e: [[f64; 3]; 3],
                      f: [[f64; 3]; 2],
                      g: [[f64; 2]; 4],
                      h: [[f64; 2]; 3],
                      i: [[f64; 2]; 2],
                  ) -> Self {
                      Self { a, b, c, d, e, f, g, h, i }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct StaticArrays {
                    pub a: [u32; 5],
                    pub b: [f32; 3],
                    pub c: [[[f32; 4]; 4]; 512],
                }
                impl StaticArrays {
                  pub fn new(a: [u32; 5], b: [f32; 3], c: [[[f32; 4]; 4]; 512]) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct Nested {
                    pub a: MatricesF32,
                    pub b: MatricesF64,
                }
                impl Nested {
                  pub fn new(a: MatricesF32, b: MatricesF64) -> Self {
                      Self { a, b }
                  }
                }
            },
            actual
        );
    }

    #[test]
    fn write_all_structs_glam() {
        let source = indoc! {r#"
            struct Scalars {
                a: u32,
                b: i32,
                c: f32,
            };
            var<uniform> a: Scalars;

            struct VectorsU32 {
                a: vec2<u32>,
                b: vec3<u32>,
                c: vec4<u32>,
            };
            var<uniform> b: VectorsU32;

            struct VectorsI32 {
                a: vec2<i32>,
                b: vec3<i32>,
                c: vec4<i32>,
            };
            var<uniform> c: VectorsI32;

            struct VectorsF32 {
                a: vec2<f32>,
                b: vec3<f32>,
                c: vec4<f32>,
            };
            var<uniform> d: VectorsF32;

            struct VectorsF64 {
                a: vec2<f64>,
                b: vec3<f64>,
                c: vec4<f64>,
            };
            var<uniform> e: VectorsF64;

            struct MatricesF32 {
                a: mat4x4<f32>,
                b: mat4x3<f32>,
                c: mat4x2<f32>,
                d: mat3x4<f32>,
                e: mat3x3<f32>,
                f: mat3x2<f32>,
                g: mat2x4<f32>,
                h: mat2x3<f32>,
                i: mat2x2<f32>,
            };
            var<uniform> f: MatricesF32;

            struct MatricesF64 {
                a: mat4x4<f64>,
                b: mat4x3<f64>,
                c: mat4x2<f64>,
                d: mat3x4<f64>,
                e: mat3x3<f64>,
                f: mat3x2<f64>,
                g: mat2x4<f64>,
                h: mat2x3<f64>,
                i: mat2x2<f64>,
            };
            var<uniform> g: MatricesF64;

            struct StaticArrays {
                a: array<u32, 5>,
                b: array<f32, 3>,
                c: array<mat4x4<f32>, 512>,
            };
            var<uniform> h: StaticArrays;

            struct Nested {
                a: MatricesF32,
                b: MatricesF64
            }
            var<uniform> i: Nested;

            @fragment
            fn main() {}
        "#};

        let module = naga::front::wgsl::parse_str(source).unwrap();

        let structs = structs(
            &module,
            WriteOptions {
                matrix_vector_types: MatrixVectorTypes::Glam,
                ..Default::default()
            },
        );
        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
            quote! {
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
              pub struct Scalars {
                  pub a: u32,
                  pub b: i32,
                  pub c: f32,
              }
              impl Scalars {
                  pub fn new(a: u32, b: i32, c: f32) -> Self {
                      Self { a, b, c }
                  }
              }
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
              pub struct VectorsU32 {
                  pub a: glam::UVec2,
                  pub b: glam::UVec3,
                  pub c: glam::UVec4,
              }
              impl VectorsU32 {
                  pub fn new(a: glam::UVec2, b: glam::UVec3, c: glam::UVec4) -> Self {
                      Self { a, b, c }
                  }
              }
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
              pub struct VectorsI32 {
                  pub a: glam::IVec2,
                  pub b: glam::IVec3,
                  pub c: glam::IVec4,
              }
              impl VectorsI32 {
                  pub fn new(a: glam::IVec2, b: glam::IVec3, c: glam::IVec4) -> Self {
                      Self { a, b, c }
                  }
              }
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
              pub struct VectorsF32 {
                  pub a: glam::Vec2,
                  pub b: glam::Vec3,
                  pub c: glam::Vec4,
              }
              impl VectorsF32 {
                  pub fn new(a: glam::Vec2, b: glam::Vec3, c: glam::Vec4) -> Self {
                      Self { a, b, c }
                  }
              }
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
              pub struct VectorsF64 {
                  pub a: glam::DVec2,
                  pub b: glam::DVec3,
                  pub c: glam::DVec4,
              }
              impl VectorsF64 {
                  pub fn new(a: glam::DVec2, b: glam::DVec3, c: glam::DVec4) -> Self {
                      Self { a, b, c }
                  }
              }
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
              pub struct MatricesF32 {
                  pub a: glam::Mat4,
                  pub b: [[f32; 4]; 3],
                  pub c: [[f32; 4]; 2],
                  pub d: [[f32; 3]; 4],
                  pub e: glam::Mat3,
                  pub f: [[f32; 3]; 2],
                  pub g: [[f32; 2]; 4],
                  pub h: [[f32; 2]; 3],
                  pub i: [[f32; 2]; 2],
              }
              impl MatricesF32 {
                  pub fn new(
                      a: glam::Mat4,
                      b: [[f32; 4]; 3],
                      c: [[f32; 4]; 2],
                      d: [[f32; 3]; 4],
                      e: glam::Mat3,
                      f: [[f32; 3]; 2],
                      g: [[f32; 2]; 4],
                      h: [[f32; 2]; 3],
                      i: [[f32; 2]; 2],
                  ) -> Self {
                      Self { a, b, c, d, e, f, g, h, i }
                  }
              }
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
              pub struct MatricesF64 {
                  pub a: glam::DMat4,
                  pub b: [[f64; 4]; 3],
                  pub c: [[f64; 4]; 2],
                  pub d: [[f64; 3]; 4],
                  pub e: glam::DMat3,
                  pub f: [[f64; 3]; 2],
                  pub g: [[f64; 2]; 4],
                  pub h: [[f64; 2]; 3],
                  pub i: glam::DMat2,
              }
              impl MatricesF64 {
                  pub fn new(
                      a: glam::DMat4,
                      b: [[f64; 4]; 3],
                      c: [[f64; 4]; 2],
                      d: [[f64; 3]; 4],
                      e: glam::DMat3,
                      f: [[f64; 3]; 2],
                      g: [[f64; 2]; 4],
                      h: [[f64; 2]; 3],
                      i: glam::DMat2,
                  ) -> Self {
                      Self { a, b, c, d, e, f, g, h, i }
                  }
              }
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
              pub struct StaticArrays {
                  pub a: [u32; 5],
                  pub b: [f32; 3],
                  pub c: [glam::Mat4; 512],
              }
              impl StaticArrays {
                  pub fn new(a: [u32; 5], b: [f32; 3], c: [glam::Mat4; 512]) -> Self {
                      Self { a, b, c }
                  }
              }
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
              pub struct Nested {
                  pub a: MatricesF32,
                  pub b: MatricesF64,
              }
              impl Nested {
                  pub fn new(a: MatricesF32, b: MatricesF64) -> Self {
                      Self { a, b }
                  }
              }
            },
            actual
        );
    }

    #[test]
    fn write_all_structs_nalgebra() {
        let source = indoc! {r#"
            struct Scalars {
                a: u32,
                b: i32,
                c: f32,
            };
            var<uniform> a: Scalars;

            struct VectorsU32 {
                a: vec2<u32>,
                b: vec3<u32>,
                c: vec4<u32>,
            };
            var<uniform> b: VectorsU32;

            struct VectorsI32 {
                a: vec2<i32>,
                b: vec3<i32>,
                c: vec4<i32>,
            };
            var<uniform> c: VectorsI32;

            struct VectorsF32 {
                a: vec2<f32>,
                b: vec3<f32>,
                c: vec4<f32>,
            };
            var<uniform> d: VectorsF32;

            struct VectorsF64 {
                a: vec2<f64>,
                b: vec3<f64>,
                c: vec4<f64>,
            };
            var<uniform> e: VectorsF64;

            struct MatricesF32 {
                a: mat4x4<f32>,
                b: mat4x3<f32>,
                c: mat4x2<f32>,
                d: mat3x4<f32>,
                e: mat3x3<f32>,
                f: mat3x2<f32>,
                g: mat2x4<f32>,
                h: mat2x3<f32>,
                i: mat2x2<f32>,
            };
            var<uniform> f: MatricesF32;

            struct MatricesF64 {
                a: mat4x4<f64>,
                b: mat4x3<f64>,
                c: mat4x2<f64>,
                d: mat3x4<f64>,
                e: mat3x3<f64>,
                f: mat3x2<f64>,
                g: mat2x4<f64>,
                h: mat2x3<f64>,
                i: mat2x2<f64>,
            };
            var<uniform> g: MatricesF64;

            struct StaticArrays {
                a: array<u32, 5>,
                b: array<f32, 3>,
                c: array<mat4x4<f32>, 512>,
            };
            var<uniform> h: StaticArrays;

            struct Nested {
                a: MatricesF32,
                b: MatricesF64
            }
            var<uniform> i: Nested;

            @fragment
            fn main() {}
        "#};

        let module = naga::front::wgsl::parse_str(source).unwrap();

        let structs = structs(
            &module,
            WriteOptions {
                matrix_vector_types: MatrixVectorTypes::Nalgebra,
                ..Default::default()
            },
        );
        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
            quote! {
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct Scalars {
                    pub a: u32,
                    pub b: i32,
                    pub c: f32,
                }
                impl Scalars {
                  pub fn new(a: u32, b: i32, c: f32) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct VectorsU32 {
                    pub a: nalgebra::SVector<u32, 2>,
                    pub b: nalgebra::SVector<u32, 3>,
                    pub c: nalgebra::SVector<u32, 4>,
                }
                impl VectorsU32 {
                  pub fn new(
                    a: nalgebra::SVector<u32, 2>,
                    b: nalgebra::SVector<u32, 3>,
                    c: nalgebra::SVector<u32, 4>,
                  ) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct VectorsI32 {
                    pub a: nalgebra::SVector<i32, 2>,
                    pub b: nalgebra::SVector<i32, 3>,
                    pub c: nalgebra::SVector<i32, 4>,
                }
                impl VectorsI32 {
                  pub fn new(
                    a: nalgebra::SVector<i32, 2>,
                    b: nalgebra::SVector<i32, 3>,
                    c: nalgebra::SVector<i32, 4>,
                  ) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct VectorsF32 {
                    pub a: nalgebra::SVector<f32, 2>,
                    pub b: nalgebra::SVector<f32, 3>,
                    pub c: nalgebra::SVector<f32, 4>,
                }
                impl VectorsF32 {
                  pub fn new(
                    a: nalgebra::SVector<f32, 2>,
                    b: nalgebra::SVector<f32, 3>,
                    c: nalgebra::SVector<f32, 4>,
                  ) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct VectorsF64 {
                    pub a: nalgebra::SVector<f64, 2>,
                    pub b: nalgebra::SVector<f64, 3>,
                    pub c: nalgebra::SVector<f64, 4>,
                }
                impl VectorsF64 {
                  pub fn new(
                    a: nalgebra::SVector<f64, 2>,
                    b: nalgebra::SVector<f64, 3>,
                    c: nalgebra::SVector<f64, 4>,
                  ) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct MatricesF32 {
                    pub a: nalgebra::SMatrix<f32, 4, 4>,
                    pub b: nalgebra::SMatrix<f32, 3, 4>,
                    pub c: nalgebra::SMatrix<f32, 2, 4>,
                    pub d: nalgebra::SMatrix<f32, 4, 3>,
                    pub e: nalgebra::SMatrix<f32, 3, 3>,
                    pub f: nalgebra::SMatrix<f32, 2, 3>,
                    pub g: nalgebra::SMatrix<f32, 4, 2>,
                    pub h: nalgebra::SMatrix<f32, 3, 2>,
                    pub i: nalgebra::SMatrix<f32, 2, 2>,
                }
                impl MatricesF32 {
                  pub fn new(
                      a: nalgebra::SMatrix<f32, 4, 4>,
                      b: nalgebra::SMatrix<f32, 3, 4>,
                      c: nalgebra::SMatrix<f32, 2, 4>,
                      d: nalgebra::SMatrix<f32, 4, 3>,
                      e: nalgebra::SMatrix<f32, 3, 3>,
                      f: nalgebra::SMatrix<f32, 2, 3>,
                      g: nalgebra::SMatrix<f32, 4, 2>,
                      h: nalgebra::SMatrix<f32, 3, 2>,
                      i: nalgebra::SMatrix<f32, 2, 2>,
                  ) -> Self {
                      Self { a, b, c, d, e, f, g, h, i }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct MatricesF64 {
                    pub a: nalgebra::SMatrix<f64, 4, 4>,
                    pub b: nalgebra::SMatrix<f64, 3, 4>,
                    pub c: nalgebra::SMatrix<f64, 2, 4>,
                    pub d: nalgebra::SMatrix<f64, 4, 3>,
                    pub e: nalgebra::SMatrix<f64, 3, 3>,
                    pub f: nalgebra::SMatrix<f64, 2, 3>,
                    pub g: nalgebra::SMatrix<f64, 4, 2>,
                    pub h: nalgebra::SMatrix<f64, 3, 2>,
                    pub i: nalgebra::SMatrix<f64, 2, 2>,
                }
                impl MatricesF64 {
                  pub fn new(
                      a: nalgebra::SMatrix<f64, 4, 4>,
                      b: nalgebra::SMatrix<f64, 3, 4>,
                      c: nalgebra::SMatrix<f64, 2, 4>,
                      d: nalgebra::SMatrix<f64, 4, 3>,
                      e: nalgebra::SMatrix<f64, 3, 3>,
                      f: nalgebra::SMatrix<f64, 2, 3>,
                      g: nalgebra::SMatrix<f64, 4, 2>,
                      h: nalgebra::SMatrix<f64, 3, 2>,
                      i: nalgebra::SMatrix<f64, 2, 2>,
                  ) -> Self {
                      Self { a, b, c, d, e, f, g, h, i }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct StaticArrays {
                    pub a: [u32; 5],
                    pub b: [f32; 3],
                    pub c: [nalgebra::SMatrix<f32, 4, 4>; 512],
                }
                impl StaticArrays {
                  pub fn new(
                    a: [u32; 5],
                    b: [f32; 3],
                    c: [nalgebra::SMatrix<f32, 4, 4>; 512],
                  ) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct Nested {
                    pub a: MatricesF32,
                    pub b: MatricesF64,
                }
                impl Nested {
                  pub fn new(a: MatricesF32, b: MatricesF64) -> Self {
                      Self { a, b }
                  }
                }
            },
            actual
        );
    }

    #[test]
    fn write_all_structs_encase() {
        let source = indoc! {r#"
            struct Input0 {
                a: u32,
                b: i32,
                c: f32,
            };

            struct Nested {
                a: Input0,
                b: f32
            }

            var<uniform> a: Input0;
            var<storage, read> b: Nested;

            @fragment
            fn main() {}
        "#};

        let module = naga::front::wgsl::parse_str(source).unwrap();

        let structs = structs(
            &module,
            WriteOptions {
                serialization_strategy: ShaderSerializationStrategy::Encase,
                derive_serde: false,
                matrix_vector_types: MatrixVectorTypes::Rust,
            },
        );
        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
            quote! {
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct Input0 {
                    pub a: u32,
                    pub b: i32,
                    pub c: f32,
                }
                impl Input0 {
                  pub fn new(a: u32, b: i32, c: f32) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct Nested {
                    pub a: Input0,
                    pub b: f32,
                }
                impl Nested {
                  pub fn new(a: Input0, b: f32) -> Self {
                      Self { a, b }
                  }
                }
            },
            actual
        );
    }

    #[test]
    fn write_all_structs_serde_encase() {
        let source = indoc! {r#"
            struct Input0 {
                a: u32,
                b: i32,
                c: f32,
            };

            struct Nested {
                a: Input0,
                b: f32
            }

            var<workgroup> a: Input0;
            var<uniform> b: Nested;

            @compute
            @workgroup_size(64)
            fn main() {}
        "#};

        let module = naga::front::wgsl::parse_str(source).unwrap();

        let structs = structs(
            &module,
            WriteOptions {
                serialization_strategy: ShaderSerializationStrategy::Encase,
                derive_serde: true,
                matrix_vector_types: MatrixVectorTypes::Rust,
            },
        );
        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
            quote! {
                #[repr(C)]
                #[derive(
                    Debug,
                    PartialEq,
                    Clone,
                    Copy,
                    encase::ShaderType,
                    serde::Serialize,
                    serde::Deserialize
                )]
                pub struct Input0 {
                    pub a: u32,
                    pub b: i32,
                    pub c: f32,
                }
                impl Input0 {
                  pub fn new(a: u32, b: i32, c: f32) -> Self {
                      Self { a, b, c }
                  }
                }
                #[repr(C)]
                #[derive(
                    Debug,
                    PartialEq,
                    Clone,
                    Copy,
                    encase::ShaderType,
                    serde::Serialize,
                    serde::Deserialize
                )]
                pub struct Nested {
                    pub a: Input0,
                    pub b: f32,
                }
                impl Nested {
                  pub fn new(a: Input0, b: f32) -> Self {
                      Self { a, b }
                  }
                }
            },
            actual
        );
    }

    #[test]
    fn write_all_structs_skip_stage_outputs() {
        let source = indoc! {r#"
            struct Input0 {
                a: u32,
                b: i32,
                c: f32,
            };

            struct Output0 {
                a: f32
            }

            struct Unused {
                a: vec3<f32>
            }

            @fragment
            fn main(in: Input0) -> Output0 {
                var out: Output0;
                return out;
            }
        "#};

        let module = naga::front::wgsl::parse_str(source).unwrap();

        let structs = structs(
            &module,
            WriteOptions {
                serialization_strategy: ShaderSerializationStrategy::Bytemuck,
                derive_serde: false,
                matrix_vector_types: MatrixVectorTypes::Rust,
            },
        );
        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
            quote! {
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy)]
                pub struct Input0 {
                    pub a: u32,
                    pub b: i32,
                    pub c: f32,
                }
                impl Input0 {
                  pub fn new(a: u32, b: i32, c: f32) -> Self {
                      Self { a, b, c }
                  }
                }
                unsafe impl bytemuck::Zeroable for Input0 {}
                unsafe impl bytemuck::Pod for Input0 {}
            },
            actual
        );
    }

    #[test]
    fn write_all_structs_bytemuck_skip_input_layout_validation() {
        // Structs used only for vertex inputs don't require layout validation.
        // Correctly specifying the offsets is handled by the buffer layout itself.
        let source = indoc! {r#"
            struct Input0 {
                a: u32,
                b: i32,
                c: f32,
            };

            @vertex
            fn main(input: Input0) -> vec4<f32> {
                return vec4(0.0);
            }
        "#};

        let module = naga::front::wgsl::parse_str(source).unwrap();

        let structs = structs(
            &module,
            WriteOptions {
                serialization_strategy: ShaderSerializationStrategy::Bytemuck,
                derive_serde: false,
                matrix_vector_types: MatrixVectorTypes::Rust,
            },
        );
        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
            quote! {
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy)]
                pub struct Input0 {
                    pub a: u32,
                    pub b: i32,
                    pub c: f32,
                }
                impl Input0 {
                    pub fn new(a: u32, b: i32, c: f32) -> Self {
                        Self { a, b, c }
                    }
                }
                unsafe impl bytemuck::Zeroable for Input0 {}
                unsafe impl bytemuck::Pod for Input0 {}
            },
            actual
        );
    }

    #[test]
    fn write_all_structs_bytemuck_input_layout_validation() {
        // The struct is also used with a storage buffer and should be validated.
        let source = indoc! {r#"
            struct Input0 {
                @size(8)
                a: u32,
                b: i32,
                @align(32)
                c: f32,
            };

            var<storage, read_write> test: Input0;

            struct Outer {
                inner: Inner
            }

            struct Inner {
                a: f32
            }

            var<storage, read_write> test2: array<Outer>;

            @vertex
            fn main(input: Input0) -> vec4<f32> {
                return vec4(0.0);
            }
        "#};

        let module = naga::front::wgsl::parse_str(source).unwrap();

        let structs = structs(
            &module,
            WriteOptions {
                serialization_strategy: ShaderSerializationStrategy::Bytemuck,
                derive_serde: false,
                matrix_vector_types: MatrixVectorTypes::Rust,
            },
        );
        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
            quote! {
              #[repr(C, align(4))]
              #[derive(Debug, PartialEq, Clone, Copy)]
              pub struct Input0 {
                  /// size: 4, offset: 0x0, type: u32
                  pub a: u32,
                  pub _pada: [u8; 8 - 0 - core::mem::size_of::<u32>()],
                  /// size: 4, offset: 0x8, type: i32
                  pub b: i32,
                  pub _padb: [u8; 32 - 8 - core::mem::size_of::<i32>()],
                  /// size: 4, offset: 0x20, type: f32
                  pub c: f32,
                  pub _padc: [u8; 64 - 32 - core::mem::size_of::<f32>()],
              }
              impl Input0 {
                  pub fn new(a: u32, b: i32, c: f32) -> Self {
                      Self {
                          a,
                          _pada: [0; 8 - 0 - core::mem::size_of::<u32>()],
                          b,
                          _padb: [0; 32 - 8 - core::mem::size_of::<i32>()],
                          c,
                          _padc: [0; 64 - 32 - core::mem::size_of::<f32>()],
                      }
                  }
              }
              unsafe impl bytemuck::Zeroable for Input0 {}
              unsafe impl bytemuck::Pod for Input0 {}
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy)]
              pub struct Input0Init {
                  pub a: u32,
                  pub b: i32,
                  pub c: f32,
              }
              impl Input0Init {
                  pub const fn const_into(&self) -> Input0 {
                      let init = self;
                      Input0 {
                          a: init.a,
                          _pada: [0; 8 - 0 - core::mem::size_of::<u32>()],
                          b: init.b,
                          _padb: [0; 32 - 8 - core::mem::size_of::<i32>()],
                          c: init.c,
                          _padc: [0; 64 - 32 - core::mem::size_of::<f32>()],
                      }
                  }
              }
              impl From<Input0Init> for Input0 {
                  fn from(init: Input0Init) -> Self {
                      Self {
                          a: init.a,
                          _pada: [0; 8 - 0 - core::mem::size_of::<u32>()],
                          b: init.b,
                          _padb: [0; 32 - 8 - core::mem::size_of::<i32>()],
                          c: init.c,
                          _padc: [0; 64 - 32 - core::mem::size_of::<f32>()],
                      }
                  }
              }
              const _: () = assert!(
                  std::mem::size_of:: < Input0 > () == 64, "size of Input0 does not match WGSL"
              );
              const _: () = assert!(
                  memoffset::offset_of!(Input0, a) == 0, "offset of Input0.a does not match WGSL"
              );
              const _: () = assert!(
                  memoffset::offset_of!(Input0, b) == 8, "offset of Input0.b does not match WGSL"
              );
              const _: () = assert!(
                  memoffset::offset_of!(Input0, c) == 32, "offset of Input0.c does not match WGSL"
              );
              #[repr(C, align(4))]
              #[derive(Debug, PartialEq, Clone, Copy)]
              pub struct Inner {
                  /// size: 4, offset: 0x0, type: f32
                  pub a: f32,
                  pub _pada: [u8; 4 - 0 - core::mem::size_of::<f32>()],
              }
              impl Inner {
                  pub fn new(a: f32) -> Self {
                      Self {
                          a,
                          _pada: [0; 4 - 0 - core::mem::size_of::<f32>()],
                      }
                  }
              }
              unsafe impl bytemuck::Zeroable for Inner {}
              unsafe impl bytemuck::Pod for Inner {}
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy)]
              pub struct InnerInit {
                  pub a: f32,
              }
              impl InnerInit {
                  pub const fn const_into(&self) -> Inner {
                      let init = self;
                      Inner {
                          a: init.a,
                          _pada: [0; 4 - 0 - core::mem::size_of::<f32>()],
                      }
                  }
              }
              impl From<InnerInit> for Inner {
                  fn from(init: InnerInit) -> Self {
                      Self {
                          a: init.a,
                          _pada: [0; 4 - 0 - core::mem::size_of::<f32>()],
                      }
                  }
              }
              const _: () = assert!(
                  std::mem::size_of:: < Inner > () == 4, "size of Inner does not match WGSL"
              );
              const _: () = assert!(
                  memoffset::offset_of!(Inner, a) == 0, "offset of Inner.a does not match WGSL"
              );
              #[repr(C, align(4))]
              #[derive(Debug, PartialEq, Clone, Copy)]
              pub struct Outer {
                  /// size: 4, offset: 0x0, type: struct
                  pub inner: Inner,
                  pub _padinner: [u8; 4 - 0 - core::mem::size_of::<Inner>()],
              }
              impl Outer {
                  pub fn new(inner: Inner) -> Self {
                      Self {
                          inner,
                          _padinner: [0; 4 - 0 - core::mem::size_of::<Inner>()],
                      }
                  }
              }
              unsafe impl bytemuck::Zeroable for Outer {}
              unsafe impl bytemuck::Pod for Outer {}
              #[repr(C)]
              #[derive(Debug, PartialEq, Clone, Copy)]
              pub struct OuterInit {
                  pub inner: Inner,
              }
              impl OuterInit {
                  pub const fn const_into(&self) -> Outer {
                      let init = self;
                      Outer {
                          inner: init.inner,
                          _padinner: [0; 4 - 0 - core::mem::size_of::<Inner>()],
                      }
                  }
              }
              impl From<OuterInit> for Outer {
                  fn from(init: OuterInit) -> Self {
                      Self {
                          inner: init.inner,
                          _padinner: [0; 4 - 0 - core::mem::size_of::<Inner>()],
                      }
                  }
              }
              const _: () = assert!(
                  std::mem::size_of:: < Outer > () == 4, "size of Outer does not match WGSL"
              );
              const _: () = assert!(
                  memoffset::offset_of!(Outer, inner) == 0, "offset of Outer.inner does not match WGSL"
              );
            },
            actual
        );
    }

    #[test]
    fn write_atomic_types() {
        let source = indoc! {r#"
            struct Atomics {
                num: atomic<u32>,
                numi: atomic<i32>,
            };

            @group(0) @binding(0)
            var <storage, read_write> atomics:Atomics;
        "#};

        let module = naga::front::wgsl::parse_str(source).unwrap();

        let structs = structs(
            &module,
            WriteOptions {
                matrix_vector_types: MatrixVectorTypes::Nalgebra,
                ..Default::default()
            },
        );
        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
            quote! {
                #[repr(C)]
                #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                pub struct Atomics {
                    pub num: u32,
                    pub numi: i32,
                }
                impl Atomics {
                  pub fn new(num: u32, numi: i32) -> Self {
                      Self { num, numi }
                  }
                }
            },
            actual
        );
    }

    fn runtime_sized_array_module() -> naga::Module {
        let source = indoc! {r#"
            struct RtsStruct {
                other_data: i32,
                the_array: array<u32>,
            };

            @group(0) @binding(0)
            var <storage, read_write> rts:RtsStruct;
        "#};
        naga::front::wgsl::parse_str(source).unwrap()
    }

    #[test]
    fn write_runtime_sized_array() {
        let module = runtime_sized_array_module();

        let structs = structs(
            &module,
            WriteOptions {
                serialization_strategy: ShaderSerializationStrategy::Encase,
                ..Default::default()
            },
        );
        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
            quote! {
                #[derive(Debug, PartialEq, Clone, encase::ShaderType)]
                pub struct RtsStruct {
                    pub other_data: i32,
                    #[size(runtime)]
                    pub the_array: Vec<u32>,
                }
                impl RtsStruct {
                  pub fn new(other_data: i32, the_array: Vec<u32>) -> Self {
                      Self { other_data, the_array }
                  }
                }
            },
            actual
        );
    }

    #[test]
    fn write_runtime_sized_array_bytemuck() {
        let module = runtime_sized_array_module();

        let structs = structs(
            &module,
            WriteOptions {
                serialization_strategy: ShaderSerializationStrategy::Bytemuck,
                ..Default::default()
            },
        );

        let actual = quote!(#(#structs)*);

        assert_tokens_eq!(
          quote! {
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct RtsStruct<const N: usize> {
                /// size: 4, offset: 0x0, type: i32
                pub other_data: i32,
                pub _padother_data: [u8; 4 - 0 - core::mem::size_of::<i32>()],
                /// size: 4, offset: 0x4, type: array<u32>
                pub the_array: [u32; N]
            }
            impl<const N:usize> RtsStruct<N> {
                pub fn new(other_data: i32, the_array: [u32; N]) -> Self {
                    Self {
                        other_data,
                        _padother_data: [0; 4 - 0 - core::mem::size_of::<i32>()],
                        the_array
                    }
                }
            }
            unsafe impl<const N: usize> bytemuck::Zeroable for RtsStruct<N> {}
            unsafe impl<const N: usize> bytemuck::Pod for RtsStruct<N> {}
            #[repr(C)]
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct RtsStructInit<const N: usize> {
                pub other_data: i32,
                pub the_array: [u32; N],
            }
            impl<const N: usize> RtsStructInit<N> {
                pub const fn const_into(&self) -> RtsStruct<N> {
                    let init = self;
                    RtsStruct {
                        other_data: init.other_data,
                        _padother_data: [0; 4 - 0 - core::mem::size_of::<i32>()],
                        the_array: init.the_array,
                    }
                }
            }
            impl<const N: usize> From<RtsStructInit<N>> for RtsStruct<N> {
                fn from(init: RtsStructInit<N>) -> Self {
                    Self {
                        other_data: init.other_data,
                        _padother_data: [0; 4 - 0 - core::mem::size_of::<i32>()],
                        the_array: init.the_array,
                    }
                }
            }
            const _: () = assert!(
                std::mem::size_of:: < RtsStruct<1> > () == 8, "size of RtsStruct does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(RtsStruct<1>, other_data) == 0,
                "offset of RtsStruct.other_data does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(RtsStruct<1>, the_array) == 4,
                "offset of RtsStruct.the_array does not match WGSL"
            );
          },
          actual
        )
    }

    #[test]
    #[should_panic]
    fn write_runtime_sized_array_not_last_field() {
        let source = indoc! {r#"
            struct RtsStruct {
                other_data: i32,
                the_array: array<u32>,
                more_data: i32,
            };

            @group(0) @binding(0)
            var <storage, read_write> rts:RtsStruct;
        "#};
        let module = naga::front::wgsl::parse_str(source).unwrap();

        let _structs = structs(
            &module,
            WriteOptions {
                serialization_strategy: ShaderSerializationStrategy::Encase,
                ..Default::default()
            },
        );
    }

    #[test]
    fn write_nonpower_of_2_mats_for_bytemuck_option() {
      let source = indoc! {r#"
        struct UniformsData {
          a: mat3x3<f32>,
        }

        @group(0) @binding(0)
            var <uniform> un:UniformsData;
      "#};

      let module = naga::front::wgsl::parse_str(source).unwrap();

      let structs = structs(
          &module,
          WriteOptions {
              serialization_strategy: ShaderSerializationStrategy::Bytemuck,
              ..Default::default()
          },
      );
      let actual = quote!(#(#structs)*);

      assert_tokens_eq!(
          quote! {
            #[repr(C, align(16))]
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct UniformsData {
                /// size: 48, offset: 0x0, type: mat3x3<f32>
                pub a: [[f32; 3]; 4],
                pub _pada: [u8; 48 - 0 - core::mem::size_of::<[[f32; 3]; 4]>()],
            }
            impl UniformsData {
                pub fn new(a: [[f32; 3]; 4]) -> Self {
                    Self {
                        a,
                        _pada: [0; 48 - 0 - core::mem::size_of::<[[f32; 3]; 4]>()],
                    }
                }
            }
            unsafe impl bytemuck::Zeroable for UniformsData {}
            unsafe impl bytemuck::Pod for UniformsData {}
            #[repr(C)]
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct UniformsDataInit {
                pub a: [[f32; 3]; 4],
            }
            impl UniformsDataInit {
                pub const fn const_into(&self) -> UniformsData {
                    let init = self;
                    UniformsData {
                        a: init.a,
                        _pada: [0; 48 - 0 - core::mem::size_of::<[[f32; 3]; 4]>()],
                    }
                }
            }
            impl From<UniformsDataInit> for UniformsData {
                fn from(init: UniformsDataInit) -> Self {
                    Self {
                        a: init.a,
                        _pada: [0; 48 - 0 - core::mem::size_of::<[[f32; 3]; 4]>()],
                    }
                }
            }
            const _: () = assert!(
                std::mem::size_of:: < UniformsData > () == 48,
                "size of UniformsData does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(UniformsData, a) == 0,
                "offset of UniformsData.a does not match WGSL"
            );
          },
          actual
      );
    }

    #[test]
    fn write_nonpower_of_2_mats_for_bytemuck_glam_option() {
      let source = indoc! {r#"
        struct UniformsData {
          centered_mvp: mat3x3<f32>,
        }

        @group(0) @binding(0)
            var <uniform> un:UniformsData;
      "#};

      let module = naga::front::wgsl::parse_str(source).unwrap();

      let structs = structs(
          &module,
          WriteOptions {
              serialization_strategy: ShaderSerializationStrategy::Bytemuck,
              matrix_vector_types: MatrixVectorTypes::Glam,
              ..Default::default()
          },
      );
      let actual = quote!(#(#structs)*);

      assert_tokens_eq!(
          quote! {
            #[repr(C, align(16))]
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct UniformsData {
                /// size: 48, offset: 0x0, type: mat3x3<f32>
                pub centered_mvp: glam::Mat3A,
                pub _padcentered_mvp: [u8; 48 - 0 - core::mem::size_of::<glam::Mat3A>()],
            }
            impl UniformsData {
                pub fn new(centered_mvp: glam::Mat3A) -> Self {
                    Self { 
                      centered_mvp,
                      _padcentered_mvp: [0; 48 - 0 - core::mem::size_of::<glam::Mat3A>()],
                    }
                }
            }
            unsafe impl bytemuck::Zeroable for UniformsData {}
            unsafe impl bytemuck::Pod for UniformsData {}
            #[repr(C)]
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct UniformsDataInit {
                pub centered_mvp: glam::Mat3A,
            }
            impl UniformsDataInit {
                pub const fn const_into(&self) -> UniformsData {
                    let init = self;
                    UniformsData {
                        centered_mvp: init.centered_mvp,
                        _padcentered_mvp: [0; 48 - 0 - core::mem::size_of::<glam::Mat3A>()],
                    }
                }
            }
            impl From<UniformsDataInit> for UniformsData {
                fn from(init: UniformsDataInit) -> Self {
                    Self {
                        centered_mvp: init.centered_mvp,
                        _padcentered_mvp: [0; 48 - 0 - core::mem::size_of::<glam::Mat3A>()],
                    }
                }
            }
            const _: () = assert!(
                std::mem::size_of:: < UniformsData > () == 48,
                "size of UniformsData does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(UniformsData, centered_mvp) == 0,
                "offset of UniformsData.centered_mvp does not match WGSL"
            );
          },
          actual
      );
    }

    #[test]
    fn write_nonpower_of_2_mats() {
      let source = indoc! {r#"
          struct MatricesF32 {
            a: mat4x4<f32>,
            b: mat4x3<f32>,
            c: mat4x2<f32>,
            d: mat3x4<f32>,
        };
        @group(0) @binding(0)
        var<uniform> f: MatricesF32;
      "#};

      let module = naga::front::wgsl::parse_str(source).unwrap();

      let structs = structs(
          &module,
          WriteOptions {
              serialization_strategy: ShaderSerializationStrategy::Bytemuck,
              matrix_vector_types: MatrixVectorTypes::Rust,
              ..Default::default()
          },
      );
      let actual = quote!(#(#structs)*);

      assert_tokens_eq!(
          quote! {
            #[repr(C, align(16))]
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct MatricesF32 {
                /// size: 64, offset: 0x0, type: mat4x4<f32>
                pub a: [[f32; 4]; 4],
                pub _pada: [u8; 64 - 0 - core::mem::size_of::<[[f32; 4]; 4]>()],
                /// size: 64, offset: 0x40, type: mat4x3<f32>
                pub b: [[f32; 4]; 4],
                pub _padb: [u8; 128 - 64 - core::mem::size_of::<[[f32; 4]; 4]>()],
                /// size: 32, offset: 0x80, type: mat4x2<f32>
                pub c: [[f32; 4]; 2],
                pub _padc: [u8; 160 - 128 - core::mem::size_of::<[[f32; 4]; 2]>()],
                /// size: 48, offset: 0xA0, type: mat3x4<f32>
                pub d: [[f32; 3]; 4],
                pub _padd: [u8; 208 - 160 - core::mem::size_of::<[[f32; 3]; 4]>()],
            }
            impl MatricesF32 {
                pub fn new(
                    a: [[f32; 4]; 4],
                    b: [[f32; 4]; 4],
                    c: [[f32; 4]; 2],
                    d: [[f32; 3]; 4],
                ) -> Self {
                    Self {
                        a,
                        _pada: [0; 64 - 0 - core::mem::size_of::<[[f32; 4]; 4]>()],
                        b,
                        _padb: [0; 128 - 64 - core::mem::size_of::<[[f32; 4]; 4]>()],
                        c,
                        _padc: [0; 160 - 128 - core::mem::size_of::<[[f32; 4]; 2]>()],
                        d,
                        _padd: [0; 208 - 160 - core::mem::size_of::<[[f32; 3]; 4]>()],
                    }
                }
            }
            unsafe impl bytemuck::Zeroable for MatricesF32 {}
            unsafe impl bytemuck::Pod for MatricesF32 {}
            #[repr(C)]
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct MatricesF32Init {
                pub a: [[f32; 4]; 4],
                pub b: [[f32; 4]; 4],
                pub c: [[f32; 4]; 2],
                pub d: [[f32; 3]; 4],
            }
            impl MatricesF32Init {
                pub const fn const_into(&self) -> MatricesF32 {
                    let init = self;
                    MatricesF32 {
                        a: init.a,
                        _pada: [0; 64 - 0 - core::mem::size_of::<[[f32; 4]; 4]>()],
                        b: init.b,
                        _padb: [0; 128 - 64 - core::mem::size_of::<[[f32; 4]; 4]>()],
                        c: init.c,
                        _padc: [0; 160 - 128 - core::mem::size_of::<[[f32; 4]; 2]>()],
                        d: init.d,
                        _padd: [0; 208 - 160 - core::mem::size_of::<[[f32; 3]; 4]>()],
                    }
                }
            }
            impl From<MatricesF32Init> for MatricesF32 {
                fn from(init: MatricesF32Init) -> Self {
                    Self {
                        a: init.a,
                        _pada: [0; 64 - 0 - core::mem::size_of::<[[f32; 4]; 4]>()],
                        b: init.b,
                        _padb: [0; 128 - 64 - core::mem::size_of::<[[f32; 4]; 4]>()],
                        c: init.c,
                        _padc: [0; 160 - 128 - core::mem::size_of::<[[f32; 4]; 2]>()],
                        d: init.d,
                        _padd: [0; 208 - 160 - core::mem::size_of::<[[f32; 3]; 4]>()],
                    }
                }
            }
            const _: () = assert!(
                std::mem::size_of:: < MatricesF32 > () == 208,
                "size of MatricesF32 does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(MatricesF32, a) == 0,
                "offset of MatricesF32.a does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(MatricesF32, b) == 64,
                "offset of MatricesF32.b does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(MatricesF32, c) == 128,
                "offset of MatricesF32.c does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(MatricesF32, d) == 160,
                "offset of MatricesF32.d does not match WGSL"
            );
          },
          actual
      );
    }
}
