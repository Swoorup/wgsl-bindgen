use proc_macro2::TokenStream;
use quote::quote;

use super::{WgslTypeMapBuild, WgslTypeSerializeStrategy};
use crate::{FastIndexMap, WgslType};

/// Information about a WGSL type including its Rust representation, size, and alignment
#[derive(Debug, Clone)]
pub struct WgslTypeInfo {
  /// The Rust type representation as a TokenStream
  pub quoted_type: TokenStream,
  /// Alignment of the quoted type
  pub alignment: usize,
}

impl WgslTypeInfo {
  pub fn new(quoted_type: TokenStream, alignment: usize) -> Self {
    Self {
      quoted_type,
      alignment,
    }
  }
}

pub type WgslTypeMap = FastIndexMap<WgslType, WgslTypeInfo>;

/// Helper function to create WgslTypeInfo from actual Rust type size and alignment
fn type_info_from_rust<T: 'static>(quoted_type: TokenStream) -> WgslTypeInfo {
  let alignment = std::mem::align_of::<T>();
  WgslTypeInfo::new(quoted_type, alignment)
}

/// Rust types like `[f32; 4]` or `[[f32; 4]; 4]`.
#[derive(Clone)]
pub struct RustWgslTypeMap;

impl WgslTypeMapBuild for RustWgslTypeMap {
  fn build(&self, _: WgslTypeSerializeStrategy) -> WgslTypeMap {
    WgslTypeMap::default()
  }
}

/// `glam` types like `glam::Vec4` or `glam::Mat4`.
/// Types not representable by `glam` like `mat2x3<f32>` will use the output from [RustWgslTypeMap].
#[derive(Clone)]
pub struct GlamWgslTypeMap;

impl WgslTypeMapBuild for GlamWgslTypeMap {
  fn build(&self, serialize_strategy: WgslTypeSerializeStrategy) -> WgslTypeMap {
    use crate::WgslMatType::*;
    use crate::WgslType::*;
    use crate::WgslVecType::*;

    #[cfg(feature = "glam")]
    {
      let is_encase = serialize_strategy.is_encase();
      let types = vec![
        (Vector(Vec2i), type_info_from_rust::<glam::IVec2>(quote!(glam::IVec2))),
        (Vector(Vec3i), type_info_from_rust::<glam::IVec3>(quote!(glam::IVec3))),
        (Vector(Vec4i), type_info_from_rust::<glam::IVec4>(quote!(glam::IVec4))),
        (Vector(Vec2u), type_info_from_rust::<glam::UVec2>(quote!(glam::UVec2))),
        (Vector(Vec3u), type_info_from_rust::<glam::UVec3>(quote!(glam::UVec3))),
        (Vector(Vec4u), type_info_from_rust::<glam::UVec4>(quote!(glam::UVec4))),
        (Vector(Vec2f), type_info_from_rust::<glam::Vec2>(quote!(glam::Vec2))),
        (Vector(Vec3f), type_info_from_rust::<glam::Vec3>(quote!(glam::Vec3))),
        (Vector(Vec4f), type_info_from_rust::<glam::Vec4>(quote!(glam::Vec4))),
        (Matrix(Mat2x2f), type_info_from_rust::<glam::Mat2>(quote!(glam::Mat2))),
        (Matrix(Mat3x3f), type_info_from_rust::<glam::Mat3A>(quote!(glam::Mat3A))),
        (Matrix(Mat4x4f), type_info_from_rust::<glam::Mat4>(quote!(glam::Mat4))),
      ];
      types.into_iter().collect()
    }

    #[cfg(not(feature = "glam"))]
    {
      // No fallback when glam feature is not enabled
      WgslTypeMap::default()
    }
  }
}

/// `nalgebra` types like `nalgebra::SVector<f64, 4>` or `nalgebra::SMatrix<f32, 2, 3>`.
#[derive(Clone)]
pub struct NalgebraWgslTypeMap;

impl WgslTypeMapBuild for NalgebraWgslTypeMap {
  fn build(&self, _: WgslTypeSerializeStrategy) -> WgslTypeMap {
    use crate::WgslMatType::*;
    use crate::WgslType::*;
    use crate::WgslVecType::*;

    #[cfg(feature = "nalgebra")]
    {
      vec![
        (
          Vector(Vec2i),
          type_info_from_rust::<nalgebra::SVector<i32, 2>>(
            quote!(nalgebra::SVector<i32, 2>),
          ),
        ),
        (
          Vector(Vec3i),
          type_info_from_rust::<nalgebra::SVector<i32, 3>>(
            quote!(nalgebra::SVector<i32, 3>),
          ),
        ),
        (
          Vector(Vec4i),
          type_info_from_rust::<nalgebra::SVector<i32, 4>>(
            quote!(nalgebra::SVector<i32, 4>),
          ),
        ),
        (
          Vector(Vec2u),
          type_info_from_rust::<nalgebra::SVector<u32, 2>>(
            quote!(nalgebra::SVector<u32, 2>),
          ),
        ),
        (
          Vector(Vec3u),
          type_info_from_rust::<nalgebra::SVector<u32, 3>>(
            quote!(nalgebra::SVector<u32, 3>),
          ),
        ),
        (
          Vector(Vec4u),
          type_info_from_rust::<nalgebra::SVector<u32, 4>>(
            quote!(nalgebra::SVector<u32, 4>),
          ),
        ),
        (
          Vector(Vec2f),
          type_info_from_rust::<nalgebra::SVector<f32, 2>>(
            quote!(nalgebra::SVector<f32, 2>),
          ),
        ),
        (
          Vector(Vec3f),
          type_info_from_rust::<nalgebra::SVector<f32, 3>>(
            quote!(nalgebra::SVector<f32, 3>),
          ),
        ),
        (
          Vector(Vec4f),
          type_info_from_rust::<nalgebra::SVector<f32, 4>>(
            quote!(nalgebra::SVector<f32, 4>),
          ),
        ),
        (
          Matrix(Mat2x2f),
          type_info_from_rust::<nalgebra::SMatrix<f32, 2, 2>>(
            quote!(nalgebra::SMatrix<f32, 2, 2>),
          ),
        ),
        (
          Matrix(Mat2x3f),
          type_info_from_rust::<nalgebra::SMatrix<f32, 3, 2>>(
            quote!(nalgebra::SMatrix<f32, 3, 2>),
          ),
        ),
        (
          Matrix(Mat2x4f),
          type_info_from_rust::<nalgebra::SMatrix<f32, 4, 2>>(
            quote!(nalgebra::SMatrix<f32, 4, 2>),
          ),
        ),
        (
          Matrix(Mat3x2f),
          type_info_from_rust::<nalgebra::SMatrix<f32, 2, 3>>(
            quote!(nalgebra::SMatrix<f32, 2, 3>),
          ),
        ),
        (
          Matrix(Mat3x3f),
          type_info_from_rust::<nalgebra::SMatrix<f32, 3, 3>>(
            quote!(nalgebra::SMatrix<f32, 3, 3>),
          ),
        ),
        (
          Matrix(Mat3x4f),
          type_info_from_rust::<nalgebra::SMatrix<f32, 4, 3>>(
            quote!(nalgebra::SMatrix<f32, 4, 3>),
          ),
        ),
        (
          Matrix(Mat4x2f),
          type_info_from_rust::<nalgebra::SMatrix<f32, 2, 4>>(
            quote!(nalgebra::SMatrix<f32, 2, 4>),
          ),
        ),
        (
          Matrix(Mat4x3f),
          type_info_from_rust::<nalgebra::SMatrix<f32, 3, 4>>(
            quote!(nalgebra::SMatrix<f32, 3, 4>),
          ),
        ),
        (
          Matrix(Mat4x4f),
          type_info_from_rust::<nalgebra::SMatrix<f32, 4, 4>>(
            quote!(nalgebra::SMatrix<f32, 4, 4>),
          ),
        ),
      ]
      .into_iter()
      .collect()
    }

    #[cfg(not(feature = "nalgebra"))]
    {
      // No fallback when nalgebra feature is not enabled
      WgslTypeMap::default()
    }
  }
}
