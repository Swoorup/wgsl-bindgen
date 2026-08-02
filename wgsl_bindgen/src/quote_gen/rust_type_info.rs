use naga::{Scalar, ScalarKind, VectorSize};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use strum::IntoEnumIterator;
use syn::Index;

use crate::bevy_util::demangle_str;
use crate::quote_gen::demangle_and_fully_qualify;
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
  /// If this type has tuple padding, this contains the init-friendly version
  pub init_type: Option<TokenStream>,
  /// Converts the init-friendly representation into the emitted storage type.
  pub init_conversion: Option<RustTypeInitConversion>,
}

#[derive(Debug, Clone)]
pub(crate) enum RustTypeInitConversion {
  Array(Box<Self>),
  PaddedArrayElement(Option<Box<Self>>),
}

impl RustTypeInitConversion {
  pub fn generate(&self, value: TokenStream) -> TokenStream {
    match self {
      Self::Array(element_conversion) => {
        let converted = element_conversion.generate(quote!(value));
        quote!(#value.map(|value| #converted))
      }
      Self::PaddedArrayElement(inner_conversion) => {
        let value = inner_conversion
          .as_ref()
          .map_or(value.clone(), |conversion| conversion.generate(value));
        quote!(WgslBindgenArrayElement::new(#value))
      }
    }
  }

  pub fn uses_array_element_helper(&self) -> bool {
    match self {
      Self::Array(element_conversion) => element_conversion.uses_array_element_helper(),
      Self::PaddedArrayElement(_) => true,
    }
  }
}

impl RustTypeInfo {
  pub fn alignment_value(&self) -> usize {
    self.alignment.round_up(1) as usize
  }

  pub fn aligned_size(&self) -> Option<usize> {
    let size = self.size? as u32;
    Some(self.alignment.round_up(size) as usize)
  }

  pub fn is_dynamic_array(&self) -> bool {
    self.size.is_none()
  }

  pub fn quote_min_binding_size(&self) -> TokenStream {
    if self.is_dynamic_array() {
      quote!(None)
    } else {
      let ty = quote!(#self);
      quote!(std::num::NonZeroU64::new(std::mem::size_of::<#ty>() as _))
    }
  }

  pub fn uses_array_element_helper(&self) -> bool {
    self
      .init_conversion
      .as_ref()
      .is_some_and(RustTypeInitConversion::uses_array_element_helper)
  }

  fn input_type(&self) -> TokenStream {
    self
      .init_type
      .clone()
      .unwrap_or_else(|| self.tokens.clone())
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

    let alignment = Index::from(ty.alignment_value());
    let aligned_size = Index::from(ty.aligned_size()?);

    Some(quote! {
      assert!(std::mem::size_of::<#ty>() == #aligned_size);
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
    init_type: None,
    init_conversion: None,
  }
}

#[allow(non_snake_case)]
pub(crate) fn RustTypeInfoWithInit(
  tokens: TokenStream,
  size: usize,
  alignment: naga::proc::Alignment,
  init_type: TokenStream,
  init_conversion: RustTypeInitConversion,
) -> RustTypeInfo {
  RustTypeInfo {
    tokens,
    size: Some(size),
    alignment,
    init_type: Some(init_type),
    init_conversion: Some(init_conversion),
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
    (ScalarKind::Float, 2) => RustTypeInfo(quote!(half::f16), 2, alignment),
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
    "Built in type {ty:?} has unexpected alignment"
  );
  assert_eq!(
    size_after_alignment, expected_size_after_alignment,
    "Built in type {ty:?} has unexpected size"
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

/// Generates a Rust type information for a Naga type.
///
/// Specify the invoke entry module to generate fully qualified type name.///
pub(crate) fn rust_type(
  invoking_entry_module: Option<&str>,
  module: &naga::Module,
  ty: &naga::Type,
  options: &WgslBindgenOption,
) -> RustTypeInfo {
  let mut layouter = naga::proc::Layouter::default();
  let naga_context = module.to_ctx();
  layouter.update(naga_context).unwrap();

  let (type_layout, alignment) = if let Some(t_handle) = module.types.get(ty) {
    let type_layout = layouter[t_handle];
    let alignment = type_layout.alignment;
    (type_layout, alignment)
  } else {
    // Type is not in the arena - this happens with shared bind groups across multiple entry points
    // For dynamic arrays and other synthetic types, we can determine minimal layout info manually
    // TODO: I am not sure if this is the best way to handle this, but it works for now
    match &ty.inner {
      naga::TypeInner::Array { base, .. } => {
        // For arrays, try to get the base type's alignment
        let base_layout = layouter[*base];
        (base_layout, base_layout.alignment)
      }
      _ => {
        panic!("Type {ty:?} not found in module types arena, cannot determine alignment");
      }
    }
  };

  match &ty.inner {
    naga::TypeInner::Scalar(scalar) => rust_scalar_type(scalar, alignment),
    naga::TypeInner::Vector { size, scalar } => {
      let rust_type = map_naga_vec_type(*size, *scalar, alignment, options);
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
      let rust_type = map_naga_mat_type(*columns, *rows, *scalar, alignment, options);

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
      let inner_ty =
        rust_type(invoking_entry_module, module, &module.types[*base], options);
      let inner_input_type = inner_ty.input_type();
      let count = Index::from(size.get() as usize);
      let total_size = (size.get() as usize) * (*stride as usize);

      // Check if we need padding between array elements
      if options.serialization_strategy == WgslTypeSerializeStrategy::Bytemuck {
        let element_size = inner_ty.size.unwrap_or(0);
        let actual_stride = *stride as usize;

        if element_size < actual_stride {
          // Store padded array elements as exact-stride byte blocks. The struct
          // builder emits the transparent helper type and converts the ergonomic
          // element values into these blocks in the Init builder.
          RustTypeInfoWithInit(
            quote!([WgslBindgenArrayElement<#actual_stride>; #count]),
            total_size,
            alignment,
            quote!([#inner_input_type; #count]),
            RustTypeInitConversion::Array(Box::new(
              RustTypeInitConversion::PaddedArrayElement(
                inner_ty.init_conversion.clone().map(Box::new),
              ),
            )),
          )
        } else if let Some(inner_conversion) = inner_ty.init_conversion.clone() {
          RustTypeInfoWithInit(
            quote!([#inner_ty; #count]),
            total_size,
            alignment,
            quote!([#inner_input_type; #count]),
            RustTypeInitConversion::Array(Box::new(inner_conversion)),
          )
        } else {
          // No padding needed
          RustTypeInfo(quote!([#inner_ty; #count]), total_size, alignment)
        }
      } else {
        RustTypeInfo(quote!([#inner_ty; #count]), total_size, alignment)
      }
    }
    naga::TypeInner::Array {
      base,
      size: naga::ArraySize::Dynamic,
      stride,
    } => {
      let element_type =
        rust_type(invoking_entry_module, module, &module.types[*base], options);
      let input_element_type = element_type.input_type();
      let (member_type, init_type, init_conversion) = match options.serialization_strategy
      {
        WgslTypeSerializeStrategy::Encase => (quote!(Vec<#element_type>), None, None),
        WgslTypeSerializeStrategy::Bytemuck => {
          let element_size = element_type.size.unwrap_or(0);
          let stride = *stride as usize;
          if element_size < stride {
            (
              quote!(WgslBindgenArrayElement<#stride>),
              Some(input_element_type),
              Some(RustTypeInitConversion::PaddedArrayElement(
                element_type.init_conversion.clone().map(Box::new),
              )),
            )
          } else if let Some(element_conversion) = element_type.init_conversion.clone() {
            (
              element_type.tokens.clone(),
              Some(input_element_type),
              Some(element_conversion),
            )
          } else {
            (element_type.tokens.clone(), None, None)
          }
        }
      };
      RustTypeInfo {
        tokens: member_type,
        size: None,
        alignment,
        init_type,
        init_conversion,
      }
    }
    naga::TypeInner::Array {
      size: naga::ArraySize::Pending(_),
      ..
    } => {
      unimplemented!("Pending arrays are not supported yet");
    }
    naga::TypeInner::Struct { members, span: _ } => {
      let name_str = ty.name.as_ref().unwrap();
      let name = demangle_and_fully_qualify(name_str, invoking_entry_module);

      let size = type_layout.size as usize;

      // custom map struct
      let mut mapped_type = WgslType::Struct {
        fully_qualified_name: demangle_str(name_str).into(),
      }
      .get_mapped_type(&options.type_map, size, alignment)
      .unwrap_or(RustTypeInfo(name, size, alignment));

      // check if the last member is a runtime sized array
      if let Some(last) = members.last() {
        if let naga::TypeInner::Array {
          size: naga::ArraySize::Dynamic,
          ..
        } = &module.types[last.ty].inner
        {
          mapped_type.size = None;
        }
      }

      mapped_type
    }
    naga::TypeInner::BindingArray { base: _, size: _ } => todo!(),
    naga::TypeInner::AccelerationStructure { .. } => todo!(),
    naga::TypeInner::RayQuery { .. } => todo!(),
    naga::TypeInner::CooperativeMatrix { .. } => todo!(),
  }
}
