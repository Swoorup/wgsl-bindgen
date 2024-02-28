use std::collections::HashSet;

use naga::{Handle, Type};
use proc_macro2::TokenStream;

use crate::bevy_util::demangle;
use crate::quote_gen::{RustSourceItem, RustStructBuilder};
use crate::{WgslBindgenOption, WgslTypeSerializeStrategy};

pub fn structs_items(
  module: &naga::Module,
  options: &WgslBindgenOption,
) -> Vec<RustSourceItem> {
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
    .filter_map(|(t_handle, ty)| {
      if let naga::TypeInner::Struct { members, .. } = &ty.inner {
        let demangled_name = demangle(ty.name.as_ref().unwrap());

        // skip if using custom struct mapping
        if options.type_map.contains_key(&crate::WgslType::Struct {
          fully_qualified_name: demangled_name.into(),
        }) {
          None
        } else {
          let rust_struct = rust_struct(
            ty,
            members,
            &layouter,
            t_handle,
            module,
            options,
            &global_variable_types,
          );

          Some(RustSourceItem::from_mangled(ty.name.as_ref().unwrap(), rust_struct))
        }
      } else {
        None
      }
    })
    .collect()
}

#[allow(unused)]
pub fn structs(module: &naga::Module, options: &WgslBindgenOption) -> Vec<TokenStream> {
  structs_items(module, options)
    .into_iter()
    .map(|s| s.item)
    .collect()
}

fn rust_struct(
  naga_type: &naga::Type,
  naga_members: &[naga::StructMember],
  layouter: &naga::proc::Layouter,
  t_handle: naga::Handle<naga::Type>,
  naga_module: &naga::Module,
  options: &WgslBindgenOption,
  global_variable_types: &HashSet<Handle<Type>>,
) -> TokenStream {
  let layout = layouter[t_handle];

  // Assume types used in global variables are host shareable and require validation.
  // This includes storage, uniform, and workgroup variables.
  // This also means types that are never used will not be validated.
  // Structs used only for vertex inputs do not require validation on desktop platforms.
  // Vertex input layout is handled already by setting the attribute offsets and types.
  // This allows vertex input field types without padding like vec3 for positions.
  let is_host_sharable = global_variable_types.contains(&t_handle);

  let has_rts_array = struct_has_rts_array_member(naga_members, naga_module);
  let is_directly_sharable = options.serialization_strategy
    == WgslTypeSerializeStrategy::Bytemuck
    && is_host_sharable;

  let builder = RustStructBuilder::from_naga(
    naga_type,
    naga_members,
    naga_module,
    &options,
    layout,
    is_directly_sharable,
    is_host_sharable,
    has_rts_array,
  );
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
    naga::TypeInner::BindingArray { base, .. } => {
      add_types_recursive(types, module, *base)
    }
    _ => (),
  }
}

fn struct_has_rts_array_member(
  members: &[naga::StructMember],
  module: &naga::Module,
) -> bool {
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
  use quote::quote;

  use super::*;
  use crate::*;

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

    let structs = structs(&module, &WgslBindgenOption::default());
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
            pub const fn new(a: u32, b: i32, c: f32) -> Self {
                Self { a, b, c }
            }
          }
          #[repr(C)]
          #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
          pub struct VectorsU32 {
              pub a: [u32; 2],
              pub b: [u32; 4],
              pub c: [u32; 4],
          }
          impl VectorsU32 {
            pub const fn new(a: [u32; 2], b: [u32; 4], c: [u32; 4]) -> Self {
                Self { a, b, c }
            }
          }
          #[repr(C)]
          #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
          pub struct VectorsI32 {
              pub a: [i32; 2],
              pub b: [i32; 4],
              pub c: [i32; 4],
          }
          impl VectorsI32 {
            pub const fn new(a: [i32; 2], b: [i32; 4], c: [i32; 4]) -> Self {
                Self { a, b, c }
            }
          }
          #[repr(C)]
          #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
          pub struct VectorsF32 {
              pub a: [f32; 2],
              pub b: [f32; 4],
              pub c: [f32; 4],
          }
          impl VectorsF32 {
            pub const fn new(a: [f32; 2], b: [f32; 4], c: [f32; 4]) -> Self {
                Self { a, b, c }
            }
          }
          #[repr(C)]
          #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
          pub struct VectorsF64 {
              pub a: [f64; 2],
              pub b: [f64; 4],
              pub c: [f64; 4],
          }
          impl VectorsF64 {
            pub const fn new(a: [f64; 2], b: [f64; 4], c: [f64; 4]) -> Self {
                Self { a, b, c }
            }
          }
          #[repr(C)]
          #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
          pub struct MatricesF32 {
              pub a: [[f32; 4]; 4],
              pub b: [[f32; 4]; 4],
              pub c: [[f32; 2]; 4],
              pub d: [[f32; 4]; 3],
              pub e: [[f32; 4]; 3],
              pub f: [[f32; 2]; 3],
              pub g: [[f32; 4]; 2],
              pub h: [[f32; 4]; 2],
              pub i: [[f32; 2]; 2],
          }
          impl MatricesF32 {
            pub const fn new(
                a: [[f32; 4]; 4],
                b: [[f32; 4]; 4],
                c: [[f32; 2]; 4],
                d: [[f32; 4]; 3],
                e: [[f32; 4]; 3],
                f: [[f32; 2]; 3],
                g: [[f32; 4]; 2],
                h: [[f32; 4]; 2],
                i: [[f32; 2]; 2],
            ) -> Self {
                Self { a, b, c, d, e, f, g, h, i }
            }
          }
          #[repr(C)]
          #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
          pub struct MatricesF64 {
              pub a: [[f64; 4]; 4],
              pub b: [[f64; 4]; 4],
              pub c: [[f64; 2]; 4],
              pub d: [[f64; 4]; 3],
              pub e: [[f64; 4]; 3],
              pub f: [[f64; 2]; 3],
              pub g: [[f64; 4]; 2],
              pub h: [[f64; 4]; 2],
              pub i: [[f64; 2]; 2],
          }
          impl MatricesF64 {
            pub const fn new(
                a: [[f64; 4]; 4],
                b: [[f64; 4]; 4],
                c: [[f64; 2]; 4],
                d: [[f64; 4]; 3],
                e: [[f64; 4]; 3],
                f: [[f64; 2]; 3],
                g: [[f64; 4]; 2],
                h: [[f64; 4]; 2],
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
            pub const fn new(a: [u32; 5], b: [f32; 3], c: [[[f32; 4]; 4]; 512]) -> Self {
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
            pub const fn new(a: MatricesF32, b: MatricesF64) -> Self {
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

            struct StaticArrays {
                a: array<u32, 5>,
                b: array<f32, 3>,
                c: array<mat4x4<f32>, 512>,
            };
            var<uniform> h: StaticArrays;

            struct Nested {
                a: MatricesF32,
                b: VectorsF32
            }
            var<uniform> i: Nested;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        type_map: GlamWgslTypeMap.build(WgslTypeSerializeStrategy::Encase),
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
            pub const fn new(a: u32, b: i32, c: f32) -> Self {
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
            pub const fn new(a: glam::UVec2, b: glam::UVec3, c: glam::UVec4) -> Self {
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
            pub const fn new(a: glam::IVec2, b: glam::IVec3, c: glam::IVec4) -> Self {
                Self { a, b, c }
            }
        }
        #[repr(C)]
        #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
        pub struct VectorsF32 {
            pub a: glam::Vec2,
            pub b: glam::Vec3A,
            pub c: glam::Vec4,
        }
        impl VectorsF32 {
            pub const fn new(a: glam::Vec2, b: glam::Vec3A, c: glam::Vec4) -> Self {
                Self { a, b, c }
            }
        }
        #[repr(C)]
        #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
        pub struct MatricesF32 {
            pub a: glam::Mat4,
            pub b: [[f32; 4]; 4],
            pub c: [[f32; 2]; 4],
            pub d: [[f32; 4]; 3],
            pub e: glam::Mat3A,
            pub f: [[f32; 2]; 3],
            pub g: [[f32; 4]; 2],
            pub h: [[f32; 4]; 2],
            pub i: glam::Mat2,
        }
        impl MatricesF32 {
            pub const fn new(
                a: glam::Mat4,
                b: [[f32; 4]; 4],
                c: [[f32; 2]; 4],
                d: [[f32; 4]; 3],
                e: glam::Mat3A,
                f: [[f32; 2]; 3],
                g: [[f32; 4]; 2],
                h: [[f32; 4]; 2],
                i: glam::Mat2,
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
            pub const fn new(a: [u32; 5], b: [f32; 3], c: [glam::Mat4; 512]) -> Self {
                Self { a, b, c }
            }
        }
        #[repr(C)]
        #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
        pub struct Nested {
            pub a: MatricesF32,
            pub b: VectorsF32,
        }
        impl Nested {
            pub const fn new(a: MatricesF32, b: VectorsF32) -> Self {
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

            struct StaticArrays {
                a: array<u32, 5>,
                b: array<f32, 3>,
                c: array<mat4x4<f32>, 512>,
            };
            var<uniform> h: StaticArrays;

            struct Nested {
                a: MatricesF32,
                b: VectorsF32
            }
            var<uniform> i: Nested;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        type_map: NalgebraWgslTypeMap.build(WgslTypeSerializeStrategy::Encase),
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
            pub const fn new(a: u32, b: i32, c: f32) -> Self {
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
            pub const fn new(
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
            pub const fn new(
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
            pub const fn new(
              a: nalgebra::SVector<f32, 2>,
              b: nalgebra::SVector<f32, 3>,
              c: nalgebra::SVector<f32, 4>,
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
            pub const fn new(
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
          pub struct StaticArrays {
              pub a: [u32; 5],
              pub b: [f32; 3],
              pub c: [nalgebra::SMatrix<f32, 4, 4>; 512],
          }
          impl StaticArrays {
            pub const fn new(
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
              pub b: VectorsF32,
          }
          impl Nested {
            pub const fn new(a: MatricesF32, b: VectorsF32) -> Self {
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Encase,
        derive_serde: false,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Encase),
        ..Default::default()
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
            pub const fn new(a: u32, b: i32, c: f32) -> Self {
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
            pub const fn new(a: Input0, b: f32) -> Self {
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Encase,
        derive_serde: true,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Encase),
        ..Default::default()
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
            pub const fn new(a: u32, b: i32, c: f32) -> Self {
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
            pub const fn new(a: Input0, b: f32) -> Self {
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        derive_serde: false,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        ..Default::default()
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
            pub const fn new(a: u32, b: i32, c: f32) -> Self {
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        derive_serde: false,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        ..Default::default()
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
              pub const fn new(a: u32, b: i32, c: f32) -> Self {
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        derive_serde: false,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_eq!(
      quote! {
        #[repr(C, align(4))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct Input0 {
            /// size: 4, offset: 0x0, type: `u32`
            pub a: u32,
            pub _pad_a: [u8; 0x8 - core::mem::size_of::<u32>()],
            /// size: 4, offset: 0x8, type: `i32`
            pub b: i32,
            pub _pad_b: [u8; 0x18 - core::mem::size_of::<i32>()],
            /// size: 4, offset: 0x20, type: `f32`
            pub c: f32,
            pub _pad_c: [u8; 0x20 - core::mem::size_of::<f32>()],
        }
        impl Input0 {
            pub const fn new(a: u32, b: i32, c: f32) -> Self {
                Self {
                    a,
                    _pad_a: [0; 0x8 - core::mem::size_of::<u32>()],
                    b,
                    _pad_b: [0; 0x18 - core::mem::size_of::<i32>()],
                    c,
                    _pad_c: [0; 0x20 - core::mem::size_of::<f32>()],
                }
            }
        }
        unsafe impl bytemuck::Zeroable for Input0 {}
        unsafe impl bytemuck::Pod for Input0 {}
        const _: () = {
          assert!(std::mem::offset_of!(Input0, a) == 0);
          assert!(std::mem::offset_of!(Input0, b) == 8);
          assert!(std::mem::offset_of!(Input0, c) == 32);
          assert!(std::mem::size_of::<Input0>() == 64);
        };

        #[repr(C)]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct Input0Init {
            pub a: u32,
            pub b: i32,
            pub c: f32,
        }
        impl Input0Init {
            pub const fn const_into(&self) -> Input0 {
                Input0 {
                    a: self.a,
                    _pad_a: [0; 0x8 - core::mem::size_of::<u32>()],
                    b: self.b,
                    _pad_b: [0; 0x18 - core::mem::size_of::<i32>()],
                    c: self.c,
                    _pad_c: [0; 0x20 - core::mem::size_of::<f32>()],
                }
            }
        }
        impl From<Input0Init> for Input0 {
            fn from(data: Input0Init) -> Self {
                data.const_into()
            }
        }

        #[repr(C, align(4))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct Inner {
            /// size: 4, offset: 0x0, type: `f32`
            pub a: f32,
        }
        impl Inner {
            pub const fn new(a: f32) -> Self {
                Self { a }
            }
        }
        unsafe impl bytemuck::Zeroable for Inner {}
        unsafe impl bytemuck::Pod for Inner {}
        const _: () = {
          assert!(std::mem::offset_of!(Inner, a) == 0);
          assert!(std::mem::size_of:: < Inner > () == 4);
        };
        #[repr(C, align(4))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct Outer {
            /// size: 4, offset: 0x0, type: `struct`
            pub inner: Inner,
        }
        impl Outer {
            pub const fn new(inner: Inner) -> Self {
                Self { inner }
            }
        }
        unsafe impl bytemuck::Zeroable for Outer {}
        unsafe impl bytemuck::Pod for Outer {}
        const _: () = {
          assert!(std::mem::offset_of!(Outer, inner) == 0);
          assert!(std::mem::size_of:: < Outer > () == 4);
        };
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
      &WgslBindgenOption {
        type_map: NalgebraWgslTypeMap.build(WgslTypeSerializeStrategy::Encase),
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
            pub const fn new(num: u32, numi: i32) -> Self {
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Encase,
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
            pub const fn new(other_data: i32, the_array: Vec<u32>) -> Self {
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        ..Default::default()
      },
    );

    let actual = quote!(#(#structs)*);

    assert_tokens_eq!(
      quote! {
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct RtsStruct<const N: usize> {
            /// size: 4, offset: 0x0, type: `i32`
            pub other_data: i32,
            /// size: 4, offset: 0x4, type: `array<u32>`
            pub the_array: [u32; N]
        }
        impl<const N:usize> RtsStruct<N> {
            pub const fn new(other_data: i32, the_array: [u32; N]) -> Self {
                Self { other_data, the_array }
            }
        }
        unsafe impl<const N: usize> bytemuck::Zeroable for RtsStruct<N> {}
        unsafe impl<const N: usize> bytemuck::Pod for RtsStruct<N> {}
        const _: () = {
            assert!(std::mem::offset_of!(RtsStruct<1>, other_data) == 0);
            assert!(std::mem::offset_of!(RtsStruct<1>, the_array) == 4);
            assert!(std::mem::size_of::<RtsStruct<1> >() == 8);
        };
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Encase,
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_eq!(
      quote! {
        #[repr(C, align(16))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct UniformsData {
            /// size: 48, offset: 0x0, type: `mat3x3<f32>`
            pub a: [[f32; 4]; 3],
        }
        impl UniformsData {
            pub const fn new(a: [[f32; 4]; 3]) -> Self {
                Self { a }
            }
        }
        unsafe impl bytemuck::Zeroable for UniformsData {}
        unsafe impl bytemuck::Pod for UniformsData {}
        const _: () = {
             assert!(std::mem::offset_of!(UniformsData, a) == 0);
             assert!(std::mem::size_of::<UniformsData> () == 48);
        };
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        type_map: GlamWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_eq!(
      quote! {
        #[repr(C, align(16))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct UniformsData {
            /// size: 48, offset: 0x0, type: `mat3x3<f32>`
            pub centered_mvp: glam::Mat3A,
        }
        impl UniformsData {
            pub const fn new(centered_mvp: glam::Mat3A) -> Self {
                Self { centered_mvp }
            }
        }
        unsafe impl bytemuck::Zeroable for UniformsData {}
        unsafe impl bytemuck::Pod for UniformsData {}
        const _: () = {
            assert!(std::mem::offset_of!(UniformsData, centered_mvp) == 0);
            assert!(std::mem::size_of:: <UniformsData>() == 48);
        };
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
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_eq!(
      quote! {
        #[repr(C, align(16))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct MatricesF32 {
            /// size: 64, offset: 0x0, type: `mat4x4<f32>`
            pub a: [[f32; 4]; 4],
            /// size: 64, offset: 0x40, type: `mat4x3<f32>`
            pub b: [[f32; 4]; 4],
            /// size: 32, offset: 0x80, type: `mat4x2<f32>`
            pub c: [[f32; 2]; 4],
            /// size: 48, offset: 0xA0, type: `mat3x4<f32>`
            pub d: [[f32; 4]; 3],
        }
        impl MatricesF32 {
            pub const fn new(
                a: [[f32; 4]; 4],
                b: [[f32; 4]; 4],
                c: [[f32; 2]; 4],
                d: [[f32; 4]; 3],
            ) -> Self {
                Self { a, b, c, d }
            }
        }
        unsafe impl bytemuck::Zeroable for MatricesF32 {}
        unsafe impl bytemuck::Pod for MatricesF32 {}
        const _: () = {
            assert!(std::mem::offset_of!(MatricesF32, a) == 0);
            assert!(std::mem::offset_of!(MatricesF32, b) == 64);
            assert!(std::mem::offset_of!(MatricesF32, c) == 128);
            assert!(std::mem::offset_of!(MatricesF32, d) == 160);
            assert!(std::mem::size_of::<MatricesF32>() == 208);
        };
      },
      actual
    );
  }

  #[test]
  fn write_shorter_constructor() {
    let source = indoc! {r#"
        struct Uniform {
            position_data: vec2<f32>,
        };
        @group(0) @binding(0) var<uniform> u: Uniform;
      "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        type_map: GlamWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        short_constructor: Some(1),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_eq!(
      quote! {
        #[repr(C, align(8))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct Uniform {
            /// size: 8, offset: 0x0, type: `vec2<f32>`
            pub position_data: [f32; 2],
        }

        pub const fn Uniform(position_data: [f32; 2]) -> Uniform {
            Uniform { position_data }
        }
        unsafe impl bytemuck::Zeroable for Uniform {}
        unsafe impl bytemuck::Pod for Uniform {}
        const _: () = {
            assert!(std::mem::offset_of!(Uniform, position_data) == 0);
            assert!(std::mem::size_of:: < Uniform > () == 8);
        };
      },
      actual
    );
  }
}
