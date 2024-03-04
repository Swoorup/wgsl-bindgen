use std::path::PathBuf;

use derive_builder::Builder;
use derive_more::IsVariant;
use enumflags2::{bitflags, BitFlags};
pub use naga::valid::Capabilities as WgslShaderIRCapabilities;
use proc_macro2::TokenStream;
use quote::quote;

use crate::{WGSLBindgen, WgslBindgenError, WgslType, WgslTypeSerializeStrategy};

/// An enum representing the source type that will be generated for the output.
#[bitflags(default = UseEmbed)]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, IsVariant)]
pub enum WgslShaderSourceType {
  /// Preparse the shader modules and embed the final shader string in the output.
  /// This option skips the naga_oil dependency in the output, and but doesn't allow shader defines.
  UseEmbed = 0b0001,

  /// Use Composer with embedded strings for each shader module,
  /// This option allows shader defines and but doesn't allow hot-reloading.
  UseComposerEmbed = 0b0010,

  /// Use Composer with absolute path to shaders, useful for hot-reloading
  /// This option allows shader defines and is useful for hot-reloading.
  UseComposerWithPath = 0b0100,
}

/// A struct representing a directory to scan for additional source files.
///
/// This struct is used to represent a directory to scan for additional source files
/// when generating Rust bindings for WGSL shaders. The `module_import_root` field
/// is used to specify the root prefix or namespace that should be applied to all
/// shaders given as the entrypoints, and the `directory` field is used to specify
/// the directory to scan for additional source files.
#[derive(Debug, Clone, Default)]
pub struct AdditionalScanDirectory {
  pub module_import_root: Option<String>,
  pub directory: String,
}

impl From<(Option<&str>, &str)> for AdditionalScanDirectory {
  fn from((module_import_root, directory): (Option<&str>, &str)) -> Self {
    Self {
      module_import_root: module_import_root.map(ToString::to_string),
      directory: directory.to_string(),
    }
  }
}

pub type WgslTypeMap = std::collections::HashMap<WgslType, TokenStream>;

/// A trait for building `WgslType` to `TokenStream` map.
///
/// This map is used to convert built-in WGSL types into their corresponding
/// representations in the generated Rust code. The specific format used for
/// matrix and vector types can vary, and the generated types for the same WGSL
/// type may differ in size or alignment.
///
/// Implementations of this trait provide a `build` function that takes a
/// `WgslTypeSerializeStrategy` and returns an `WgslTypeMap`.
pub trait WgslTypeMapBuild {
  /// Builds the `WgslTypeMap` based on the given serialization strategy.
  fn build(&self, strategy: WgslTypeSerializeStrategy) -> WgslTypeMap;
}

impl WgslTypeMapBuild for WgslTypeMap {
  fn build(&self, _: WgslTypeSerializeStrategy) -> WgslTypeMap {
    self.clone()
  }
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

/// This struct is used to create a custom mapping from the wgsl side to rust side,
/// skipping generation of the struct and using the custom one instead.
/// This also means skipping checks for alignment and size when using bytemuck
/// for the struct.
/// This is useful for core primitive types you would want to model in Rust side
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CustomStructMapping {
  /// fully qualified struct name of the struct in wgsl, eg: `lib::fp64::Fp64`
  from: String,
  /// fully qualified struct name in your crate, eg: `crate::fp64::Fp64`
  to: String,
}

impl From<(&str, &str)> for CustomStructMapping {
  fn from((from, to): (&str, &str)) -> Self {
    CustomStructMapping {
      from: from.to_owned(),
      to: to.to_owned(),
    }
  }
}

#[derive(Debug, Default, Builder)]
#[builder(
  setter(into),
  field(private),
  build_fn(private, name = "fallible_build")
)]
pub struct WgslBindgenOption {
  /// A vector of entry points to be added. Each entry point is represented as a `String`.
  #[builder(setter(each(name = "add_entry_point", into)))]
  pub entry_points: Vec<String>,

  /// The root prefix/namespace if any applied to all shaders given as the entrypoints.
  #[builder(default, setter(strip_option, into))]
  pub module_import_root: Option<String>,

  /// The root shader workspace directory where all the imports will tested for resolution.
  #[builder(setter(into))]
  pub workspace_root: PathBuf,

  /// A boolean flag indicating whether to emit a rerun-if-changed directive to Cargo. Defaults to `true`.
  #[builder(default = "true")]
  pub emit_rerun_if_change: bool,

  /// A boolean flag indicating whether to skip header comments. Enabling headers allows to not rerun if contents did not change.
  #[builder(default = "false")]
  pub skip_header_comments: bool,

  /// A boolean flag indicating whether to skip the hash check. This will avoid reruns of bindings generation if
  /// entry shaders including their imports has not changed. Defaults to `false`.
  #[builder(default = "false")]
  pub skip_hash_check: bool,

  /// Derive [encase::ShaderType](https://docs.rs/encase/latest/encase/trait.ShaderType.html#)
  /// for user defined WGSL structs when `WgslTypeSerializeStrategy::Encase`.
  /// else derive bytemuck
  #[builder(default)]
  pub serialization_strategy: WgslTypeSerializeStrategy,

  /// Derive [serde::Serialize](https://docs.rs/serde/1.0.159/serde/trait.Serialize.html)
  /// and [serde::Deserialize](https://docs.rs/serde/1.0.159/serde/trait.Deserialize.html)
  /// for user defined WGSL structs when `true`.
  #[builder(default = "false")]
  pub derive_serde: bool,

  /// The shader source type generated bitflags. Defaults to `WgslShaderSourceType::UseSingleString`.
  #[builder(default)]
  pub shader_source_type: BitFlags<WgslShaderSourceType>,

  /// The output file path for the generated Rust bindings. Defaults to `None`.
  #[builder(default, setter(strip_option, into))]
  pub output: Option<PathBuf>,

  /// The additional set of directories to scan for source files.
  #[builder(default, setter(into, each(name = "additional_scan_dir", into)))]
  pub additional_scan_dirs: Vec<AdditionalScanDirectory>,

  /// The [wgpu::naga::valid::Capabilities](https://docs.rs/wgpu/latest/wgpu/naga/valid/struct.Capabilities.html) to support. Defaults to `None`.
  #[builder(default, setter(strip_option))]
  pub ir_capabilities: Option<WgslShaderIRCapabilities>,

  /// Whether to generate short constructor similar to enums constructors instead of `new`, if number of parameters are below the specified threshold
  /// Defaults to `None`
  #[builder(default, setter(strip_option, into))]
  pub short_constructor: Option<i32>,

  /// A mapping operation for WGSL built-in types. This is used to map WGSL built-in types to their corresponding representations.
  #[builder(setter(custom))]
  pub type_map: WgslTypeMap,

  /// A vector of custom struct mappings to be added, which will override the struct to be generated.
  #[builder(default, setter(each(name = "custom_struct_mapping", into)))]
  pub custom_struct_mappings: Vec<CustomStructMapping>,
}

impl WgslBindgenOptionBuilder {
  pub fn build(&mut self) -> Result<WGSLBindgen, WgslBindgenError> {
    self.merge_struct_mapping();

    let options = self.fallible_build()?;
    WGSLBindgen::new(options)
  }

  pub fn type_map(&mut self, map_build: impl WgslTypeMapBuild) -> &mut Self {
    let serialization_strategy = self
      .serialization_strategy
      .expect("Serialization strategy must be set before `wgs_type_map`");

    let map = map_build.build(serialization_strategy);

    match self.type_map.as_mut() {
      Some(m) => m.extend(map),
      None => self.type_map = Some(map),
    }

    self
  }

  fn merge_struct_mapping(&mut self) {
    let struct_mappings = self
      .custom_struct_mappings
      .iter()
      .flatten()
      .map(|mapping| {
        let wgsl_type = WgslType::Struct {
          fully_qualified_name: mapping.from.clone(),
        };
        let token_stream = syn::parse_str::<TokenStream>(&mapping.to).unwrap();
        (wgsl_type, token_stream)
      })
      .collect::<std::collections::HashMap<_, _>>();

    self.type_map(struct_mappings);
  }
}
