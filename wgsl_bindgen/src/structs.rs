use std::collections::HashSet;

use naga::{Handle, Type};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Index};

use crate::{wgsl::rust_type, ShaderSerializationStrategy, WriteOptions};

struct StructMemberPaddingInfo {
  pad_name: Ident,
  pad_size: TokenStream,
}

struct StructMemberEntry<'a> {
  name: Ident,
  member: &'a naga::StructMember,
  ty: &'a naga::Type,
  q_ty: syn::Type,
  attr: Option<TokenStream>,
  padding: Option<StructMemberPaddingInfo>
}

struct StructInfo<'a> {
  name: Ident,
  members: Vec<StructMemberEntry<'a>>,
  is_host_shareable: bool,
  module: &'a naga::Module,
  options: WriteOptions
}

impl<'a> StructInfo<'a> {

  fn should_pad(&self) -> bool {
    self.options.serialization_strategy == ShaderSerializationStrategy::Bytemuck
    && self.is_host_shareable
  }

  fn use_padding(&self) -> bool {
    self.members.iter().any(|StructMemberEntry { padding, .. }| padding.is_some())
  }

  fn build_init_struct(&self) -> TokenStream {
    if !self.should_pad() || !self.use_padding() {
      return quote!();
    }

    let struct_name = self.name.clone();
    let init_struct_name = Ident::new(&format!("{}Init", struct_name.to_string()), Span::call_site());
    let mut init_struct_members = vec![]; 
    let mut mem_assignments = vec![];
    
    for StructMemberEntry { name: member_name, q_ty: member_ty, padding, .. } in self.members.iter() {
      init_struct_members.push(quote!(pub #member_name: #member_ty));
      mem_assignments.push(quote!(#member_name: init.#member_name));

      if let Some(StructMemberPaddingInfo { pad_name, pad_size, .. }) = padding {
        mem_assignments.push(quote!(#pad_name: [0; #pad_size]));
      }
    };


    quote! {
      #[repr(C)]
      #[derive(Debug, Copy, Clone, PartialEq)]
      pub struct #init_struct_name {
        #(#init_struct_members),*
      }

      impl #init_struct_name {
        pub const fn const_into(&self) -> #struct_name {
          let init = self;
          #struct_name {
            #(#mem_assignments),*
          }
        }
      }

      impl From<#init_struct_name> for #struct_name {
        fn from(init: #init_struct_name) -> Self {
          Self {
            #(#mem_assignments),*
          }
        }
      }
    }
  }

  fn build_new_fn(&self) -> TokenStream {
    let struct_name = self.name.clone();

    let mut non_padding_members = Vec::new();
    let mut member_assignments = Vec::new();

    for StructMemberEntry {name: member_name, q_ty: member_ty, padding, ..} in &self.members {
      non_padding_members.push(quote!(#member_name: #member_ty));
      member_assignments.push(quote!(#member_name));

      if let Some(StructMemberPaddingInfo { pad_name, pad_size, .. }) = padding {
        member_assignments.push(quote!(#pad_name: [0; #pad_size]));
      }
    }

    quote! {
      impl #struct_name {
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

  fn build_struct_fields(&self) -> Vec<TokenStream> {
    let ctx = self.module.to_ctx();
    let members = self.members
      .iter()
      .map(|StructMemberEntry { name, q_ty, padding, attr, member, ty }| {

        let doc = if self.use_padding() {
          let offset = member.offset;
          let size = ty.inner.size(ctx);
          let doc = format!(" Offset: 0x{:x}, Size: 0x{:x}", offset, size);
          quote!(#[doc = #doc])
        } else {
          quote!()
        };

        let mut tokstream_members = Vec::new();

        let field = 
          match attr {
            Some(attr) => {
            quote!{
              #doc
              #attr
              pub #name: #q_ty
            }
          }, 
          _ => {
            quote!{
              #doc
              pub #name: #q_ty
            }
          }
        };

        tokstream_members.push(field);

        if self.options.serialization_strategy == ShaderSerializationStrategy::Bytemuck {
          if let Some(StructMemberPaddingInfo { pad_name, pad_size: q_pad_size, .. }) = padding {
            tokstream_members.push(quote!(#pad_name: [u8; #q_pad_size]))
          } 
        }

        quote!(#(#tokstream_members), *)
      })
      .collect();

      members
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

    let assert_member_offsets: Vec<_> = members
        .iter()
        .map(|m| {
            let name = Ident::new(m.name.as_ref().unwrap(), Span::call_site());
            let rust_offset = quote!(memoffset::offset_of!(#struct_name, #name));

            let wgsl_offset = Index::from(m.offset as usize);

            let assert_text = format!(
                "offset of {}.{} does not match WGSL",
                t.name.as_ref().unwrap(),
                m.name.as_ref().unwrap()
            );
            quote! {
                const _: () = assert!(#rust_offset == #wgsl_offset, #assert_text);
            }
        })
        .collect();

    let layout = layouter[t_handle];

    // TODO: Does the Rust alignment matter if it's copied to a buffer anyway?
    let struct_size = Index::from(layout.size as usize);
    let assert_size_text = format!("size of {} does not match WGSL", t.name.as_ref().unwrap());
    let assert_size = quote! {
        const _: () = assert!(std::mem::size_of::<#struct_name>() == #struct_size, #assert_size_text);
    };

    // Assume types used in global variables are host shareable and require validation.
    // This includes storage, uniform, and workgroup variables.
    // This also means types that are never used will not be validated.
    // Structs used only for vertex inputs do not require validation on desktop platforms.
    // Vertex input layout is handled already by setting the attribute offsets and types.
    // This allows vertex input field types without padding like vec3 for positions.
    let is_host_shareable = global_variable_types.contains(&t_handle);

    let has_rts_array = struct_has_rts_array_member(members, module);
    let should_generate_padding = is_host_shareable && options.serialization_strategy == ShaderSerializationStrategy::Bytemuck;
    let struct_members_entries = struct_members(members, module, options, layout.size as usize, should_generate_padding);
    let mut derives = Vec::new();

    derives.push(quote!(Debug));
    if !has_rts_array {
        derives.push(quote!(Copy));
    }
    derives.push(quote!(Clone));
    derives.push(quote!(PartialEq));

    match options.serialization_strategy {
      ShaderSerializationStrategy::Bytemuck => {
        if has_rts_array {
            panic!("Runtime-sized array fields are not supported in options.derive_bytemuck mode");
        }
        derives.push(quote!(bytemuck::Pod));
        derives.push(quote!(bytemuck::Zeroable));
      },
      ShaderSerializationStrategy::Encase => {
        derives.push(quote!(encase::ShaderType));
      }
    }
    if options.derive_serde {
        derives.push(quote!(serde::Serialize));
        derives.push(quote!(serde::Deserialize));
    }

    let assert_layout = if options.serialization_strategy == ShaderSerializationStrategy::Bytemuck && is_host_shareable {
        // Assert that the Rust layout matches the WGSL layout.
        // Enable for bytemuck since it uses the Rust struct's memory layout.
        quote! {
            #assert_size
            #(#assert_member_offsets)*
        }
    } else {
        quote!()
    };

    let repr_c = if !has_rts_array {
      if should_generate_padding {
        quote!(#[repr(C, packed)])
      }
      else {
        quote!(#[repr(C)])
      }
    } else {
        quote!()
    };

    let struct_info = StructInfo {
        name: struct_name.clone(),
        members: struct_members_entries,
        is_host_shareable, 
        module,
        options
    };

    let members = struct_info.build_struct_fields();
    let struct_new_fn = struct_info.build_new_fn();
    let init_struct = struct_info.build_init_struct();

    quote! {
        #repr_c
        #[derive(#(#derives),*)]
        pub struct #struct_name {
            #(#members),*
        }
        
        #struct_new_fn

        #init_struct

        #assert_layout
    }
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
    should_generate_padding: bool,
) -> Vec<StructMemberEntry<'a>> {
    let mut member_entries: Vec<StructMemberEntry<'a>> = vec![];

    members
        .iter()
        .enumerate()
        .for_each(|(index, member)| {
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
                let element_type =
                    rust_type(module, &module.types[*base], options.matrix_vector_types);
                let member_type = syn::Type::Verbatim(quote!(Vec<#element_type>));

                member_entries.push(StructMemberEntry {
                  name: member_name.clone(), 
                  member,
                  ty,
                  q_ty: member_type.clone(), 
                  attr: Some(quote!(#[size(runtime)])),
                  padding: None
                });
            } else {
              let member_type = syn::Type::Verbatim(rust_type(module, ty, options.matrix_vector_types));

              if !should_generate_padding {
                member_entries.push(
                  StructMemberEntry {
                    name: member_name.clone(), 
                    member,
                    ty,
                    q_ty: member_type, 
                    attr: None,
                    padding: None
                  }
                );
              } else {
                let current_offset = Index::from(member.offset as usize);

                let next_offset = if index == members.len() - 1 {
                  required_struct_size
                } else {
                  members[index + 1].offset as usize
                };
                let next_offset = Index::from(next_offset);

                let pad_name = Ident::new(&format!("_pad_{}", member_name), Span::call_site());
                let pad_size = quote!(#next_offset - #current_offset - core::mem::size_of::<#member_type>());

                let padding = StructMemberPaddingInfo { 
                  pad_name,
                  pad_size 
                };

                member_entries.push(
                  StructMemberEntry {
                    name: member_name.clone(),
                    member,
                    ty,
                    q_ty: member_type.clone(),
                    attr: None,
                    padding: Some(padding)
                  }
                );
              }
            }
        });

        member_entries
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
                pub struct MatricesF32 {
                    pub a: glam::Mat4,
                    pub b: [[f32; 4]; 3],
                    pub c: [[f32; 4]; 2],
                    pub d: [[f32; 3]; 4],
                    pub e: glam::Mat3,
                    pub f: [[f32; 3]; 2],
                    pub g: [[f32; 2]; 4],
                    pub h: [[f32; 2]; 3],
                    pub i: glam::Mat2,
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
                      i: glam::Mat2,
                  ) -> Self {
                      Self { a, b, c, d, e, f, g, h, i }
                  }
                }
                #[repr(C)]
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                    Copy,
                    Clone,
                    PartialEq,
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
                    Copy,
                    Clone,
                    PartialEq,
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
                #[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
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
                #[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
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
                #[repr(C, packed)]
                #[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
                pub struct Input0 {
                    /// Offset: 0x0, Size: 0x4
                    pub a: u32,
                    _pad_a: [u8; 8 - 0 - core::mem::size_of::<u32>()],
                    /// Offset: 0x8, Size: 0x4
                    pub b: i32,
                    _pad_b: [u8; 32 - 8 - core::mem::size_of::<i32>()],
                    /// Offset: 0x20, Size: 0x4
                    pub c: f32,
                    _pad_c: [u8; 64 - 32 - core::mem::size_of::<f32>()],
                }
                impl Input0 {
                    pub fn new(a: u32, b: i32, c: f32) -> Self {
                        Self {
                            a,
                            _pad_a: [0; 8 - 0 - core::mem::size_of::<u32>()],
                            b,
                            _pad_b: [0; 32 - 8 - core::mem::size_of::<i32>()],
                            c,
                            _pad_c: [0; 64 - 32 - core::mem::size_of::<f32>()],
                        }
                    }
                }
                #[repr(C)]
                #[derive(Debug, Copy, Clone, PartialEq)]
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
                          _pad_a: [0; 8 - 0 - core::mem::size_of::<u32>()],
                          b: init.b,
                          _pad_b: [0; 32 - 8 - core::mem::size_of::<i32>()],
                          c: init.c,
                          _pad_c: [0; 64 - 32 - core::mem::size_of::<f32>()],
                      }
                  }
                }
                impl From<Input0Init> for Input0 {
                  fn from(init: Input0Init) -> Self {
                      Self {
                          a: init.a,
                          _pad_a: [0; 8 - 0 - core::mem::size_of::<u32>()],
                          b: init.b,
                          _pad_b: [0; 32 - 8 - core::mem::size_of::<i32>()],
                          c: init.c,
                          _pad_c: [0; 64 - 32 - core::mem::size_of::<f32>()],
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
                #[repr(C, packed)]
                #[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
                pub struct Inner {
                    /// Offset: 0x0, Size: 0x4
                    pub a: f32,
                    _pad_a: [u8; 4 - 0 - core::mem::size_of::<f32>()],
                }
                impl Inner {
                  pub fn new(a: f32) -> Self {
                      Self { 
                        a,
                        _pad_a: [0; 4 - 0 - core::mem::size_of::<f32>()]
                      }
                  }
                }
                #[repr(C)]
                #[derive(Debug, Copy, Clone, PartialEq)]
                pub struct InnerInit {
                    pub a: f32,
                }
                impl InnerInit {
                    pub const fn const_into(&self) -> Inner {
                        let init = self;
                        Inner {
                            a: init.a,
                            _pad_a: [0; 4 - 0 - core::mem::size_of::<f32>()],
                        }
                    }
                }
                impl From<InnerInit> for Inner {
                    fn from(init: InnerInit) -> Self {
                        Self {
                            a: init.a,
                            _pad_a: [0; 4 - 0 - core::mem::size_of::<f32>()],
                        }
                    }
                }
                const _: () = assert!(
                    std::mem::size_of:: < Inner > () == 4, "size of Inner does not match WGSL"
                );
                const _: () = assert!(
                    memoffset::offset_of!(Inner, a) == 0, "offset of Inner.a does not match WGSL"
                );
                #[repr(C, packed)]
                #[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
                pub struct Outer {
                    /// Offset: 0x0, Size: 0x4
                    pub inner: Inner,
                    _pad_inner: [u8; 4 - 0 - core::mem::size_of::<Inner>()],
                }
                impl Outer {
                  pub fn new(inner: Inner) -> Self {
                      Self { 
                        inner,
                        _pad_inner: [0; 4 - 0 - core::mem::size_of::<Inner>()],
                      }
                  }
                }
                #[repr(C)]
                #[derive(Debug, Copy, Clone, PartialEq)]
                pub struct OuterInit {
                    pub inner: Inner,
                }
                impl OuterInit {
                    pub const fn const_into(&self) -> Outer {
                        let init = self;
                        Outer {
                            inner: init.inner,
                            _pad_inner: [0; 4 - 0 - core::mem::size_of::<Inner>()],
                        }
                    }
                }
                impl From<OuterInit> for Outer {
                    fn from(init: OuterInit) -> Self {
                        Self {
                            inner: init.inner,
                            _pad_inner: [0; 4 - 0 - core::mem::size_of::<Inner>()],
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
                #[derive(Debug, Copy, Clone, PartialEq, encase::ShaderType)]
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
                #[derive(Debug, Clone, PartialEq, encase::ShaderType)]
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
    #[should_panic]
    fn write_runtime_sized_array_no_encase() {
        let module = runtime_sized_array_module();

        let _structs = structs(
            &module,
            WriteOptions {
              serialization_strategy: ShaderSerializationStrategy::Bytemuck,
              ..Default::default()
            },
        );
    }

    #[test]
    #[should_panic]
    fn write_runtime_sized_array_bytemuck() {
        let module = runtime_sized_array_module();

        let _structs = structs(
            &module,
            WriteOptions {
                serialization_strategy: ShaderSerializationStrategy::Bytemuck,
                ..Default::default()
            },
        );
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
    fn write_assertion_for_bytemuck_option() {
      let source = indoc! {r#"
        struct ScalingModeData {
          kind: i32,
          // log base 10
          logical_offset: f32,
          coord_offset: f32,
          // percentage
          base_value: f32,
        }
        
        struct UniformsData {
          x_scaling_mode: ScalingModeData,
          y_scaling_mode: ScalingModeData,
          logical_space_center_point: vec2<f32>,
          // centered_mvp: mat2x2<f32>,
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
              ..Default::default()
          },
      );
      let actual = quote!(#(#structs)*);

      assert_tokens_eq!(
          quote! {
            #[repr(C, packed)]
            #[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
            pub struct ScalingModeData {
                /// Offset: 0x0, Size: 0x4
                pub kind: i32,
                _pad_kind: [u8; 4 - 0 - core::mem::size_of::<i32>()],
                /// Offset: 0x4, Size: 0x4
                pub logical_offset: f32,
                _pad_logical_offset: [u8; 8 - 4 - core::mem::size_of::<f32>()],
                /// Offset: 0x8, Size: 0x4
                pub coord_offset: f32,
                _pad_coord_offset: [u8; 12 - 8 - core::mem::size_of::<f32>()],
                /// Offset: 0xc, Size: 0x4
                pub base_value: f32,
                _pad_base_value: [u8; 16 - 12 - core::mem::size_of::<f32>()],
            }
            impl ScalingModeData {
                pub fn new(
                    kind: i32,
                    logical_offset: f32,
                    coord_offset: f32,
                    base_value: f32,
                ) -> Self {
                    Self {
                        kind,
                        _pad_kind: [0; 4 - 0 - core::mem::size_of::<i32>()],
                        logical_offset,
                        _pad_logical_offset: [0; 8 - 4 - core::mem::size_of::<f32>()],
                        coord_offset,
                        _pad_coord_offset: [0; 12 - 8 - core::mem::size_of::<f32>()],
                        base_value,
                        _pad_base_value: [0; 16 - 12 - core::mem::size_of::<f32>()],
                    }
                }
            }
            #[repr(C)]
            #[derive(Debug, Copy, Clone, PartialEq)]
            pub struct ScalingModeDataInit {
                pub kind: i32,
                pub logical_offset: f32,
                pub coord_offset: f32,
                pub base_value: f32,
            }
            impl ScalingModeDataInit {
                pub const fn const_into(&self) -> ScalingModeData {
                    let init = self;
                    ScalingModeData {
                        kind: init.kind,
                        _pad_kind: [0; 4 - 0 - core::mem::size_of::<i32>()],
                        logical_offset: init.logical_offset,
                        _pad_logical_offset: [0; 8 - 4 - core::mem::size_of::<f32>()],
                        coord_offset: init.coord_offset,
                        _pad_coord_offset: [0; 12 - 8 - core::mem::size_of::<f32>()],
                        base_value: init.base_value,
                        _pad_base_value: [0; 16 - 12 - core::mem::size_of::<f32>()],
                    }
                }
            }
            impl From<ScalingModeDataInit> for ScalingModeData {
                fn from(init: ScalingModeDataInit) -> Self {
                    Self {
                        kind: init.kind,
                        _pad_kind: [0; 4 - 0 - core::mem::size_of::<i32>()],
                        logical_offset: init.logical_offset,
                        _pad_logical_offset: [0; 8 - 4 - core::mem::size_of::<f32>()],
                        coord_offset: init.coord_offset,
                        _pad_coord_offset: [0; 12 - 8 - core::mem::size_of::<f32>()],
                        base_value: init.base_value,
                        _pad_base_value: [0; 16 - 12 - core::mem::size_of::<f32>()],
                    }
                }
            }
            const _: () = assert!(
                std::mem::size_of:: < ScalingModeData > () == 16,
                "size of ScalingModeData does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(ScalingModeData, kind) == 0,
                "offset of ScalingModeData.kind does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(ScalingModeData, logical_offset) == 4,
                "offset of ScalingModeData.logical_offset does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(ScalingModeData, coord_offset) == 8,
                "offset of ScalingModeData.coord_offset does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(ScalingModeData, base_value) == 12,
                "offset of ScalingModeData.base_value does not match WGSL"
            );
            #[repr(C, packed)]
            #[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
            pub struct UniformsData {
                /// Offset: 0x0, Size: 0x10
                pub x_scaling_mode: ScalingModeData,
                _pad_x_scaling_mode: [u8; 16 - 0 - core::mem::size_of::<ScalingModeData>()],
                /// Offset: 0x10, Size: 0x10
                pub y_scaling_mode: ScalingModeData,
                _pad_y_scaling_mode: [u8; 32 - 16 - core::mem::size_of::<ScalingModeData>()],
                /// Offset: 0x20, Size: 0x8
                pub logical_space_center_point: [f32; 2],
                _pad_logical_space_center_point: [u8; 48 - 32 - core::mem::size_of::<[f32; 2]>()],
                /// Offset: 0x30, Size: 0x30
                pub centered_mvp: [[f32; 3]; 3],
                _pad_centered_mvp: [u8; 96 - 48 - core::mem::size_of::<[[f32; 3]; 3]>()],
            }
            impl UniformsData {
                pub fn new(
                    x_scaling_mode: ScalingModeData,
                    y_scaling_mode: ScalingModeData,
                    logical_space_center_point: [f32; 2],
                    centered_mvp: [[f32; 3]; 3],
                ) -> Self {
                    Self {
                        x_scaling_mode,
                        _pad_x_scaling_mode: [0; 16 - 0 - core::mem::size_of::<ScalingModeData>()],
                        y_scaling_mode,
                        _pad_y_scaling_mode: [0; 32 - 16 - core::mem::size_of::<ScalingModeData>()],
                        logical_space_center_point,
                        _pad_logical_space_center_point: [0; 48 - 32
                            - core::mem::size_of::<[f32; 2]>()],
                        centered_mvp,
                        _pad_centered_mvp: [0; 96 - 48 - core::mem::size_of::<[[f32; 3]; 3]>()],
                    }
                }
            }
            #[repr(C)]
            #[derive(Debug, Copy, Clone, PartialEq)]
            pub struct UniformsDataInit {
                pub x_scaling_mode: ScalingModeData,
                pub y_scaling_mode: ScalingModeData,
                pub logical_space_center_point: [f32; 2],
                pub centered_mvp: [[f32; 3]; 3],
            }
            impl UniformsDataInit {
                pub const fn const_into(&self) -> UniformsData {
                    let init = self;
                    UniformsData {
                        x_scaling_mode: init.x_scaling_mode,
                        _pad_x_scaling_mode: [0; 16 - 0 - core::mem::size_of::<ScalingModeData>()],
                        y_scaling_mode: init.y_scaling_mode,
                        _pad_y_scaling_mode: [0; 32 - 16 - core::mem::size_of::<ScalingModeData>()],
                        logical_space_center_point: init.logical_space_center_point,
                        _pad_logical_space_center_point: [0; 48 - 32
                            - core::mem::size_of::<[f32; 2]>()],
                        centered_mvp: init.centered_mvp,
                        _pad_centered_mvp: [0; 96 - 48 - core::mem::size_of::<[[f32; 3]; 3]>()],
                    }
                }
            }
            impl From<UniformsDataInit> for UniformsData {
                fn from(init: UniformsDataInit) -> Self {
                    Self {
                        x_scaling_mode: init.x_scaling_mode,
                        _pad_x_scaling_mode: [0; 16 - 0 - core::mem::size_of::<ScalingModeData>()],
                        y_scaling_mode: init.y_scaling_mode,
                        _pad_y_scaling_mode: [0; 32 - 16 - core::mem::size_of::<ScalingModeData>()],
                        logical_space_center_point: init.logical_space_center_point,
                        _pad_logical_space_center_point: [0; 48 - 32
                            - core::mem::size_of::<[f32; 2]>()],
                        centered_mvp: init.centered_mvp,
                        _pad_centered_mvp: [0; 96 - 48 - core::mem::size_of::<[[f32; 3]; 3]>()],
                    }
                }
            }
            const _: () = assert!(
                std::mem::size_of:: < UniformsData > () == 96,
                "size of UniformsData does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(UniformsData, x_scaling_mode) == 0,
                "offset of UniformsData.x_scaling_mode does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(UniformsData, y_scaling_mode) == 16,
                "offset of UniformsData.y_scaling_mode does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(UniformsData, logical_space_center_point) == 32,
                "offset of UniformsData.logical_space_center_point does not match WGSL"
            );
            const _: () = assert!(
                memoffset::offset_of!(UniformsData, centered_mvp) == 48,
                "offset of UniformsData.centered_mvp does not match WGSL"
            );
          },
          actual
      );
    }
}
