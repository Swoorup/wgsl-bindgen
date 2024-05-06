use std::collections::BTreeMap;

use derive_more::Constructor;
use quote::{format_ident, quote};

use crate::wgsl::buffer_binding_type;
use crate::*;

mod layout_builder;
use layout_builder::*;

pub struct GroupData<'a> {
  pub bindings: Vec<GroupBinding<'a>>,
}

pub struct GroupBinding<'a> {
  pub name: Option<String>,
  pub binding_index: u32,
  pub binding_type: &'a naga::Type,
  pub address_space: naga::AddressSpace,
}

#[derive(Constructor)]
struct BindGroupBuilder<'a> {
  entry_name: &'a str,
  group_no: u32,
  data: &'a GroupData<'a>,
  shader_stages: wgpu::ShaderStages,
  wgpu_generator: &'a BindGroupLayoutGenerator,
}

impl<'a> BindGroupBuilder<'a> {
  fn bind_group_layout_descriptor(&self) -> TokenStream {
    let entries: Vec<_> = self
      .data
      .bindings
      .iter()
      .map(|binding| bind_group_layout_entry(binding, self.shader_stages))
      .collect();

    let bind_group_label =
      format!("{}::BindGroup{}::LayoutDescriptor", self.entry_name, self.group_no);

    quote! {
        wgpu::BindGroupLayoutDescriptor {
            label: Some(#bind_group_label),
            entries: &[
                #(#entries),*
            ],
        }
    }
  }

  fn struct_name(&self) -> syn::Ident {
    indexed_name_ident("WgpuBindGroup", self.group_no)
  }

  fn bind_group_struct_impl(&self) -> TokenStream {
    // TODO: Support compute shader with vertex/fragment in the same module?
    let is_compute = self.shader_stages == wgpu::ShaderStages::COMPUTE;

    let render_pass = if is_compute {
      quote!(wgpu::ComputePass<'a>)
    } else {
      quote!(wgpu::RenderPass<'a>)
    };

    let bind_group_name = self.struct_name();
    let bind_group_layout_name =
      indexed_name_ident(&self.wgpu_generator.layout_prefix_name, self.group_no);

    let bind_group_layout_descriptor = self.bind_group_layout_descriptor();

    let group_no = Index::from(self.group_no as usize);
    let bind_group_label = format!("{}::BindGroup{}", self.entry_name, self.group_no);

    quote! {
        impl #bind_group_name {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = #bind_group_layout_descriptor;

            pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
            }

            pub fn from_bindings(device: &wgpu::Device, bindings: #bind_group_layout_name) -> Self {
                let bind_group_layout = Self::get_bind_group_layout(&device);
                let entries = bindings.entries();
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(#bind_group_label),
                    layout: &bind_group_layout,
                    entries: &entries,
                });
                Self(bind_group)
            }

            pub fn set<'a>(&'a self, render_pass: &mut #render_pass) {
                render_pass.set_bind_group(#group_no, &self.0, &[]);
            }
        }
    }
  }

  fn build(self) -> TokenStream {
    let bind_group_name = self.struct_name();

    let group_struct = quote! {
        #[derive(Debug)]
        pub struct #bind_group_name(wgpu::BindGroup);
    };

    let group_impl = self.bind_group_struct_impl();

    quote! {
        #group_struct
        #group_impl
    }
  }
}

// TODO: Take an iterator instead?
pub fn bind_groups_module(
  invoking_entry_module: &str,
  options: &WgslBindgenOption,
  bind_group_data: &BTreeMap<u32, GroupData>,
  shader_stages: wgpu::ShaderStages,
) -> TokenStream {
  let entry_name = sanitize_and_pascal_case(invoking_entry_module);
  let bind_groups: Vec<_> = bind_group_data
    .iter()
    .map(|(group_no, group)| {
      let wgpu_generator = &options.wgpu_binding_generator;

      let wgpu_layout = BindGroupLayoutBuilder::new(
        invoking_entry_module,
        *group_no,
        group,
        &wgpu_generator.bind_group_layout,
      )
      .build();

      let additional_layout =
        if let Some(additional_generator) = &options.extra_binding_generator {
          BindGroupLayoutBuilder::new(
            invoking_entry_module,
            *group_no,
            group,
            &additional_generator.bind_group_layout,
          )
          .build()
        } else {
          quote!()
        };

      let bindgroup = BindGroupBuilder::new(
        &entry_name,
        *group_no,
        group,
        shader_stages,
        &wgpu_generator.bind_group_layout,
      )
      .build();

      quote! {
        #additional_layout
        #wgpu_layout
        #bindgroup
      }
    })
    .collect();

  let bind_group_fields: Vec<_> = bind_group_data
    .keys()
    .map(|group_no| {
      let group_name = indexed_name_ident("WgpuBindGroup", *group_no);
      let field = indexed_name_ident("bind_group", *group_no);
      quote!(pub #field: &'a #group_name)
    })
    .collect();

  // TODO: Support compute shader with vertex/fragment in the same module?
  let is_compute = shader_stages == wgpu::ShaderStages::COMPUTE;
  let render_pass = if is_compute {
    quote!(wgpu::ComputePass<'a>)
  } else {
    quote!(wgpu::RenderPass<'a>)
  };

  let group_parameters: Vec<_> = bind_group_data
    .keys()
    .map(|group_no| {
      let group = indexed_name_ident("bind_group", *group_no);
      let group_type = indexed_name_ident("WgpuBindGroup", *group_no);
      quote!(#group: &'a bind_groups::#group_type)
    })
    .collect();

  // The set function for each bind group already sets the index.
  let set_groups: Vec<_> = bind_group_data
    .keys()
    .map(|group_no| {
      let group = indexed_name_ident("bind_group", *group_no);
      quote!(#group.set(pass);)
    })
    .collect();

  let set_bind_groups = quote! {
      pub fn set_bind_groups<'a>(
          pass: &mut #render_pass,
          #(#group_parameters),*
      ) {
          #(#set_groups)*
      }
  };

  if bind_groups.is_empty() {
    // Don't include empty modules.
    quote!()
  } else {
    // Create a module to avoid name conflicts with user structs.
    quote! {
        pub mod bind_groups {
            #(#bind_groups)*

            #[derive(Debug, Copy, Clone)]
            pub struct WgpuBindGroups<'a> {
                #(#bind_group_fields),*
            }

            impl<'a> WgpuBindGroups<'a> {
                pub fn set(&self, pass: &mut #render_pass) {
                    #(self.#set_groups)*
                }
            }
        }
        #set_bind_groups
    }
  }
}

fn bind_group_layout_entry(
  binding: &GroupBinding,
  shader_stages: wgpu::ShaderStages,
) -> TokenStream {
  // TODO: Assume storage is only used for compute?
  // TODO: Support just vertex or fragment?
  // TODO: Visible from all stages?
  let stages = match shader_stages {
    wgpu::ShaderStages::VERTEX_FRAGMENT => quote!(wgpu::ShaderStages::VERTEX_FRAGMENT),
    wgpu::ShaderStages::COMPUTE => quote!(wgpu::ShaderStages::COMPUTE),
    wgpu::ShaderStages::VERTEX => quote!(wgpu::ShaderStages::VERTEX),
    wgpu::ShaderStages::FRAGMENT => quote!(wgpu::ShaderStages::FRAGMENT),
    _ => todo!(),
  };

  let binding_index = Index::from(binding.binding_index as usize);
  // TODO: Support more types.
  let binding_type = match binding.binding_type.inner {
    naga::TypeInner::Scalar(_)
    | naga::TypeInner::Struct { .. }
    | naga::TypeInner::Array { .. } => {
      let buffer_binding_type = buffer_binding_type(binding.address_space);

      quote!(wgpu::BindingType::Buffer {
          ty: #buffer_binding_type,
          has_dynamic_offset: false,
          min_binding_size: None,
      })
    }
    naga::TypeInner::Image { dim, class, .. } => {
      let view_dim = match dim {
        naga::ImageDimension::D1 => quote!(wgpu::TextureViewDimension::D1),
        naga::ImageDimension::D2 => quote!(wgpu::TextureViewDimension::D2),
        naga::ImageDimension::D3 => quote!(wgpu::TextureViewDimension::D3),
        naga::ImageDimension::Cube => quote!(wgpu::TextureViewDimension::Cube),
      };

      match class {
        naga::ImageClass::Sampled { kind, multi } => {
          let sample_type = match kind {
            naga::ScalarKind::Sint => quote!(wgpu::TextureSampleType::Sint),
            naga::ScalarKind::Uint => quote!(wgpu::TextureSampleType::Uint),
            naga::ScalarKind::Float => {
              quote!(wgpu::TextureSampleType::Float { filterable: false })
            }
            _ => panic!("Unsupported sample type: {kind:#?}"),
          };

          // TODO: Don't assume all textures are filterable.
          quote!(wgpu::BindingType::Texture {
              sample_type: #sample_type,
              view_dimension: #view_dim,
              multisampled: #multi,
          })
        }
        naga::ImageClass::Depth { multi } => {
          quote!(wgpu::BindingType::Texture {
              sample_type: wgpu::TextureSampleType::Depth,
              view_dimension: #view_dim,
              multisampled: #multi,
          })
        }
        naga::ImageClass::Storage { format, access } => {
          // TODO: Will the debug implementation always work with the macro?
          // Assume texture format variants are the same as storage formats.
          let format = syn::Ident::new(&format!("{format:?}"), Span::call_site());
          let storage_access = storage_access(access);

          quote!(wgpu::BindingType::StorageTexture {
              access: #storage_access,
              format: wgpu::TextureFormat::#format,
              view_dimension: #view_dim,
          })
        }
      }
    }
    naga::TypeInner::Sampler { comparison } => {
      let sampler_type = if comparison {
        quote!(wgpu::SamplerBindingType::Comparison)
      } else {
        quote!(wgpu::SamplerBindingType::Filtering)
      };
      quote!(wgpu::BindingType::Sampler(#sampler_type))
    }
    // TODO: Better error handling.
    _ => panic!("Failed to generate BindingType."),
  };

  quote! {
      wgpu::BindGroupLayoutEntry {
          binding: #binding_index,
          visibility: #stages,
          ty: #binding_type,
          count: None,
      }
  }
}

fn storage_access(access: naga::StorageAccess) -> TokenStream {
  let is_read = access.contains(naga::StorageAccess::LOAD);
  let is_write = access.contains(naga::StorageAccess::STORE);
  match (is_read, is_write) {
    (true, true) => quote!(wgpu::StorageTextureAccess::ReadWrite),
    (true, false) => quote!(wgpu::StorageTextureAccess::ReadOnly),
    (false, true) => quote!(wgpu::StorageTextureAccess::WriteOnly),
    _ => todo!(), // shouldn't be possible
  }
}

pub fn get_bind_group_data(
  module: &naga::Module,
) -> Result<BTreeMap<u32, GroupData>, CreateModuleError> {
  // Use a BTree to sort type and field names by group index.
  // This isn't strictly necessary but makes the generated code cleaner.
  let mut groups = BTreeMap::new();

  for global_handle in module.global_variables.iter() {
    let global = &module.global_variables[global_handle.0];
    if let Some(binding) = &global.binding {
      let group = groups.entry(binding.group).or_insert(GroupData {
        bindings: Vec::new(),
      });
      let binding_type = &module.types[module.global_variables[global_handle.0].ty];

      let group_binding = GroupBinding {
        name: global.name.clone(),
        binding_index: binding.binding,
        binding_type,
        address_space: global.space,
      };
      // Repeated bindings will probably cause a compile error.
      // We'll still check for it here just in case.
      if group
        .bindings
        .iter()
        .any(|g| g.binding_index == binding.binding)
      {
        return Err(CreateModuleError::DuplicateBinding {
          binding: binding.binding,
        });
      }
      group.bindings.push(group_binding);
    }
  }

  // wgpu expects bind groups to be consecutive starting from 0.
  if groups.keys().map(|i| *i as usize).eq(0..groups.len()) {
    Ok(groups)
  } else {
    Err(CreateModuleError::NonConsecutiveBindGroups)
  }
}

#[cfg(test)]
mod tests {
  use indoc::indoc;

  use super::*;
  use crate::assert_tokens_eq;

  #[test]
  fn bind_group_data_consecutive_bind_groups() {
    let source = indoc! {r#"
            @group(0) @binding(0) var<uniform> a: vec4<f32>;
            @group(1) @binding(0) var<uniform> b: vec4<f32>;
            @group(2) @binding(0) var<uniform> c: vec4<f32>;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert_eq!(3, get_bind_group_data(&module).unwrap().len());
  }

  #[test]
  fn bind_group_data_first_group_not_zero() {
    let source = indoc! {r#"
            @group(1) @binding(0) var<uniform> a: vec4<f32>;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert!(matches!(
      get_bind_group_data(&module),
      Err(CreateModuleError::NonConsecutiveBindGroups)
    ));
  }

  #[test]
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
      get_bind_group_data(&module),
      Err(CreateModuleError::NonConsecutiveBindGroups)
    ));
  }

  #[test]
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
    let bind_group_data = get_bind_group_data(&module).unwrap();

    let actual = bind_groups_module(
      "",
      &WgslBindgenOption::default(),
      &bind_group_data,
      wgpu::ShaderStages::COMPUTE,
    );

    assert_tokens_eq!(
      quote! {
          pub mod bind_groups {
              #[derive(Debug)]
              pub struct WgpuBindGroupLayout0<'a> {
                  pub src: wgpu::BufferBinding<'a>,
                  pub vertex_weights: wgpu::BufferBinding<'a>,
                  pub dst: wgpu::BufferBinding<'a>,
              }
              impl<'a> WgpuBindGroupLayout0<'a> {
                pub fn entries(self) -> [wgpu::BindGroupEntry<'a>; 3] {
                  [
                      wgpu::BindGroupEntry {
                          binding: 0,
                          resource: wgpu::BindingResource::Buffer(self.src),
                      },
                      wgpu::BindGroupEntry {
                          binding: 1,
                          resource: wgpu::BindingResource::Buffer(self.vertex_weights),
                      },
                      wgpu::BindGroupEntry {
                          binding: 2,
                          resource: wgpu::BindingResource::Buffer(self.dst),
                      },
                  ]
                }
              }
              #[derive(Debug)]
              pub struct WgpuBindGroup0(wgpu::BindGroup);
              impl WgpuBindGroup0 {
                pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                    label: Some("::BindGroup0::LayoutDescriptor"),
                    entries: &[
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage {
                                    read_only: false,
                                },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                };
                  pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                      device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
                  }
                  pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroupLayout0) -> Self {
                      let bind_group_layout = Self::get_bind_group_layout(&device);
                      let entries = bindings.entries();
                      let bind_group = device
                          .create_bind_group(
                              &wgpu::BindGroupDescriptor {
                                  label: Some("::BindGroup0"),
                                  layout: &bind_group_layout,
                                  entries: &entries,
                              },
                          );
                      Self(bind_group)
                  }
                  pub fn set<'a>(&'a self, render_pass: &mut wgpu::ComputePass<'a>) {
                      render_pass.set_bind_group(0, &self.0, &[]);
                  }
              }
              #[derive(Debug)]
              pub struct WgpuBindGroupLayout1<'a> {
                  pub transforms: wgpu::BufferBinding<'a>,
              }
              impl<'a> WgpuBindGroupLayout1<'a> {
                pub fn entries(self) -> [wgpu::BindGroupEntry<'a>; 1] {
                  [
                      wgpu::BindGroupEntry {
                          binding: 0,
                          resource: wgpu::BindingResource::Buffer(self.transforms),
                      },
                  ]
                }
              }
              #[derive(Debug)]
              pub struct WgpuBindGroup1(wgpu::BindGroup);
              impl WgpuBindGroup1 {
                pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                    label: Some("::BindGroup1::LayoutDescriptor"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                };

                  pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                      device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
                  }
                  pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroupLayout1) -> Self {
                      let bind_group_layout = Self::get_bind_group_layout(&device);
                      let entries = bindings.entries();
                      let bind_group = device
                          .create_bind_group(
                              &wgpu::BindGroupDescriptor {
                                  label: Some("::BindGroup1"),
                                  layout: &bind_group_layout,
                                  entries: &entries,
                              },
                          );
                      Self(bind_group)
                  }
                  pub fn set<'a>(&'a self, render_pass: &mut wgpu::ComputePass<'a>) {
                      render_pass.set_bind_group(1, &self.0, &[]);
                  }
              }
              #[derive(Debug, Copy, Clone)]
              pub struct WgpuBindGroups<'a> {
                  pub bind_group0: &'a WgpuBindGroup0,
                  pub bind_group1: &'a WgpuBindGroup1,
              }
              impl<'a> WgpuBindGroups<'a> {
                  pub fn set(&self, pass: &mut wgpu::ComputePass<'a>) {
                      self.bind_group0.set(pass);
                      self.bind_group1.set(pass);
                  }
              }
          }
          pub fn set_bind_groups<'a>(
              pass: &mut wgpu::ComputePass<'a>,
              bind_group0: &'a bind_groups::WgpuBindGroup0,
              bind_group1: &'a bind_groups::WgpuBindGroup1,
          ) {
              bind_group0.set(pass);
              bind_group1.set(pass);
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
    let bind_group_data = get_bind_group_data(&module).unwrap();

    let actual = bind_groups_module(
      "",
      &WgslBindgenOption::default(),
      &bind_group_data,
      wgpu::ShaderStages::VERTEX_FRAGMENT,
    );

    // TODO: Are storage buffers valid for vertex/fragment?
    assert_tokens_eq!(
      quote! {
          pub mod bind_groups {
              #[derive(Debug)]
              pub struct WgpuBindGroupLayout0<'a> {
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
              impl<'a> WgpuBindGroupLayout0<'a> {
                pub fn entries(self) -> [wgpu::BindGroupEntry<'a>; 11] {
                  [
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            self.color_texture,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            self.color_texture_i32,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            self.color_texture_u32,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(
                            self.color_sampler,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(
                            self.depth_texture,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(
                            self.comparison_sampler,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(
                            self.storage_tex_read,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(
                            self.storage_tex_write,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: wgpu::BindingResource::TextureView(
                            self.storage_tex_read_write,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: wgpu::BindingResource::TextureView(
                            self.color_texture_msaa,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: wgpu::BindingResource::TextureView(
                            self.depth_texture_msaa,
                        ),
                    },
                ]
                }
              }
              #[derive(Debug)]
              pub struct WgpuBindGroup0(wgpu::BindGroup);
              impl WgpuBindGroup0 {
                pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                    label: Some("::BindGroup0::LayoutDescriptor"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float {
                                    filterable: false,
                                },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                            count: None,
                        },
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 9,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float {
                                    filterable: false,
                                },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: true,
                            },
                            count: None,
                        },
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
                  pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroupLayout0) -> Self {
                      let bind_group_layout = Self::get_bind_group_layout(&device);
                      let entries = bindings.entries();
                      let bind_group = device
                          .create_bind_group(
                              &wgpu::BindGroupDescriptor {
                                  label: Some("::BindGroup0"),
                                  layout: &bind_group_layout,
                                  entries: &entries,
                              },
                          );
                      Self(bind_group)
                  }
                  pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
                      render_pass.set_bind_group(0, &self.0, &[]);
                  }
              }
              #[derive(Debug)]
              pub struct WgpuBindGroupLayout1<'a> {
                  pub transforms: wgpu::BufferBinding<'a>,
                  pub one: wgpu::BufferBinding<'a>,
              }
              impl<'a> WgpuBindGroupLayout1<'a> {
                pub fn entries(self) -> [wgpu::BindGroupEntry<'a>; 2] {
                  [
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(self.transforms),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(self.one),
                    },
                  ]
                }
              }
              #[derive(Debug)]
              pub struct WgpuBindGroup1(wgpu::BindGroup);
              impl WgpuBindGroup1 {
                pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                    label: Some("::BindGroup1::LayoutDescriptor"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                };
                  pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                      device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
                  }
                  pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroupLayout1) -> Self {
                      let bind_group_layout = Self::get_bind_group_layout(&device);
                      let entries = bindings.entries();
                      let bind_group = device
                          .create_bind_group(
                              &wgpu::BindGroupDescriptor {
                                  label: Some("::BindGroup1"),
                                  layout: &bind_group_layout,
                                  entries: &entries,
                              },
                          );
                      Self(bind_group)
                  }
                  pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
                      render_pass.set_bind_group(1, &self.0, &[]);
                  }
              }
              #[derive(Debug, Copy, Clone)]
              pub struct WgpuBindGroups<'a> {
                  pub bind_group0: &'a WgpuBindGroup0,
                  pub bind_group1: &'a WgpuBindGroup1,
              }
              impl<'a> WgpuBindGroups<'a> {
                  pub fn set(&self, pass: &mut wgpu::RenderPass<'a>) {
                      self.bind_group0.set(pass);
                      self.bind_group1.set(pass);
                  }
              }
          }
          pub fn set_bind_groups<'a>(
              pass: &mut wgpu::RenderPass<'a>,
              bind_group0: &'a bind_groups::WgpuBindGroup0,
              bind_group1: &'a bind_groups::WgpuBindGroup1,

          ) {
              bind_group0.set(pass);
              bind_group1.set(pass);
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
    let bind_group_data = get_bind_group_data(&module).unwrap();

    let actual = bind_groups_module(
      "",
      &WgslBindgenOption::default(),
      &bind_group_data,
      wgpu::ShaderStages::VERTEX,
    );

    assert_tokens_eq!(
      quote! {
          pub mod bind_groups {
              #[derive(Debug)]
              pub struct WgpuBindGroupLayout0<'a> {
                  pub transforms: wgpu::BufferBinding<'a>,
              }
              impl<'a> WgpuBindGroupLayout0<'a> {
                pub fn entries(self) -> [wgpu::BindGroupEntry<'a>; 1] {
                  [
                      wgpu::BindGroupEntry {
                          binding: 0,
                          resource: wgpu::BindingResource::Buffer(self.transforms),
                      },
                  ]
                }
              }
              #[derive(Debug)]
              pub struct WgpuBindGroup0(wgpu::BindGroup);
              impl WgpuBindGroup0 {
                pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                    label: Some("::BindGroup0::LayoutDescriptor"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                };
                  pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                      device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
                  }
                  pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroupLayout0) -> Self {
                      let bind_group_layout = Self::get_bind_group_layout(&device);
                      let entries = bindings.entries();
                      let bind_group = device
                          .create_bind_group(
                              &wgpu::BindGroupDescriptor {
                                  label: Some("::BindGroup0"),
                                  layout: &bind_group_layout,
                                  entries: &entries,
                              },
                          );
                      Self(bind_group)
                  }
                  pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
                      render_pass.set_bind_group(0, &self.0, &[]);
                  }
              }
              #[derive(Debug, Copy, Clone)]
              pub struct WgpuBindGroups<'a> {
                  pub bind_group0: &'a WgpuBindGroup0,
              }
              impl<'a> WgpuBindGroups<'a> {
                  pub fn set(&self, pass: &mut wgpu::RenderPass<'a>) {
                      self.bind_group0.set(pass);
                  }
              }
          }
          pub fn set_bind_groups<'a>(
              pass: &mut wgpu::RenderPass<'a>,
              bind_group0: &'a bind_groups::WgpuBindGroup0,
          ) {
              bind_group0.set(pass);
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
    let bind_group_data = get_bind_group_data(&module).unwrap();

    let actual = bind_groups_module(
      "",
      &WgslBindgenOption::default(),
      &bind_group_data,
      wgpu::ShaderStages::FRAGMENT,
    );

    assert_tokens_eq!(
      quote! {
          pub mod bind_groups {
              #[derive(Debug)]
              pub struct WgpuBindGroupLayout0<'a> {
                  pub transforms: wgpu::BufferBinding<'a>,
              }
              impl<'a> WgpuBindGroupLayout0<'a> {
                pub fn entries(self) -> [wgpu::BindGroupEntry<'a>; 1] {
                  [
                      wgpu::BindGroupEntry {
                          binding: 0,
                          resource: wgpu::BindingResource::Buffer(self.transforms),
                      },
                  ]
                }
              }
              #[derive(Debug)]
              pub struct WgpuBindGroup0(wgpu::BindGroup);
              impl WgpuBindGroup0 {
                pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                  label: Some("::BindGroup0::LayoutDescriptor"),
                  entries: &[
                      wgpu::BindGroupLayoutEntry {
                          binding: 0,
                          visibility: wgpu::ShaderStages::FRAGMENT,
                          ty: wgpu::BindingType::Buffer {
                              ty: wgpu::BufferBindingType::Uniform,
                              has_dynamic_offset: false,
                              min_binding_size: None,
                          },
                          count: None,
                      },
                  ],
                };

                  pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                      device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
                  }
                  pub fn from_bindings(device: &wgpu::Device, bindings: WgpuBindGroupLayout0) -> Self {
                      let bind_group_layout = Self::get_bind_group_layout(&device);
                      let entries = bindings.entries();
                      let bind_group = device
                          .create_bind_group(
                              &wgpu::BindGroupDescriptor {
                                  label: Some("::BindGroup0"),
                                  layout: &bind_group_layout,
                                  entries: &entries,
                              },
                          );
                      Self(bind_group)
                  }
                  pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
                      render_pass.set_bind_group(0, &self.0, &[]);
                  }
              }
              #[derive(Debug, Copy, Clone)]
              pub struct WgpuBindGroups<'a> {
                  pub bind_group0: &'a WgpuBindGroup0,
              }
              impl<'a> WgpuBindGroups<'a> {
                  pub fn set(&self, pass: &mut wgpu::RenderPass<'a>) {
                      self.bind_group0.set(pass);
                  }
              }
          }
          pub fn set_bind_groups<'a>(
              pass: &mut wgpu::RenderPass<'a>,
              bind_group0: &'a bind_groups::WgpuBindGroup0,
          ) {
              bind_group0.set(pass);
          }
      },
      actual
    );
  }
}
