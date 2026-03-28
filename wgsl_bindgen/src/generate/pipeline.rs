use std::collections::BTreeMap;

use derive_more::Constructor;
use generate::quote_shader_stages;
use smol_str::ToSmolStr;

use super::bind_group::{ShaderEntryBindGroups, SingleBindGroupData};
use crate::bind_group::ShaderBindGroupRefKind;
use crate::quote_gen::RustSourceItemPath;
use crate::*;

#[derive(Constructor)]
pub struct PipelineLayoutDataEntriesBuilder<'a> {
  generator: &'a PipelineLayoutGenerator,
  bind_group_data: &'a BTreeMap<u32, SingleBindGroupData<'a>>,
}

impl<'a> PipelineLayoutDataEntriesBuilder<'a> {
  fn bind_group_layout_entries_fn(&self) -> TokenStream {
    let entry_type = self.generator.bind_group_layout_type.clone();
    let len = Index::from(self.bind_group_data.len());

    quote! {
      pub fn bind_group_layout_entries(entries: [#entry_type; #len]) -> [#entry_type; #len] {
        entries
      }
    }
  }

  fn build(&self) -> TokenStream {
    let name = format_ident!("{}", self.generator.layout_name);
    let bind_group_layout_entries_fn = self.bind_group_layout_entries_fn();

    quote! {
      #[derive(Debug)]
      pub struct #name;

      impl #name {
        #bind_group_layout_entries_fn
      }
    }
  }
}

pub fn create_pipeline_layout_fn(
  entry_name: &str,
  naga_module: &naga::Module,
  shader_entry_bind_groups: &ShaderEntryBindGroups,
  options: &WgslBindgenOption,
) -> TokenStream {
  let bind_group_layouts: Vec<_> = shader_entry_bind_groups
    .bind_group_ref
    .iter()
    .map(|(&group_no, group_ref)| {
      let group_name = options
        .wgpu_binding_generator
        .bind_group_layout
        .bind_group_name_ident(group_no);

      // if all entries have a common module, reference that module instead
      let group_name = match group_ref.kind {
        ShaderBindGroupRefKind::Common => {
          let containing_module = group_ref.data.first_module();
          let path = RustSourceItemPath::new(containing_module, group_name.to_smolstr());
          quote!(#path)
        }
        ShaderBindGroupRefKind::Entrypoint => quote!(#group_name),
      };

      quote!(#group_name::get_bind_group_layout(device))
    })
    .collect();

  let wgpu_pipeline_gen = &options.wgpu_binding_generator.pipeline_layout;
  let wgpu_pipeline_entries_struct = PipelineLayoutDataEntriesBuilder::new(
    wgpu_pipeline_gen,
    &shader_entry_bind_groups.original_bind_group,
  )
  .build();

  let additional_pipeline_entries_struct =
    if let Some(a) = options.extra_binding_generator.as_ref() {
      PipelineLayoutDataEntriesBuilder::new(
        &a.pipeline_layout,
        &shader_entry_bind_groups.original_bind_group,
      )
      .build()
    } else {
      quote!()
    };

  let immediate_size = naga_module
    .global_variables
    .iter()
    .find_map(|g| {
      if g.1.space == naga::AddressSpace::Immediate {
        Some(naga_module.types[g.1.ty].inner.size(naga_module.to_ctx()))
      } else {
        None
      }
    })
    .unwrap_or(0);

  let pipeline_layout_name = format!("{entry_name}::PipelineLayout");

  quote! {
    #additional_pipeline_entries_struct
    #wgpu_pipeline_entries_struct
      pub fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
          device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
              label: Some(#pipeline_layout_name),
              bind_group_layouts: &[
                  #(Some(&#bind_group_layouts)),*
              ],
              immediate_size: #immediate_size,
          })
      }
  }
}
