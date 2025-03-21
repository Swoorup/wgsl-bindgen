mod raw_shader_bind_group;
mod single_bind_group;

use std::collections::BTreeMap;

use generate::quote_shader_stages;
use quote::{format_ident, quote};
use quote_gen::{demangle_and_fully_qualify_str, rust_type};
pub use raw_shader_bind_group::{get_bind_group_data_for_entry, RawShadersBindGroups};
use single_bind_group::SingleBindGroupBuilder;
pub use single_bind_group::SingleBindGroupData;
use smol_str::{SmolStr, ToSmolStr};

use crate::quote_gen::{
  RustSourceItem, RustSourceItemCategory, RustSourceItemPath, MOD_REFERENCE_ROOT,
};
use crate::wgsl::buffer_binding_type;
use crate::*;

/// A collection of bind groups that are common to all entrypoints.
struct CommonShaderBindGroups<'a> {
  containing_module: SmolStr,
  bind_group_data: BTreeMap<u32, SingleBindGroupData<'a>>,
}

#[derive(Clone, Eq, Copy, PartialEq, Ord, PartialOrd, Hash)]
pub enum ShaderBindGroupRefKind {
  Common,
  Entrypoint,
}

pub struct ShaderBindGroupRef<'a> {
  pub kind: ShaderBindGroupRefKind,
  pub data: SingleBindGroupData<'a>,
}

pub struct ShaderEntryBindGroups<'a> {
  pub containing_module: SmolStr,
  pub shader_stages: wgpu::ShaderStages,
  pub bind_group_ref: BTreeMap<u32, ShaderBindGroupRef<'a>>,
  pub original_bind_group: BTreeMap<u32, SingleBindGroupData<'a>>,
}

pub struct ReusableShaderBindGroups<'a> {
  common_bind_groups: FastIndexMap<SmolStr, CommonShaderBindGroups<'a>>,
  pub entrypoint_bindgroups: FastIndexMap<SmolStr, ShaderEntryBindGroups<'a>>,
}

impl<'a> ReusableShaderBindGroups<'a> {
  pub fn new() -> Self {
    Self {
      common_bind_groups: FastIndexMap::default(),
      entrypoint_bindgroups: FastIndexMap::default(),
    }
  }

  pub fn generate_bind_groups(&self, options: &WgslBindgenOption) -> Vec<RustSourceItem> {
    let mut items = Vec::new();
    // generate the common single bind groups.
    for common_bind_groups in self.common_bind_groups.values() {
      for (&group_no, group_data) in &common_bind_groups.bind_group_data {
        let builder = SingleBindGroupBuilder {
          containing_module: &common_bind_groups.containing_module,
          group_no,
          group_data,
          options,
        };
        items.push(builder.build());
      }
    }

    // generate the entrypoint single bind groups.
    for (_, shader) in &self.entrypoint_bindgroups {
      for (&group_no, group_ref) in &shader.bind_group_ref {
        // skip common bind groups.
        if group_ref.kind == ShaderBindGroupRefKind::Common {
          continue;
        }

        let builder = SingleBindGroupBuilder {
          containing_module: &shader.containing_module,
          group_no,
          group_data: &group_ref.data,
          options,
        };
        items.push(builder.build());
      }
    }

    // generate the bind groups module extras.
    for (_, shader) in &self.entrypoint_bindgroups {
      items.extend(generate_bind_groups_module_extras(
        &shader.containing_module,
        options,
        &shader.bind_group_ref,
        shader.shader_stages,
      ));
    }

    items
  }
}

fn generate_bind_groups_module_extras(
  invoking_entry_module: &str,
  options: &WgslBindgenOption,
  bind_group_data: &BTreeMap<u32, ShaderBindGroupRef<'_>>,
  shader_stages: wgpu::ShaderStages,
) -> Vec<RustSourceItem> {
  let bind_group_fields: Vec<_> = bind_group_data
    .iter()
    .map(|(group_no, group_ref)| {
      let group_name = options
        .wgpu_binding_generator
        .bind_group_layout
        .bind_group_name_ident(*group_no);

      let group_name = match group_ref.kind {
        ShaderBindGroupRefKind::Common => {
          let containing_module = group_ref.data.first_module();
          let path = RustSourceItemPath::new(containing_module, group_name.to_smolstr());
          quote!(#path)
        }
        ShaderBindGroupRefKind::Entrypoint => quote!(#group_name),
      };

      let field = indexed_name_ident("bind_group", *group_no);
      quote!(pub #field: &'a #group_name)
    })
    .collect();

  let has_compute = shader_stages.contains(wgpu::ShaderStages::COMPUTE);
  let has_render = shader_stages.contains(wgpu::ShaderStages::VERTEX_FRAGMENT)
    || shader_stages.contains(wgpu::ShaderStages::FRAGMENT)
    || shader_stages.contains(wgpu::ShaderStages::VERTEX);

  // The set function for each bind group already sets the index.
  let set_groups: Vec<_> = bind_group_data
    .keys()
    .map(|group_no| {
      let group = indexed_name_ident("bind_group", *group_no);
      quote!(#group.set(pass);)
    })
    .collect();

  if bind_group_data.is_empty() {
    // Don't include empty modules.
    vec![]
  } else {
    let bind_group_trait = RustSourceItem::new(
      RustSourceItemCategory::TypeDefs.into(),
      RustSourceItemPath::new(MOD_REFERENCE_ROOT.into(), "SetBindGroup".into()),
      quote! {
        pub trait SetBindGroup {
          fn set_bind_group(
              &mut self,
              index: u32,
              bind_group: &wgpu::BindGroup,
              offsets: &[wgpu::DynamicOffset],
          );
        }
      },
    );

    let mut set_bind_group_impls = Vec::new();
    if has_compute {
      set_bind_group_impls.push(RustSourceItem::new(
        RustSourceItemCategory::TraitImpls.into(),
        RustSourceItemPath::new(
          MOD_REFERENCE_ROOT.into(),
          "impl SetBindGroup for wgpu::ComputePass<'_>".into(),
        ),
        quote! {
          impl SetBindGroup for wgpu::ComputePass<'_> {
            fn set_bind_group(
              &mut self,
              index: u32,
              bind_group: &wgpu::BindGroup,
              offsets: &[wgpu::DynamicOffset],
            ) {
                self.set_bind_group(index, bind_group, offsets);
            }
          }
        },
      ));
    }

    if has_render {
      set_bind_group_impls.extend([
        RustSourceItem::new(
          RustSourceItemCategory::TraitImpls.into(),
          RustSourceItemPath::new(
            MOD_REFERENCE_ROOT.into(),
            "impl SetBindGroup for wgpu::RenderPass<'_>".into(),
          ),
          quote! {
            impl SetBindGroup for wgpu::RenderPass<'_> {
              fn set_bind_group(
                &mut self,
                index: u32,
                bind_group: &wgpu::BindGroup,
                offsets: &[wgpu::DynamicOffset],
              ) {
                  self.set_bind_group(index, bind_group, offsets);
              }
            }
          },
        ),
        RustSourceItem::new(
          RustSourceItemCategory::TraitImpls.into(),
          RustSourceItemPath::new(
            MOD_REFERENCE_ROOT.into(),
            "impl SetBindGroup for wgpu::RenderBundleEncoder<'_>".into(),
          ),
          quote! {
            impl SetBindGroup for wgpu::RenderBundleEncoder<'_> {
              fn set_bind_group(
                &mut self,
                index: u32,
                bind_group: &wgpu::BindGroup,
                offsets: &[wgpu::DynamicOffset],
              ) {
                  self.set_bind_group(index, bind_group, offsets);
              }
            }
          },
        ),
      ]);
    };

    let entry_bind_groups = RustSourceItem::new(
      RustSourceItemCategory::TypeDefs | RustSourceItemCategory::TypeImpls,
      RustSourceItemPath::new(invoking_entry_module.into(), "WgpuBindGroups".into()),
      quote! {
        #[doc = " Bind groups can be set individually using their set(render_pass) method, or all at once using `WgpuBindGroups::set`."]
        #[doc = " For optimal performance with many draw calls, it's recommended to organize bindings into bind groups based on update frequency:"]
        #[doc = "   - Bind group 0: Least frequent updates (e.g. per frame resources)"]
        #[doc = "   - Bind group 1: More frequent updates"]
        #[doc = "   - Bind group 2: More frequent updates"]
        #[doc = "   - Bind group 3: Most frequent updates (e.g. per draw resources)"]
        #[derive(Debug, Copy, Clone)]
        pub struct WgpuBindGroups<'a> {
            #(#bind_group_fields),*
        }

        impl<'a> WgpuBindGroups<'a> {
            pub fn set(&self, pass: &mut impl SetBindGroup) {
                #(self.#set_groups)*
            }
        }
      },
    );

    [bind_group_trait, entry_bind_groups]
      .into_iter()
      .chain(set_bind_group_impls)
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use indoc::indoc;

  use super::*;
  use crate::assert_tokens_eq;
  use crate::bind_group::raw_shader_bind_group::RawShaderEntryBindGroups;

  #[test]
  #[ignore = "TODO: Failing due to unhandled BindingType for vec4<f32> like cases"]
  fn bind_group_data_consecutive_bind_groups() {
    let source = indoc! {r#"
            @group(0) @binding(0) var<uniform> a: vec4<f32>;
            @group(1) @binding(0) var<uniform> b: vec4<f32>;
            @group(2) @binding(0) var<uniform> c: vec4<f32>;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert_eq!(
      3,
      get_bind_group_data_for_entry(
        &module,
        wgpu::ShaderStages::NONE,
        &WgslBindgenOption::default(),
        "test"
      )
      .unwrap()
      .bind_group_data
      .len()
    );
  }

  #[test]
  #[ignore = "TODO: Failing due to unhandled BindingType for vec4<f32> like cases"]
  fn bind_group_data_first_group_not_zero() {
    let source = indoc! {r#"
            @group(1) @binding(0) var<uniform> a: vec4<f32>;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert!(matches!(
      get_bind_group_data_for_entry(
        &module,
        wgpu::ShaderStages::FRAGMENT,
        &WgslBindgenOption::default(),
        "test"
      ),
      Err(CreateModuleError::NonConsecutiveBindGroups)
    ));
  }

  #[test]
  #[ignore = "TODO: Failing due to unhandled BindingType for vec4<f32> like cases"]
  fn bind_group_data_non_consecutive_bind_groups() {
    let source = indoc! {r#"
            @group(0) @binding(0) var<uniform> a: vec4<f32>;
            @group(1) @binding(0) var<uniform> b: vec4<f32>;
            @group(3) @binding(0) var<uniform> c: vec4<f32>;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert!(matches!(
      get_bind_group_data_for_entry(
        &module,
        wgpu::ShaderStages::NONE,
        &WgslBindgenOption::default(),
        "test"
      ),
      Err(CreateModuleError::NonConsecutiveBindGroups)
    ));
  }

  fn generate_test_bind_groups_module(
    bind_group_data: &BTreeMap<u32, SingleBindGroupData>,
    shader_stages: wgpu::ShaderStages,
    options: &WgslBindgenOption,
  ) -> TokenStream {
    let raw_shader_entry_bind_groups = RawShaderEntryBindGroups {
      containing_module: "test".into(),
      shader_stages,
      bind_group_data: bind_group_data.clone(),
    };

    let mut raw_shaders_bind_groups = RawShadersBindGroups::new(options);
    raw_shaders_bind_groups.add(raw_shader_entry_bind_groups);
    let items = raw_shaders_bind_groups
      .create_reusable_shader_bind_groups()
      .generate_bind_groups(&WgslBindgenOption::default());
    let all_matching = items
      .into_iter()
      .filter(|item| item.path.name.contains("WgpuBindGroup"))
      .map(|item| item.tokenstream)
      .collect::<Vec<_>>();

    quote!(#(#all_matching)*)
  }

  #[test]
  #[ignore = "TODO: Failing due to unhandled BindingType for vec4<f32> like cases"]
  fn bind_groups_module_compute() {
    let source = indoc! {r#"
            struct VertexInput0 {};
            struct VertexWeight {};
            struct Vertices {};
            struct VertexWeights {};
            struct Transforms {};

            @group(0) @binding(0) var<storage, read> src: array<vec4<f32>>;
            @group(0) @binding(1) var<storage, read> vertex_weights: VertexWeights;
            @group(0) @binding(2) var<storage, read_write> dst: Vertices;

            @group(1) @binding(0) var<uniform> transforms: Transforms;

            @compute
            @workgroup_size(64)
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let options = WgslBindgenOption::default();
    let bind_group_data =
      get_bind_group_data_for_entry(&module, wgpu::ShaderStages::NONE, &options, "test")
        .unwrap()
        .bind_group_data;

    let actual = generate_test_bind_groups_module(
      &bind_group_data,
      wgpu::ShaderStages::COMPUTE,
      &options,
    );

    assert_tokens_eq!(
      quote! {
          #[derive(Debug)]
          pub struct WgpuBindGroup0EntriesParams<'a> {
              pub src: wgpu::BufferBinding<'a>,
              pub vertex_weights: wgpu::BufferBinding<'a>,
              pub dst: wgpu::BufferBinding<'a>,
          }
          #[derive(Clone, Debug)]
          pub struct WgpuBindGroup0Entries<'a> {
              pub src: wgpu::BindGroupEntry<'a>,
              pub vertex_weights: wgpu::BindGroupEntry<'a>,
              pub dst: wgpu::BindGroupEntry<'a>,
          }
          impl<'a> WgpuBindGroup0Entries<'a> {
            pub fn new(params: WgpuBindGroup0EntriesParams<'a>) -> Self {
              Self {
                  src: wgpu::BindGroupEntry {
                      binding: 0,
                      resource: wgpu::BindingResource::Buffer(params.src),
                  },
                  vertex_weights: wgpu::BindGroupEntry {
                      binding: 1,
                      resource: wgpu::BindingResource::Buffer(params.vertex_weights),
                  },
                  dst: wgpu::BindGroupEntry {
                      binding: 2,
                      resource: wgpu::BindingResource::Buffer(params.dst),
                  },
              }
            }
            pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 3] {
              [ self.src, self.vertex_weights, self.dst ]
            }
            pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
                self.as_array().into_iter().collect()
            }
          }
          #[derive(Debug)]
          pub struct WgpuBindGroup0(wgpu::BindGroup);
          impl WgpuBindGroup0 {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                label: Some("Test::BindGroup0::LayoutDescriptor"),
                entries: &[
                    /// @binding(0): "src"
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: true,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    /// @binding(1): "vertex_weights"
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: true,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                              std::mem::size_of::<_root::test::VertexWeights>() as _,
                            ),
                        },
                        count: None,
                    },
                    /// @binding(2): "dst"
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: false,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                              std::mem::size_of::<_root::test::Vertices>() as _,
                            ),
                        },
                        count: None,
                    },
                ],
            };
              pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                  device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
              }
              pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroup0Entries) -> Self {
                  let bind_group_layout = Self::get_bind_group_layout(&device);
                  let entries = bindings.as_array();
                  let bind_group = device
                      .create_bind_group(
                          &wgpu::BindGroupDescriptor {
                              label: Some("Test::BindGroup0"),
                              layout: &bind_group_layout,
                              entries: &entries,
                          },
                      );
                  Self(bind_group)
              }
              pub fn set(&self, pass: &mut impl SetBindGroup) {
                  pass.set_bind_group(0, &self.0, &[]);
              }
          }
          #[derive(Debug)]
          pub struct WgpuBindGroup1EntriesParams<'a> {
              pub transforms: wgpu::BufferBinding<'a>,
          }
          #[derive(Clone, Debug)]
          pub struct WgpuBindGroup1Entries<'a> {
              pub transforms: wgpu::BindGroupEntry<'a>,
          }
          impl<'a> WgpuBindGroup1Entries<'a> {
            pub fn new(params: WgpuBindGroup1EntriesParams<'a>) -> Self {
              Self {
                  transforms: wgpu::BindGroupEntry {
                      binding: 0,
                      resource: wgpu::BindingResource::Buffer(params.transforms),
                  },
              }
            }
            pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 1] {
              [ self.transforms ]
            }
            pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
                self.as_array().into_iter().collect()
            }
          }
          #[derive(Debug)]
          pub struct WgpuBindGroup1(wgpu::BindGroup);
          impl WgpuBindGroup1 {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                label: Some("Test::BindGroup1::LayoutDescriptor"),
                entries: &[
                    /// @binding(0): "transforms"
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                              std::mem::size_of::<_root::test::Transforms>() as _,
                            ),
                        },
                        count: None,
                    },
                ],
            };

              pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                  device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
              }
              pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroup1Entries) -> Self {
                  let bind_group_layout = Self::get_bind_group_layout(&device);
                  let entries = bindings.as_array();
                  let bind_group = device
                      .create_bind_group(
                          &wgpu::BindGroupDescriptor {
                              label: Some("Test::BindGroup1"),
                              layout: &bind_group_layout,
                              entries: &entries,
                          },
                      );
                  Self(bind_group)
              }
              pub fn set(&self, pass: &mut impl SetBindGroup) {
                  pass.set_bind_group(1, &self.0, &[]);
              }
          }
          /// Bind groups can be set individually using their set(render_pass) method, or all at once using `WgpuBindGroups::set`.
          /// For optimal performance with many draw calls, it's recommended to organize bindings into bind groups based on update frequency:
          ///   - Bind group 0: Least frequent updates (e.g. per frame resources)
          ///   - Bind group 1: More frequent updates
          ///   - Bind group 2: More frequent updates
          ///   - Bind group 3: Most frequent updates (e.g. per draw resources)
          #[derive(Debug, Copy, Clone)]
          pub struct WgpuBindGroups<'a> {
              pub bind_group0: &'a WgpuBindGroup0,
              pub bind_group1: &'a WgpuBindGroup1,
          }
          impl<'a> WgpuBindGroups<'a> {
              pub fn set(&self, pass: &mut impl SetBindGroup) {
                  self.bind_group0.set(pass);
                  self.bind_group1.set(pass);
              }
          }
      },
      actual
    );
  }

  #[test]
  fn bind_groups_module_vertex_fragment() {
    // Test different texture and sampler types.
    // TODO: Storage textures.
    let source = indoc! {r#"
            struct Transforms {};

            @group(0) @binding(0)
            var color_texture: texture_2d<f32>;
            @group(0) @binding(1)
            var color_texture_i32: texture_2d<i32>;
            @group(0) @binding(2)
            var color_texture_u32: texture_2d<u32>;
            @group(0) @binding(3)
            var color_sampler: sampler;
            @group(0) @binding(4)
            var depth_texture: texture_depth_2d;
            @group(0) @binding(5)
            var comparison_sampler: sampler_comparison;

            @group(0) @binding(6)
            var storage_tex_read: texture_storage_2d<r32float, read>;
            @group(0) @binding(7)
            var storage_tex_write: texture_storage_2d<rg32sint, write>;
            @group(0) @binding(8)
            var storage_tex_read_write: texture_storage_2d<rgba8uint, read_write>;

            @group(0) @binding(9)
            var color_texture_msaa: texture_multisampled_2d<f32>;
            @group(0) @binding(10)
            var depth_texture_msaa: texture_depth_multisampled_2d;

            @group(1) @binding(0) var<uniform> transforms: Transforms;
            @group(1) @binding(1) var<uniform> one: f32;

            @vertex
            fn vs_main() {}

            @fragment
            fn fs_main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let options = WgslBindgenOption::default();
    let bind_group_data = get_bind_group_data_for_entry(
      &module,
      wgpu::ShaderStages::VERTEX_FRAGMENT,
      &options,
      "test",
    )
    .unwrap()
    .bind_group_data;

    let actual = generate_test_bind_groups_module(
      &bind_group_data,
      wgpu::ShaderStages::VERTEX_FRAGMENT,
      &options,
    );

    // TODO: Are storage buffers valid for vertex/fragment?
    assert_tokens_eq!(
      quote! {
          #[derive(Debug)]
          pub struct WgpuBindGroup0EntriesParams<'a> {
              pub color_texture: &'a wgpu::TextureView,
              pub color_texture_i32: &'a wgpu::TextureView,
              pub color_texture_u32: &'a wgpu::TextureView,
              pub color_sampler: &'a wgpu::Sampler,
              pub depth_texture: &'a wgpu::TextureView,
              pub comparison_sampler: &'a wgpu::Sampler,
              pub storage_tex_read: &'a wgpu::TextureView,
              pub storage_tex_write: &'a wgpu::TextureView,
              pub storage_tex_read_write: &'a wgpu::TextureView,
              pub color_texture_msaa: &'a wgpu::TextureView,
              pub depth_texture_msaa: &'a wgpu::TextureView,
          }
          #[derive(Clone, Debug)]
          pub struct WgpuBindGroup0Entries<'a> {
              pub color_texture: wgpu::BindGroupEntry<'a>,
              pub color_texture_i32: wgpu::BindGroupEntry<'a>,
              pub color_texture_u32: wgpu::BindGroupEntry<'a>,
              pub color_sampler: wgpu::BindGroupEntry<'a>,
              pub depth_texture: wgpu::BindGroupEntry<'a>,
              pub comparison_sampler: wgpu::BindGroupEntry<'a>,
              pub storage_tex_read: wgpu::BindGroupEntry<'a>,
              pub storage_tex_write: wgpu::BindGroupEntry<'a>,
              pub storage_tex_read_write: wgpu::BindGroupEntry<'a>,
              pub color_texture_msaa: wgpu::BindGroupEntry<'a>,
              pub depth_texture_msaa: wgpu::BindGroupEntry<'a>,
          }
          impl<'a> WgpuBindGroup0Entries<'a> {
            pub fn new(params: WgpuBindGroup0EntriesParams<'a>) -> Self {
              Self {
                color_texture: wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        params.color_texture,
                    ),
                },
                color_texture_i32: wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        params.color_texture_i32,
                    ),
                },
                color_texture_u32: wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        params.color_texture_u32,
                    ),
                },
                color_sampler: wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(
                        params.color_sampler,
                    ),
                },
                depth_texture: wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(
                        params.depth_texture,
                    ),
                },
                comparison_sampler: wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(
                        params.comparison_sampler,
                    ),
                },
                storage_tex_read: wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(
                        params.storage_tex_read,
                    ),
                },
                storage_tex_write: wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(
                        params.storage_tex_write,
                    ),
                },
                storage_tex_read_write: wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(
                        params.storage_tex_read_write,
                    ),
                },
                color_texture_msaa: wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(
                        params.color_texture_msaa,
                    ),
                },
                depth_texture_msaa: wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::TextureView(
                        params.depth_texture_msaa,
                    ),
                },

              }
            }
            pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 11] {
              [
                self.color_texture,
                self.color_texture_i32,
                self.color_texture_u32,
                self.color_sampler,
                self.depth_texture,
                self.comparison_sampler,
                self.storage_tex_read,
                self.storage_tex_write,
                self.storage_tex_read_write,
                self.color_texture_msaa,
                self.depth_texture_msaa,
              ]
            }
            pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
              self.as_array().into_iter().collect()
            }
          }
          #[derive(Debug)]
          pub struct WgpuBindGroup0(wgpu::BindGroup);
          impl WgpuBindGroup0 {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                label: Some("Test::BindGroup0::LayoutDescriptor"),
                entries: &[
                    /// @binding(0): "color_texture"
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    /// @binding(1): "color_texture_i32"
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Sint,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    /// @binding(2): "color_texture_u32"
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Uint,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    /// @binding(3): "color_sampler"
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    /// @binding(4): "depth_texture"
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    /// @binding(5): "comparison_sampler"
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    /// @binding(6): "storage_tex_read"
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadOnly,
                            format: wgpu::TextureFormat::R32Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    /// @binding(7): "storage_tex_write"
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rg32Sint,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    /// @binding(8): "storage_tex_read_write"
                    wgpu::BindGroupLayoutEntry {
                        binding: 8,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadWrite,
                            format: wgpu::TextureFormat::Rgba8Uint,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    /// @binding(9): "color_texture_msaa"
                    wgpu::BindGroupLayoutEntry {
                        binding: 9,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                    /// @binding(10): "depth_texture_msaa"
                    wgpu::BindGroupLayoutEntry {
                        binding: 10,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                ],
            };
              pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                  device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
              }
              pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroup0Entries) -> Self {
                  let bind_group_layout = Self::get_bind_group_layout(&device);
                  let entries = bindings.as_array();
                  let bind_group = device
                      .create_bind_group(
                          &wgpu::BindGroupDescriptor {
                              label: Some("Test::BindGroup0"),
                              layout: &bind_group_layout,
                              entries: &entries,
                          },
                      );
                  Self(bind_group)
              }
              pub fn set(&self, pass: &mut impl SetBindGroup) {
                  pass.set_bind_group(0, &self.0, &[]);
              }
          }
          #[derive(Debug)]
          pub struct WgpuBindGroup1EntriesParams<'a> {
              pub transforms: wgpu::BufferBinding<'a>,
              pub one: wgpu::BufferBinding<'a>,
          }
          #[derive(Clone, Debug)]
          pub struct WgpuBindGroup1Entries<'a> {
              pub transforms: wgpu::BindGroupEntry<'a>,
              pub one: wgpu::BindGroupEntry<'a>,
          }
          impl<'a> WgpuBindGroup1Entries<'a> {
            pub fn new(params: WgpuBindGroup1EntriesParams<'a>) -> Self {
              Self {
                transforms: wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(params.transforms),
                },
                one: wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(params.one),
                },
              }
            }
            pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 2] {
              [ self.transforms, self.one ]
            }
            pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
              self.as_array().into_iter().collect()
            }
          }
          #[derive(Debug)]
          pub struct WgpuBindGroup1(wgpu::BindGroup);
          impl WgpuBindGroup1 {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                label: Some("Test::BindGroup1::LayoutDescriptor"),
                entries: &[
                    /// @binding(0): "transforms"
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                              std::mem::size_of::<_root::test::Transforms>() as _,
                            ),
                        },
                        count: None,
                    },
                    /// @binding(1): "one"
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                              std::mem::size_of::<f32>() as _,
                            ),
                        },
                        count: None,
                    },
                ],
            };
              pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                  device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
              }
              pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroup1Entries) -> Self {
                  let bind_group_layout = Self::get_bind_group_layout(&device);
                  let entries = bindings.as_array();
                  let bind_group = device
                      .create_bind_group(
                          &wgpu::BindGroupDescriptor {
                              label: Some("Test::BindGroup1"),
                              layout: &bind_group_layout,
                              entries: &entries,
                          },
                      );
                  Self(bind_group)
              }
              pub fn set(&self, pass: &mut impl SetBindGroup) {
                  pass.set_bind_group(1, &self.0, &[]);
              }
          }
          /// Bind groups can be set individually using their set(render_pass) method, or all at once using `WgpuBindGroups::set`.
          /// For optimal performance with many draw calls, it's recommended to organize bindings into bind groups based on update frequency:
          ///   - Bind group 0: Least frequent updates (e.g. per frame resources)
          ///   - Bind group 1: More frequent updates
          ///   - Bind group 2: More frequent updates
          ///   - Bind group 3: Most frequent updates (e.g. per draw resources)
          #[derive(Debug, Copy, Clone)]
          pub struct WgpuBindGroups<'a> {
              pub bind_group0: &'a WgpuBindGroup0,
              pub bind_group1: &'a WgpuBindGroup1,
          }
          impl<'a> WgpuBindGroups<'a> {
              pub fn set(&self, pass: &mut impl SetBindGroup) {
                  self.bind_group0.set(pass);
                  self.bind_group1.set(pass);
              }
          }
      },
      actual
    );
  }

  #[test]
  fn bind_groups_module_vertex() {
    // The actual content of the structs doesn't matter.
    // We only care about the groups and bindings.
    let source = indoc! {r#"
            struct Transforms {};

            @group(0) @binding(0) var<uniform> transforms: Transforms;

            @vertex
            fn vs_main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let options = WgslBindgenOption::default();
    let bind_group_data = get_bind_group_data_for_entry(
      &module,
      wgpu::ShaderStages::VERTEX,
      &options,
      "test",
    )
    .unwrap()
    .bind_group_data;

    let actual = generate_test_bind_groups_module(
      &bind_group_data,
      wgpu::ShaderStages::VERTEX,
      &options,
    );

    assert_tokens_eq!(
      quote! {
          #[derive(Debug)]
          pub struct WgpuBindGroup0EntriesParams<'a> {
              pub transforms: wgpu::BufferBinding<'a>,
          }
          #[derive(Clone, Debug)]
          pub struct WgpuBindGroup0Entries<'a> {
              pub transforms: wgpu::BindGroupEntry<'a>,
          }
          impl<'a> WgpuBindGroup0Entries<'a> {
            pub fn new(params: WgpuBindGroup0EntriesParams<'a>) -> Self {
              Self {
                  transforms: wgpu::BindGroupEntry {
                      binding: 0,
                      resource: wgpu::BindingResource::Buffer(params.transforms),
                  },
              }
            }
            pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 1] {
              [
                self.transforms,
              ]
            }
            pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
                self.as_array().into_iter().collect()
            }
          }
          #[derive(Debug)]
          pub struct WgpuBindGroup0(wgpu::BindGroup);
          impl WgpuBindGroup0 {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                label: Some("Test::BindGroup0::LayoutDescriptor"),
                entries: &[
                    /// @binding(0): "transforms"
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                              std::mem::size_of::<_root::test::Transforms>() as _,
                            ),
                        },
                        count: None,
                    },
                ],
            };
              pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                  device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
              }
              pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroup0Entries) -> Self {
                  let bind_group_layout = Self::get_bind_group_layout(&device);
                  let entries = bindings.as_array();
                  let bind_group = device
                      .create_bind_group(
                          &wgpu::BindGroupDescriptor {
                              label: Some("Test::BindGroup0"),
                              layout: &bind_group_layout,
                              entries: &entries,
                          },
                      );
                  Self(bind_group)
              }
              pub fn set(&self, pass: &mut impl SetBindGroup) {
                  pass.set_bind_group(0, &self.0, &[]);
              }
          }
          /// Bind groups can be set individually using their set(render_pass) method, or all at once using `WgpuBindGroups::set`.
          /// For optimal performance with many draw calls, it's recommended to organize bindings into bind groups based on update frequency:
          ///   - Bind group 0: Least frequent updates (e.g. per frame resources)
          ///   - Bind group 1: More frequent updates
          ///   - Bind group 2: More frequent updates
          ///   - Bind group 3: Most frequent updates (e.g. per draw resources)
          #[derive(Debug, Copy, Clone)]
          pub struct WgpuBindGroups<'a> {
              pub bind_group0: &'a WgpuBindGroup0,
          }
          impl<'a> WgpuBindGroups<'a> {
              pub fn set(&self, pass: &mut impl SetBindGroup) {
                  self.bind_group0.set(pass);
              }
          }
      },
      actual
    );
  }

  #[test]
  fn bind_groups_module_fragment() {
    // The actual content of the structs doesn't matter.
    // We only care about the groups and bindings.
    let source = indoc! {r#"
            struct Transforms {};

            @group(0) @binding(0) var<uniform> transforms: Transforms;

            @fragment
            fn fs_main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let options = WgslBindgenOption::default();
    let bind_group_data = get_bind_group_data_for_entry(
      &module,
      wgpu::ShaderStages::FRAGMENT,
      &options,
      "test",
    )
    .unwrap()
    .bind_group_data;

    let actual = generate_test_bind_groups_module(
      &bind_group_data,
      wgpu::ShaderStages::FRAGMENT,
      &options,
    );

    assert_tokens_eq!(
      quote! {
          #[derive(Debug)]
          pub struct WgpuBindGroup0EntriesParams<'a> {
              pub transforms: wgpu::BufferBinding<'a>,
          }
          #[derive(Clone, Debug)]
          pub struct WgpuBindGroup0Entries<'a> {
              pub transforms: wgpu::BindGroupEntry<'a>,
          }
          impl<'a> WgpuBindGroup0Entries<'a> {
            pub fn new(params: WgpuBindGroup0EntriesParams<'a>) -> Self {
              Self {
                  transforms: wgpu::BindGroupEntry {
                      binding: 0,
                      resource: wgpu::BindingResource::Buffer(params.transforms),
                  },
              }
            }
            pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 1] {
              [ self.transforms ]
            }
            pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
              self.as_array().into_iter().collect()
            }
          }
          #[derive(Debug)]
          pub struct WgpuBindGroup0(wgpu::BindGroup);
          impl WgpuBindGroup0 {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
              label: Some("Test::BindGroup0::LayoutDescriptor"),
              entries: &[
                  /// @binding(0): "transforms"
                  wgpu::BindGroupLayoutEntry {
                      binding: 0,
                      visibility: wgpu::ShaderStages::FRAGMENT,
                      ty: wgpu::BindingType::Buffer {
                          ty: wgpu::BufferBindingType::Uniform,
                          has_dynamic_offset: false,
                          min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<_root::test::Transforms>() as _,
                          ),
                      },
                      count: None,
                  },
              ],
            };

              pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                  device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
              }
              pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroup0Entries) -> Self {
                  let bind_group_layout = Self::get_bind_group_layout(&device);
                  let entries = bindings.as_array();
                  let bind_group = device
                      .create_bind_group(
                          &wgpu::BindGroupDescriptor {
                              label: Some("Test::BindGroup0"),
                              layout: &bind_group_layout,
                              entries: &entries,
                          },
                      );
                  Self(bind_group)
              }
              pub fn set(&self, pass: &mut impl SetBindGroup) {
                  pass.set_bind_group(0, &self.0, &[]);
              }
          }
          /// Bind groups can be set individually using their set(render_pass) method, or all at once using `WgpuBindGroups::set`.
          /// For optimal performance with many draw calls, it's recommended to organize bindings into bind groups based on update frequency:
          ///   - Bind group 0: Least frequent updates (e.g. per frame resources)
          ///   - Bind group 1: More frequent updates
          ///   - Bind group 2: More frequent updates
          ///   - Bind group 3: Most frequent updates (e.g. per draw resources)
          #[derive(Debug, Copy, Clone)]
          pub struct WgpuBindGroups<'a> {
              pub bind_group0: &'a WgpuBindGroup0,
          }
          impl<'a> WgpuBindGroups<'a> {
              pub fn set(&self, pass: &mut impl SetBindGroup) {
                  self.bind_group0.set(pass);
              }
          }
      },
      actual
    );
  }
}
