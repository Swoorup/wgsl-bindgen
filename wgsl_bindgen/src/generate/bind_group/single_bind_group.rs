use derive_more::Constructor;
use smol_str::ToSmolStr;

use self::quote_gen::RustSourceItemPath;
use super::*;

#[derive(Constructor)]
struct BindGroupEntriesStructBuilder<'a> {
  containing_module: &'a str,
  group_no: u32,
  data: &'a SingleBindGroupData<'a>,
  generator: &'a BindGroupLayoutGenerator,
}

impl<'a> BindGroupEntriesStructBuilder<'a> {
  /// Generates a binding entry from a parameter variable and a group binding.
  fn create_entry_from_parameter(
    &self,
    binding_var_name: &Ident,
    binding: &SingleBindGroupEntry,
  ) -> TokenStream {
    let entry_cons = self.generator.entry_constructor;
    let binding_index = binding.binding_index as usize;
    let demangled_name = RustSourceItemPath::from_mangled(
      binding.name.as_ref().unwrap(),
      self.containing_module,
    );
    let binding_name = Ident::new(&demangled_name.name, Span::call_site());
    let binding_var = quote!(#binding_var_name.#binding_name);

    match binding.binding_type.inner {
      naga::TypeInner::Scalar(_)
      | naga::TypeInner::Struct { .. }
      | naga::TypeInner::Array { .. } => {
        entry_cons(binding_index, binding_var, BindResourceType::Buffer)
      }
      naga::TypeInner::Image { .. } => {
        entry_cons(binding_index, binding_var, BindResourceType::Texture)
      }
      naga::TypeInner::Sampler { .. } => {
        entry_cons(binding_index, binding_var, BindResourceType::Sampler)
      }
      // TODO: Better error handling.
      _ => panic!("Failed to generate BindingType."),
    }
  }

  /// Assigns entries for the bind group from the provided parameters.
  fn assign_entries_from_parameters(&self, param_var_name: Ident) -> Vec<TokenStream> {
    self
      .data
      .bindings
      .iter()
      .map(|binding| {
        let demangled_name = RustSourceItemPath::from_mangled(
          binding.name.as_ref().unwrap(),
          self.containing_module,
        );
        let binding_name = Ident::new(&demangled_name.name, Span::call_site());
        let create_entry = self.create_entry_from_parameter(&param_var_name, binding);

        quote! {
          #binding_name: #create_entry
        }
      })
      .collect()
  }

  /// Generates a tuple of parameter field and entry field for a binding.
  fn binding_field_tuple(
    &self,
    binding: &SingleBindGroupEntry,
  ) -> (TokenStream, TokenStream) {
    let rust_item_path = RustSourceItemPath::from_mangled(
      binding.name.as_ref().unwrap(),
      self.containing_module,
    );
    let field_name = format_ident!("{}", &rust_item_path.name.as_str());

    // TODO: Support more types.
    let resource_type = match binding.binding_type.inner {
      naga::TypeInner::Struct { .. } => BindResourceType::Buffer,
      naga::TypeInner::Image { .. } => BindResourceType::Texture,
      naga::TypeInner::Sampler { .. } => BindResourceType::Sampler,
      naga::TypeInner::Array { .. } => BindResourceType::Buffer,
      naga::TypeInner::Scalar(_) => BindResourceType::Buffer,
      _ => panic!("Unsupported type for binding fields."),
    };

    let param_field_type = self.generator.binding_type_map[&resource_type].clone();
    let field_type = self.generator.entry_struct_type.clone();

    let param_field = quote!(pub #field_name: #param_field_type);
    let entry_field = quote!(pub #field_name: #field_type);

    (param_field, entry_field)
  }

  fn all_entries(&self, binding_var_name: Ident) -> Vec<TokenStream> {
    self
      .data
      .bindings
      .iter()
      .map(|binding| {
        let demangled_name = RustSourceItemPath::from_mangled(
          binding.name.as_ref().unwrap(),
          self.containing_module,
        );
        let binding_name = Ident::new(&demangled_name.name, Span::call_site());
        quote! (#binding_var_name.#binding_name)
      })
      .collect()
  }

  pub(super) fn build(&self) -> TokenStream {
    let (entries_param_fields, entries_fields): (Vec<_>, Vec<_>) = self
      .data
      .bindings
      .iter()
      .map(|binding| self.binding_field_tuple(binding))
      .collect();

    let entry_collection_name = self
      .generator
      .bind_group_entries_struct_name_ident(self.group_no);
    let entry_collection_param_name = format_ident!(
      "{}Params",
      self
        .generator
        .bind_group_entries_struct_name_ident(self.group_no)
    );
    let entry_struct_type = self.generator.entry_struct_type.clone();

    let lifetime = if self.generator.uses_lifetime {
      quote!(<'a>)
    } else {
      quote!()
    };

    let entries_from_params =
      self.assign_entries_from_parameters(format_ident!("params"));
    let entries_length = Index::from(entries_from_params.len() as usize);
    let all_entries = self.all_entries(format_ident!("self"));

    quote! {
        #[derive(Debug)]
        pub struct #entry_collection_param_name #lifetime {
            #(#entries_param_fields),*
        }

        #[derive(Clone, Debug)]
        pub struct #entry_collection_name #lifetime {
            #(#entries_fields),*
        }

        impl #lifetime #entry_collection_name #lifetime {
          pub fn new(params: #entry_collection_param_name #lifetime) -> Self {
            Self {
              #(#entries_from_params),*
            }
          }

          pub fn as_array(self) -> [#entry_struct_type; #entries_length] {
            [ #(#all_entries),* ]
          }

          pub fn collect<B: FromIterator<#entry_struct_type>>(self) -> B {
            self.as_array().into_iter().collect()
          }
        }
    }
  }
}

#[derive(Constructor)]
struct BindGroupStructBuilder<'a> {
  sanitized_entry_name: String,
  group_no: u32,
  data: &'a SingleBindGroupData<'a>,
  options: &'a WgslBindgenOption,
}

impl<'a> BindGroupStructBuilder<'a> {
  fn bind_group_layout_descriptor(&self) -> TokenStream {
    let entries: Vec<_> = self
      .data
      .bindings
      .iter()
      .map(|binding| &binding.layout_entry_token_stream)
      .collect();

    let bind_group_label = format!(
      "{}::BindGroup{}::LayoutDescriptor",
      self.sanitized_entry_name, self.group_no
    );

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
    self
      .options
      .wgpu_binding_generator
      .bind_group_layout
      .bind_group_name_ident(self.group_no)
  }

  fn bind_group_struct_impl(&self) -> TokenStream {
    // TODO: Support compute shader with vertex/fragment in the same module?
    let bind_group_name = self.struct_name();
    let bind_group_entries_struct_name = self
      .options
      .wgpu_binding_generator
      .bind_group_layout
      .bind_group_entries_struct_name_ident(self.group_no);

    let bind_group_layout_descriptor = self.bind_group_layout_descriptor();

    let group_no = Index::from(self.group_no as usize);
    let bind_group_label =
      format!("{}::BindGroup{}", self.sanitized_entry_name, self.group_no);

    quote! {
        impl #bind_group_name {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = #bind_group_layout_descriptor;

            pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
            }

            pub fn from_bindings(device: &wgpu::Device, bindings: #bind_group_entries_struct_name) -> Self {
                let bind_group_layout = Self::get_bind_group_layout(&device);
                let entries = bindings.as_array();
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(#bind_group_label),
                    layout: &bind_group_layout,
                    entries: &entries,
                });
                Self(bind_group)
            }

            pub fn set(&self, pass: &mut impl SetBindGroup) {
                pass.set_bind_group(#group_no, &self.0, &[]);
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

#[derive(Constructor)]
pub struct SingleBindGroupBuilder<'a> {
  pub containing_module: &'a str,
  pub group_no: u32,
  pub group_data: &'a SingleBindGroupData<'a>,
  pub options: &'a WgslBindgenOption,
}

impl<'a> SingleBindGroupBuilder<'a> {
  pub(super) fn build(&self) -> RustSourceItem {
    let wgpu_generator = &self.options.wgpu_binding_generator;

    let bind_group_entries_struct = BindGroupEntriesStructBuilder::new(
      self.containing_module,
      self.group_no,
      self.group_data,
      &wgpu_generator.bind_group_layout,
    )
    .build();

    let additional_layout =
      if let Some(additional_generator) = &self.options.extra_binding_generator {
        BindGroupEntriesStructBuilder::new(
          self.containing_module,
          self.group_no,
          self.group_data,
          &additional_generator.bind_group_layout,
        )
        .build()
      } else {
        quote!()
      };

    let bindgroup_struct_builder = BindGroupStructBuilder::new(
      sanitize_and_pascal_case(self.containing_module),
      self.group_no,
      self.group_data,
      self.options,
    );

    let source_item_path = RustSourceItemPath::new(
      self.containing_module.into(),
      bindgroup_struct_builder.struct_name().to_smolstr(),
    );
    let bindgroup = bindgroup_struct_builder.build();

    let rust_source_item = RustSourceItem::new(
      RustSourceItemCategory::TypeDefs | RustSourceItemCategory::TypeImpls,
      source_item_path,
      quote! {
        #additional_layout
        #bind_group_entries_struct
        #bindgroup
      },
    );

    rust_source_item
  }
}

fn bind_group_layout_entry(
  invoking_entry_module: &str,
  naga_module: &naga::Module,
  options: &WgslBindgenOption,
  shader_stages: wgpu::ShaderStages,
  binding_index: u32,
  binding_type: &naga::Type,
  name: Option<String>,
  address_space: naga::AddressSpace,
) -> TokenStream {
  // TODO: Assume storage is only used for compute?
  // TODO: Support just vertex or fragment?
  // TODO: Visible from all stages?
  let stages = quote_shader_stages(shader_stages);

  // TODO: Support more types.
  let binding_type = match &binding_type.inner {
    naga::TypeInner::Scalar(_)
    | naga::TypeInner::Struct { .. }
    | naga::TypeInner::Array { .. } => {
      let buffer_binding_type = buffer_binding_type(address_space);

      let rust_type =
        rust_type(Some(invoking_entry_module), naga_module, &binding_type, options);

      let min_binding_size = rust_type.quote_min_binding_size();

      quote!(wgpu::BindingType::Buffer {
          ty: #buffer_binding_type,
          has_dynamic_offset: false,
          min_binding_size: #min_binding_size,
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
              quote!(wgpu::TextureSampleType::Float { filterable: true })
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
          let storage_access = storage_access(*access);

          quote!(wgpu::BindingType::StorageTexture {
              access: #storage_access,
              format: wgpu::TextureFormat::#format,
              view_dimension: #view_dim,
          })
        }
      }
    }
    naga::TypeInner::Sampler { comparison } => {
      let sampler_type = if *comparison {
        quote!(wgpu::SamplerBindingType::Comparison)
      } else {
        quote!(wgpu::SamplerBindingType::Filtering)
      };
      quote!(wgpu::BindingType::Sampler(#sampler_type))
    }
    // TODO: Better error handling.
    unknown => panic!("Failed to generate BindingType for {:?}.", &unknown),
  };

  let doc = format!(
    " @binding({}): \"{}\"",
    binding_index,
    demangle_and_fully_qualify_str(name.as_ref().unwrap(), None),
  );

  let binding_index = Index::from(binding_index as usize);

  quote! {
      #[doc = #doc]
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

#[derive(Clone)]
pub struct SingleBindGroupData<'a> {
  pub bindings: Vec<SingleBindGroupEntry<'a>>,
}

impl SingleBindGroupData<'_> {
  pub fn first_module(&self) -> SmolStr {
    self.bindings.first().unwrap().item_path.module.clone()
  }

  pub fn are_all_same_module(&self) -> bool {
    let first_module = self.first_module();
    self
      .bindings
      .iter()
      .all(|b| b.item_path.module == first_module)
  }
}

#[derive(Clone)]
pub struct SingleBindGroupEntry<'a> {
  pub name: Option<String>,
  pub item_path: RustSourceItemPath,
  pub binding_index: u32,
  pub binding_type: &'a naga::Type,
  pub layout_entry_token_stream: TokenStream,
}

impl<'a> SingleBindGroupEntry<'a> {
  pub fn new(
    name: Option<String>,
    invoking_entry_module: &'a str,
    options: &WgslBindgenOption,
    naga_module: &naga::Module,
    shader_stages: wgpu::ShaderStages,
    binding_index: u32,
    binding_type: &'a naga::Type,
    address_space: naga::AddressSpace,
  ) -> Self {
    let item_path =
      RustSourceItemPath::from_mangled(name.as_ref().unwrap(), invoking_entry_module);

    let layout_entry_token_stream = bind_group_layout_entry(
      invoking_entry_module,
      naga_module,
      options,
      shader_stages,
      binding_index,
      binding_type,
      name.clone(),
      address_space,
    );

    Self {
      name,
      item_path,
      binding_index,
      binding_type,
      layout_entry_token_stream,
    }
  }
}
