mod bindings;
mod types;

use std::collections::HashMap;
use std::path::PathBuf;

pub use bindings::*;
use derive_builder::Builder;
use derive_more::IsVariant;
use enumflags2::{bitflags, BitFlags};
pub use naga::valid::Capabilities as WgslShaderIrCapabilities;
use proc_macro2::TokenStream;
use regex::Regex;
pub use types::*;

use crate::{
  FastIndexMap, WGSLBindgen, WgslBindgenError, WgslType, WgslTypeSerializeStrategy,
};

/// An enum representing the source type that will be generated for the output.
#[bitflags(default = EmbedSource)]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, IsVariant)]
pub enum WgslShaderSourceType {
  /// Preparse the shader modules and embed the final shader string in the output.
  /// This option skips the naga_oil dependency in the output, and but doesn't allow shader defines.
  EmbedSource,

  /// Use Composer with embedded strings for each shader module,
  /// This option allows shader defines and but doesn't allow hot-reloading.
  EmbedWithNagaOilComposer,

  /// Use Composer with relative paths and user-provided file loading
  /// This option allows shader defines and custom IO without requiring nightly Rust.
  ComposerWithRelativePath,
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
  /// the alignment of the struct in bytes, this is used to ensure that the struct is aligned correctly
  pub alignment: usize,
}

impl From<(&str, TokenStream, usize)> for OverrideStruct {
  fn from((from, to, alignment): (&str, TokenStream, usize)) -> Self {
    OverrideStruct {
      from: from.to_owned(),
      to,
      alignment,
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
      struct_regex,
      alignment,
    }
  }
}
impl From<(&str, u16)> for OverrideStructAlignment {
  fn from((struct_regex, alignment): (&str, u16)) -> Self {
    Self {
      struct_regex: Regex::new(struct_regex).expect("Failed to create struct regex"),
      alignment,
    }
  }
}

/// Struct for overriding binding module path of bindgroup entry
#[derive(Clone, Debug)]
pub struct OverrideBindGroupEntryModulePath {
  pub bind_group_entry_regex: Regex,
  pub target_path: String,
}
impl From<(Regex, &str)> for OverrideBindGroupEntryModulePath {
  fn from((bind_group_entry_regex, target_path): (Regex, &str)) -> Self {
    Self {
      bind_group_entry_regex,
      target_path: target_path.to_string(),
    }
  }
}
impl From<(&str, &str)> for OverrideBindGroupEntryModulePath {
  fn from((bind_group_entry_regex, target_path): (&str, &str)) -> Self {
    Self {
      bind_group_entry_regex: Regex::new(bind_group_entry_regex)
        .expect("Failed to create bind group entry regex"),
      target_path: target_path.to_string(),
    }
  }
}

/// Struct for overriding texture filterability for specific bindings
#[derive(Clone, Debug)]
pub struct OverrideTextureFilterability {
  /// Regex to match binding path (e.g., "shared_data::.*texture.*")
  pub binding_regex: Regex,
  /// Whether the texture should be filterable
  pub filterable: bool,
}
impl From<(Regex, bool)> for OverrideTextureFilterability {
  fn from((binding_regex, filterable): (Regex, bool)) -> Self {
    Self {
      binding_regex,
      filterable,
    }
  }
}
impl From<(&str, bool)> for OverrideTextureFilterability {
  fn from((binding_regex, filterable): (&str, bool)) -> Self {
    Self {
      binding_regex: Regex::new(binding_regex).expect("Failed to create binding regex"),
      filterable,
    }
  }
}

/// Enum for sampler binding types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplerType {
  Filtering,
  NonFiltering,
  Comparison,
}

/// Struct for overriding sampler types for specific bindings
#[derive(Clone, Debug)]
pub struct OverrideSamplerType {
  /// Regex to match binding path (e.g., ".*shadow_sampler.*")
  pub binding_regex: Regex,
  /// The sampler type to use
  pub sampler_type: SamplerType,
}
impl From<(Regex, SamplerType)> for OverrideSamplerType {
  fn from((binding_regex, sampler_type): (Regex, SamplerType)) -> Self {
    Self {
      binding_regex,
      sampler_type,
    }
  }
}
impl From<(&str, SamplerType)> for OverrideSamplerType {
  fn from((binding_regex, sampler_type): (&str, SamplerType)) -> Self {
    Self {
      binding_regex: Regex::new(binding_regex).expect("Failed to create binding regex"),
      sampler_type,
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

  /// The shader source type generated bitflags. Defaults to `WgslShaderSourceType::EmbedSource`.
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

  /// A vector of regular expressions and target module path that that override the module path for bind group entries.
  /// This can be used to customize where bind group entries are generated in the output code.
  #[builder(default, setter(into))]
  pub override_bind_group_entry_module_path: Vec<OverrideBindGroupEntryModulePath>,

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

  /// A vector of texture filterability overrides for specific bindings.
  /// Allows specifying which textures should not be filterable.
  #[builder(default, setter(into))]
  pub override_texture_filterability: Vec<OverrideTextureFilterability>,

  /// A vector of sampler type overrides for specific bindings.
  /// Allows specifying the sampler binding type (Filtering, NonFiltering, Comparison).
  #[builder(default, setter(into))]
  pub override_sampler_type: Vec<OverrideSamplerType>,

  /// Shader definitions to be passed to naga-oil for conditional compilation.
  /// These are preprocessor definitions that can be used in WGSL shaders with #ifdef, #ifndef, etc.
  #[builder(default, setter(into))]
  pub shader_defs: Vec<(String, naga_oil::compose::ShaderDefValue)>,
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

  /// Add a shader definition value
  pub fn add_shader_def(
    &mut self,
    name: impl Into<String>,
    value: naga_oil::compose::ShaderDefValue,
  ) -> &mut Self {
    if self.shader_defs.is_none() {
      self.shader_defs = Some(Vec::new());
    }
    self
      .shader_defs
      .as_mut()
      .unwrap()
      .push((name.into(), value));
    self
  }

  /// Add multiple shader definitions from a Vec
  pub fn add_shader_defs(
    &mut self,
    defs: Vec<(String, naga_oil::compose::ShaderDefValue)>,
  ) -> &mut Self {
    match self.shader_defs.as_mut() {
      Some(existing) => existing.extend(defs),
      None => self.shader_defs = Some(defs),
    }
    self
  }

  fn merge_struct_type_overrides(&mut self) {
    let struct_mappings = self
      .override_struct
      .iter()
      .flatten()
      .map(
        |OverrideStruct {
           from,
           to,
           alignment,
         }| {
          let wgsl_type = WgslType::Struct {
            fully_qualified_name: from.clone(),
          };
          // For struct overrides, we don't know the exact size/alignment, so use placeholders
          // These will be calculated later when the struct is actually used
          (wgsl_type, WgslTypeInfo::new(to.clone(), *alignment))
        },
      )
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
