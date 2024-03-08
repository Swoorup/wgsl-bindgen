use quote::quote;

use super::{WgslTypeMap, WgslTypeMapBuild, WgslTypeSerializeStrategy};

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
    let is_encase = serialize_strategy.is_encase();
    let types = if is_encase {
      vec![
        (Vector(Vec2i), quote!(glam::IVec2)),
        (Vector(Vec3i), quote!(glam::IVec3)),
        (Vector(Vec4i), quote!(glam::IVec4)),
        (Vector(Vec2u), quote!(glam::UVec2)),
        (Vector(Vec3u), quote!(glam::UVec3)),
        (Vector(Vec4u), quote!(glam::UVec4)),
        (Vector(Vec2f), quote!(glam::Vec2)),
        (Vector(Vec3f), quote!(glam::Vec3A)),
        (Vector(Vec4f), quote!(glam::Vec4)),
        (Matrix(Mat2x2f), quote!(glam::Mat2)),
        (Matrix(Mat3x3f), quote!(glam::Mat3A)),
        (Matrix(Mat4x4f), quote!(glam::Mat4)),
      ]
    } else {
      vec![
        (Vector(Vec3f), quote!(glam::Vec3A)),
        (Vector(Vec4f), quote!(glam::Vec4)),
        (Matrix(Mat3x3f), quote!(glam::Mat3A)),
        (Matrix(Mat4x4f), quote!(glam::Mat4)),
      ]
    };

    types.into_iter().collect()
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

    vec![
      (Vector(Vec2i), quote!(nalgebra::SVector<i32, 2>)),
      (Vector(Vec3i), quote!(nalgebra::SVector<i32, 3>)),
      (Vector(Vec4i), quote!(nalgebra::SVector<i32, 4>)),
      (Vector(Vec2u), quote!(nalgebra::SVector<u32, 2>)),
      (Vector(Vec3u), quote!(nalgebra::SVector<u32, 3>)),
      (Vector(Vec4u), quote!(nalgebra::SVector<u32, 4>)),
      (Vector(Vec2f), quote!(nalgebra::SVector<f32, 2>)),
      (Vector(Vec3f), quote!(nalgebra::SVector<f32, 3>)),
      (Vector(Vec4f), quote!(nalgebra::SVector<f32, 4>)),
      (Matrix(Mat2x2f), quote!(nalgebra::SMatrix<f32, 2, 2>)),
      (Matrix(Mat2x3f), quote!(nalgebra::SMatrix<f32, 3, 2>)),
      (Matrix(Mat2x4f), quote!(nalgebra::SMatrix<f32, 4, 2>)),
      (Matrix(Mat3x2f), quote!(nalgebra::SMatrix<f32, 2, 3>)),
      (Matrix(Mat3x3f), quote!(nalgebra::SMatrix<f32, 3, 3>)),
      (Matrix(Mat3x4f), quote!(nalgebra::SMatrix<f32, 4, 3>)),
      (Matrix(Mat4x2f), quote!(nalgebra::SMatrix<f32, 2, 4>)),
      (Matrix(Mat4x3f), quote!(nalgebra::SMatrix<f32, 3, 4>)),
      (Matrix(Mat4x4f), quote!(nalgebra::SMatrix<f32, 4, 4>)),
    ]
    .into_iter()
    .collect()
  }
}
