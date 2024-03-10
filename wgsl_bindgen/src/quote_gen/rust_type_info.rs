use naga::{Scalar, ScalarKind, VectorSize};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use strum::IntoEnumIterator;
use syn::Index;

use crate::bevy_util::demangle_str;
use crate::quote_gen::demangle_and_qualify;
use crate::wgsl_type::WgslBuiltInMappedType;
use crate::{
  WgslBindgenOption, WgslMatType, WgslType, WgslTypeAlignmentAndSize,
  WgslTypeSerializeStrategy, WgslVecType,
};

#[derive(Debug, Clone)]
pub(crate) struct RustTypeInfo {
  pub tokens: TokenStream,
  // size in bytes, if none then it is a runtime sized array
  pub size: Option<usize>,
  pub alignment: naga::proc::Alignment,
}

impl RustTypeInfo {
  pub fn alignment_value(&self) -> usize {
    self.alignment.round_up(1) as usize
  }

  pub fn size_after_alignment(&self) -> Option<usize> {
    let size = self.size? as u32;
    Some(self.alignment.round_up(size) as usize)
  }
}

impl ToTokens for RustTypeInfo {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    tokens.extend(self.tokens.clone())
  }
}

pub(crate) fn custom_vector_matrix_assertions(
  options: &WgslBindgenOption,
) -> Option<TokenStream> {
  if options.serialization_strategy.is_encase() {
    return None;
  }

  fn build_assert_for(
    options: &WgslBindgenOption,
    ty: impl WgslTypeAlignmentAndSize + Into<WgslType> + WgslBuiltInMappedType,
  ) -> Option<TokenStream> {
    let ty = ty.get_mapped_type(&options.type_map)?;
    let size_after_alignment = ty.size_after_alignment()?;

    let alignment = Index::from(ty.alignment_value());
    let size_after_alignment = Index::from(size_after_alignment);

    Some(quote! {
      assert!(std::mem::size_of::<#ty>() == #size_after_alignment);
      assert!(std::mem::align_of::<#ty>() == #alignment);
    })
  }

  let assertions = WgslVecType::iter()
    .filter_map(|ty| build_assert_for(options, ty))
    .chain(WgslMatType::iter().filter_map(|ty| build_assert_for(options, ty)))
    .collect::<Vec<_>>();

  Some(quote! {
    const WGSL_BASE_TYPE_ASSERTS: () = { #(#assertions)* };
  })
}

#[allow(non_snake_case)]
pub(crate) const fn RustTypeInfo(
  tokens: TokenStream,
  size: usize,
  alignment: naga::proc::Alignment,
) -> RustTypeInfo {
  RustTypeInfo {
    tokens,
    size: Some(size),
    alignment,
  }
}

pub(crate) fn rust_scalar_type(
  scalar: &naga::Scalar,
  alignment: naga::proc::Alignment,
) -> RustTypeInfo {
  // TODO: Support other widths?
  match (scalar.kind, scalar.width) {
    (ScalarKind::Sint, 1) => RustTypeInfo(quote!(i8), 1, alignment),
    (ScalarKind::Uint, 1) => RustTypeInfo(quote!(u8), 1, alignment),
    (ScalarKind::Sint, 2) => RustTypeInfo(quote!(i16), 2, alignment),
    (ScalarKind::Uint, 2) => RustTypeInfo(quote!(u16), 2, alignment),
    (ScalarKind::Sint, 4) => RustTypeInfo(quote!(i32), 4, alignment),
    (ScalarKind::Uint, 4) => RustTypeInfo(quote!(u32), 4, alignment),
    (ScalarKind::Float, 4) => RustTypeInfo(quote!(f32), 4, alignment),
    (ScalarKind::Float, 8) => RustTypeInfo(quote!(f64), 8, alignment),
    // TODO: Do booleans have a width?
    (ScalarKind::Bool, 1) => RustTypeInfo(quote!(bool), 1, alignment),
    _ => unreachable!(),
  }
}

/// Get the array stride and padding in bytes
fn get_stride_and_padding(
  alignment: naga::proc::Alignment,
  size: naga::VectorSize,
  width: u8,
  options: &WgslBindgenOption,
) -> (u32, u32) {
  let width = width as u32;
  let rows = size as u32;
  let used_bytes = rows * width;
  let total_bytes = alignment.round_up(used_bytes);
  let padding_bytes = total_bytes - used_bytes;

  if options.serialization_strategy == WgslTypeSerializeStrategy::Bytemuck {
    (total_bytes, padding_bytes)
  } else {
    (total_bytes, 0)
  }
}

#[inline]
fn assert_alignment_and_size(
  ty: impl WgslTypeAlignmentAndSize + std::fmt::Debug,
  expected_alignment: naga::proc::Alignment,
  expected_size_after_alignment: u32,
) {
  let (alignment, size) = ty.alignment_and_size();
  let alignment = naga::proc::Alignment::from_width(alignment);
  let size_after_alignment = alignment.round_up(size as u32);
  assert_eq!(
    alignment, expected_alignment,
    "Built in type {:?} has unexpected alignment",
    ty
  );
  assert_eq!(
    size_after_alignment, expected_size_after_alignment,
    "Built in type {:?} has unexpected size",
    ty
  );
}

fn map_naga_vec_type(
  size: VectorSize,
  scalar: Scalar,
  alignment: naga::proc::Alignment,
  options: &WgslBindgenOption,
) -> Option<RustTypeInfo> {
  use ScalarKind::*;
  use VectorSize::*;

  use crate::WgslVecType::*;
  let ty = match (size, scalar.kind, scalar.width) {
    (Bi, Sint, 4) => Vec2i,
    (Tri, Sint, 4) => Vec3i,
    (Quad, Sint, 4) => Vec4i,
    (Bi, Uint, 4) => Vec2u,
    (Tri, Uint, 4) => Vec3u,
    (Quad, Uint, 4) => Vec4u,
    (Bi, Float, 4) => Vec2f,
    (Tri, Float, 4) => Vec3f,
    (Quad, Float, 4) => Vec4f,
    (Bi, Float, 2) => Vec2h,
    (Tri, Float, 2) => Vec3h,
    (Quad, Float, 2) => Vec4h,
    _ => return None,
  };

  // validate assumptions about alignment and size
  let expected_size_after_alignment =
    alignment.round_up(size as u32 * scalar.width as u32);
  assert_alignment_and_size(ty, alignment, expected_size_after_alignment);

  ty.get_mapped_type(&options.type_map)
}

fn map_naga_mat_type(
  columns: VectorSize,
  rows: VectorSize,
  scalar: Scalar,
  alignment: naga::proc::Alignment,
  options: &WgslBindgenOption,
) -> Option<RustTypeInfo> {
  use ScalarKind::*;
  use VectorSize::*;

  use crate::WgslMatType::*;
  let ty = match (columns, rows, scalar.kind, scalar.width) {
    (Bi, Bi, Float, 4) => Mat2x2f,
    (Bi, Bi, Float, 2) => Mat2x2h,
    (Tri, Bi, Float, 4) => Mat3x2f,
    (Tri, Bi, Float, 2) => Mat3x2h,
    (Quad, Bi, Float, 4) => Mat4x2f,
    (Quad, Bi, Float, 2) => Mat4x2h,
    (Bi, Tri, Float, 4) => Mat2x3f,
    (Bi, Tri, Float, 2) => Mat2x3h,
    (Tri, Tri, Float, 4) => Mat3x3f,
    (Tri, Tri, Float, 2) => Mat3x3h,
    (Quad, Tri, Float, 4) => Mat4x3f,
    (Quad, Tri, Float, 2) => Mat4x3h,
    (Bi, Quad, Float, 4) => Mat2x4f,
    (Bi, Quad, Float, 2) => Mat2x4h,
    (Tri, Quad, Float, 4) => Mat3x4f,
    (Tri, Quad, Float, 2) => Mat3x4h,
    (Quad, Quad, Float, 4) => Mat4x4f,
    (Quad, Quad, Float, 2) => Mat4x4h,
    _ => return None,
  };

  // validate assumptions about alignment and size
  let expected_vec_r_size = alignment.round_up(rows as u32 * scalar.width as u32);
  let expected_size_after_alignment = expected_vec_r_size * columns as u32;
  assert_alignment_and_size(ty, alignment, expected_size_after_alignment);
  ty.get_mapped_type(&options.type_map)
}

pub(crate) fn rust_type(
  module: &naga::Module,
  ty: &naga::Type,
  options: &WgslBindgenOption,
) -> RustTypeInfo {
  let t_handle = module.types.get(ty).unwrap();
  let mut layouter = naga::proc::Layouter::default();
  layouter.update(module.to_ctx()).unwrap();

  let type_layout = layouter[t_handle];

  let alignment = type_layout.alignment;

  let with_validation = |ty: RustTypeInfo| -> Option<RustTypeInfo> {
    assert!(alignment == ty.alignment);
    Some(ty)
  };

  match &ty.inner {
    naga::TypeInner::Scalar(scalar) => rust_scalar_type(scalar, alignment),
    naga::TypeInner::Vector { size, scalar } => {
      let rust_type =
        map_naga_vec_type(*size, *scalar, alignment, options).and_then(with_validation);
      if let Some(ty) = rust_type {
        ty
      } else {
        // TODO: Add more built-in types to WgslTypes and handle it there instead
        // here the padding bytes are also inserted
        let (stride, _) = get_stride_and_padding(alignment, *size, scalar.width, options);
        let inner_type = rust_scalar_type(scalar, alignment).tokens;
        let len = Index::from((stride / scalar.width as u32) as usize);
        RustTypeInfo(quote!([#inner_type; #len]), stride as usize, alignment)
      }
    }
    naga::TypeInner::Matrix {
      columns,
      rows,
      scalar,
    } => {
      let rust_type = map_naga_mat_type(*columns, *rows, *scalar, alignment, options)
        .and_then(with_validation);

      if let Some(ty) = rust_type {
        ty
      } else {
        // TODO: Add more built types to WgslTypes and handle it there instead
        // here the padding bytes are also inserted
        let inner_type = rust_scalar_type(scalar, alignment).tokens;
        let (col_array_stride, _) =
          get_stride_and_padding(alignment, *rows, scalar.width, options);
        let size = col_array_stride * (*columns as u32);

        let cols = Index::from(*columns as usize);
        let rows = Index::from((col_array_stride / scalar.width as u32) as usize);
        RustTypeInfo(quote!([[#inner_type; #rows]; #cols]), size as usize, alignment)
      }
    }
    naga::TypeInner::Image { .. } => todo!(),
    naga::TypeInner::Sampler { .. } => todo!(),
    naga::TypeInner::Atomic(scalar) => rust_scalar_type(scalar, alignment),
    naga::TypeInner::Pointer { base: _, space: _ } => todo!(),
    naga::TypeInner::ValuePointer { .. } => todo!(),
    naga::TypeInner::Array {
      base,
      size: naga::ArraySize::Constant(size),
      stride,
    } => {
      let inner_ty = rust_type(module, &module.types[*base], options);
      let count = Index::from(size.get() as usize);

      RustTypeInfo(quote!([#inner_ty; #count]), *stride as usize, alignment)
    }
    naga::TypeInner::Array {
      base,
      size: naga::ArraySize::Dynamic,
      ..
    } => {
      // panic!("Runtime-sized arrays can only be used in variable declarations or as the last field of a struct.");
      let element_type = rust_type(module, &module.types[*base], &options);
      let member_type = match options.serialization_strategy {
        WgslTypeSerializeStrategy::Encase => {
          quote!(Vec<#element_type>)
        }
        WgslTypeSerializeStrategy::Bytemuck => {
          quote!([#element_type; N])
        }
      };
      RustTypeInfo {
        tokens: member_type,
        size: None,
        alignment,
      }
    }
    naga::TypeInner::Struct {
      members: _,
      span: _,
    } => {
      // TODO: Support structs?
      let name_str = ty.name.as_ref().unwrap();
      let name = demangle_and_qualify(name_str);
      let size = type_layout.size as usize;

      // custom map struct
      let mapped_type = WgslType::Struct {
        fully_qualified_name: demangle_str(name_str).into(),
      }
      .get_mapped_type(&options.type_map, size, alignment)
      .unwrap_or(RustTypeInfo(name, size, alignment));

      mapped_type
    }
    naga::TypeInner::BindingArray { base: _, size: _ } => todo!(),
    naga::TypeInner::AccelerationStructure => todo!(),
    naga::TypeInner::RayQuery => todo!(),
  }
}
