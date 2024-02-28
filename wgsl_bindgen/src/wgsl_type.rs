use std::fmt::Debug;

use derive_more::{From, IsVariant};
use strum_macros::EnumIter;

use crate::quote_gen::RustTypeInfo;
use crate::WgslTypeMap;

/// The `WgslType` enum represents various WGSL vectors.
/// See [spec](https://www.w3.org/TR/WGSL/#alignment-and-size)
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumIter)]
pub enum WgslVecType {
  Vec2i,
  Vec3i,
  Vec4i,
  Vec2u,
  Vec3u,
  Vec4u,
  Vec2f,
  Vec3f,
  Vec4f,
  Vec2h,
  Vec3h,
  Vec4h,
}

/// The `WgslType` enum represents various Wgsl matrices.
/// See [spec](https://www.w3.org/TR/WGSL/#alignment-and-size)
#[derive(Debug, From, Clone, Copy, Hash, PartialEq, Eq, EnumIter)]
pub enum WgslMatType {
  Mat2x2f,
  Mat2x3f,
  Mat2x4f,
  Mat3x2f,
  Mat3x3f,
  Mat3x4f,
  Mat4x2f,
  Mat4x3f,
  Mat4x4f,
  Mat2x2h,
  Mat2x3h,
  Mat2x4h,
  Mat3x2h,
  Mat3x3h,
  Mat3x4h,
  Mat4x2h,
  Mat4x3h,
  Mat4x4h,
}

pub(crate) trait WgslTypeAlignmentAndSize {
  fn alignment_and_size(&self) -> (u8, usize);
}

impl WgslTypeAlignmentAndSize for WgslVecType {
  fn alignment_and_size(&self) -> (u8, usize) {
    use WgslVecType::*;
    match self {
      Vec2i | Vec2u | Vec2f => (8, 8),
      Vec2h => (4, 4),
      Vec3i | Vec3u | Vec3f => (16, 12),
      Vec3h => (8, 6),
      Vec4i | Vec4u | Vec4f => (16, 16),
      Vec4h => (8, 8),
    }
  }
}

impl WgslTypeAlignmentAndSize for WgslMatType {
  fn alignment_and_size(&self) -> (u8, usize) {
    use WgslMatType::*;
    match self {
      // AlignOf(vecR), SizeOf(array<vecR, C>)
      Mat2x2f => (8, 16),
      Mat2x2h => (4, 8),
      Mat3x2f => (8, 24),
      Mat3x2h => (4, 12),
      Mat4x2f => (8, 32),
      Mat4x2h => (4, 16),
      Mat2x3f => (16, 32),
      Mat2x3h => (8, 16),
      Mat3x3f => (16, 48),
      Mat3x3h => (8, 24),
      Mat4x3f => (16, 64),
      Mat4x3h => (8, 32),
      Mat2x4f => (16, 32),
      Mat2x4h => (8, 16),
      Mat3x4f => (16, 48),
      Mat3x4h => (8, 24),
      Mat4x4f => (16, 64),
      Mat4x4h => (8, 32),
    }
  }
}

pub(crate) trait WgslBuiltInMappedType {
  fn get_mapped_type(&self, type_map: &WgslTypeMap) -> Option<RustTypeInfo>;
}

impl<T> WgslBuiltInMappedType for T
where
  T: WgslTypeAlignmentAndSize + Copy,
  WgslType: From<T>,
{
  fn get_mapped_type(&self, type_map: &WgslTypeMap) -> Option<RustTypeInfo> {
    let (alignment_width, size) = self.alignment_and_size();
    let wgsl_ty = WgslType::from(*self);
    let ty = type_map.get(&wgsl_ty)?.clone();
    let alignment = naga::proc::Alignment::from_width(alignment_width);
    Some(RustTypeInfo(ty, size, alignment))
  }
}

/// The `WgslType` enum represents various WGSL types, such as vectors and matrices.
/// See [spec](https://www.w3.org/TR/WGSL/#alignment-and-size)
#[derive(Debug, From, Clone, Hash, PartialEq, Eq, IsVariant)]
pub enum WgslType {
  Vector(WgslVecType),
  Matrix(WgslMatType),
  Struct { fully_qualified_name: String },
}

impl WgslType {
  pub(crate) fn get_mapped_type(
    &self,
    type_map: &WgslTypeMap,
    size: usize,
    alignment: naga::proc::Alignment,
  ) -> Option<RustTypeInfo> {
    match self {
      WgslType::Vector(vec_ty) => vec_ty.get_mapped_type(type_map),
      WgslType::Matrix(mat_ty) => mat_ty.get_mapped_type(type_map),
      WgslType::Struct { .. } => {
        let ty = type_map.get(self)?.clone();
        Some(RustTypeInfo(ty, size, alignment))
      }
    }
  }
}
