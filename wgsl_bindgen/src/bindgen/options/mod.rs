mod bindings;
mod types;

use std::path::PathBuf;

pub use bindings::*;
use derive_builder::Builder;
use derive_more::IsVariant;
use enumflags2::{bitflags, BitFlags};
use proc_macro2::TokenStream;
use regex::Regex;
pub use types::*;

use crate::{
  FastIndexMap, WGSLBindgen, WgslBindgenError, WgslType, WgslTypeSerializeStrategy,
};

/// The [wgpu::naga::valid::Capabilities](https://docs.rs/wgpu/latest/wgpu/naga/valid/struct.Capabilities.html) to use for the module.
#[derive(Clone, Copy, Debug)]
pub struct WgslShaderIrCapabilities {
  pub capabilities: naga::valid::Capabilities,
  pub subgroup_stages: naga::valid::ShaderStages,
}

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

pub type WgslTypeMap = FastIndexMap<WgslType, TokenStream>;

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

/// This struct is used to create a custom mapping from the wgsl side to rust side,
/// skipping generation of the struct and using the custom one instead.
/// This also means skipping checks for alignment and size when using bytemuck
/// for the struct.
/// This is useful for core primitive types you would want to model in Rust side
#[derive(Clone, Debug)]
pub struct OverrideStruct {
  /// fully qualified struct name of the struct in wgsl, eg: `lib::fp64::Fp64`
  pub from: String,
  /// fully qualified struct name in your crate, eg: `crate::fp64::Fp64`
  pub to: TokenStream,
}

impl From<(&str, TokenStream)> for OverrideStruct {
  fn from((from, to): (&str, TokenStream)) -> Self {
    OverrideStruct {
      from: from.to_owned(),
      to,
    }
  }
}

/// Struct  for overriding the field type of specific structs.
#[derive(Clone, Debug)]
pub struct OverrideStructFieldType {
  pub struct_regex: Regex,
  pub field_regex: Regex,
  pub override_type: TokenStream,
}
impl From<(Regex, Regex, TokenStream)> for OverrideStructFieldType {
  fn from(
    (struct_regex, field_regex, override_type): (Regex, Regex, TokenStream),
  ) -> Self {
    Self {
      struct_regex,
      field_regex,
      override_type,
    }
  }
}
impl From<(&str, &str, TokenStream)> for OverrideStructFieldType {
  fn from((struct_regex, field_regex, override_type): (&str, &str, TokenStream)) -> Self {
    Self {
      struct_regex: Regex::new(struct_regex).expect("Failed to create struct regex"),
      field_regex: Regex::new(field_regex).expect("Failed to create field regex"),
      override_type,
    }
  }
}

/// Struct for overriding alignment of specific structs.
#[derive(Clone, Debug)]
pub struct OverrideStructAlignment {
  pub struct_regex: Regex,
  pub alignment: u16,
}
impl From<(Regex, u16)> for OverrideStructAlignment {
  fn from((struct_regex, alignment): (Regex, u16)) -> Self {
    Self {
      struct_regex: struct_regex,
      alignment: alignment,
    }
  }
}
impl From<(&str, u16)> for OverrideStructAlignment {
  fn from((struct_regex, alignment): (&str, u16)) -> Self {
    Self {
      struct_regex: Regex::new(struct_regex).expect("Failed to create struct regex"),
      alignment: alignment,
    }
  }
}

/// An enum representing the visibility of the type generated in the output
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WgslTypeVisibility {
  /// All exported types set to `pub` visiblity
  #[default]
  Public,

  /// All exported types set to `pub(crate)` visiblity
  RestrictedCrate,

  /// All exported types set to `pub(super)` visiblity
  RestrictedSuper,
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
  pub ir_capabilities: Option<WgslShaderIrCapabilities>,

  /// Whether to generate short constructor similar to enums constructors instead of `new`, if number of parameters are below the specified threshold
  /// Defaults to `None`
  #[builder(default, setter(strip_option, into))]
  pub short_constructor: Option<i32>,

  /// Which visiblity to use for the exported types.
  #[builder(default)]
  pub type_visibility: WgslTypeVisibility,

  /// A mapping operation for WGSL built-in types. This is used to map WGSL built-in types to their corresponding representations.
  #[builder(setter(custom))]
  pub type_map: WgslTypeMap,

  /// A vector of custom struct mappings to be added, which will override the struct to be generated.
  /// This is merged with the default struct mappings.
  #[builder(default, setter(each(name = "add_override_struct_mapping", into)))]
  pub override_struct: Vec<OverrideStruct>,

  /// A vector of `OverrideStructFieldType` to override the generated types for struct fields in matching structs.
  #[builder(default, setter(into))]
  pub override_struct_field_type: Vec<OverrideStructFieldType>,

  /// A vector of regular expressions and alignments that override the generated alignment for matching structs.
  /// This can be used in scenarios where a specific minimum alignment is required for a uniform buffer.
  /// Refer to the [WebGPU specs](https://www.w3.org/TR/webgpu/#dom-supported-limits-minuniformbufferoffsetalignment) for more information.
  #[builder(default, setter(into))]
  pub override_struct_alignment: Vec<OverrideStructAlignment>,

  /// The regular expression of the padding fields used in the shader struct types.
  /// These fields will be omitted in the *Init structs generated, and will automatically be assigned the default values.
  #[builder(default, setter(each(name = "add_custom_padding_field_regexp", into)))]
  pub custom_padding_field_regexps: Vec<Regex>,

  /// Whether to always have the init struct generated in the out. This is only applicable when using bytemuck mode.
  #[builder(default = "false")]
  pub always_generate_init_struct: bool,

  /// This field can be used to provide a custom generator for extra bindings that are not covered by the default generator.
  #[builder(default, setter(custom))]
  pub extra_binding_generator: Option<BindingGenerator>,

  /// This field is used to provide the default generator for WGPU bindings. The generator is represented as a `BindingGenerator`.
  #[builder(default, setter(custom))]
  pub wgpu_binding_generator: BindingGenerator,
}

impl WgslBindgenOptionBuilder {
  pub fn build(&mut self) -> Result<WGSLBindgen, WgslBindgenError> {
    self.merge_struct_type_overrides();

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

  fn merge_struct_type_overrides(&mut self) {
    let struct_mappings = self
      .override_struct
      .iter()
      .flatten()
      .map(|mapping| {
        let wgsl_type = WgslType::Struct {
          fully_qualified_name: mapping.from.clone(),
        };
        (wgsl_type, mapping.to.clone())
      })
      .collect::<FastIndexMap<_, _>>();

    self.type_map(struct_mappings);
  }

  pub fn extra_binding_generator(
    &mut self,
    config: impl GetBindingsGeneratorConfig,
  ) -> &mut Self {
    let generator = Some(config.get_generator_config());
    self.extra_binding_generator = Some(generator);
    self
  }
}
