use std::collections::BTreeMap;

use derive_more::Constructor;

use super::bind_group::GroupData;
use crate::*;

#[derive(Constructor)]
pub struct PipelineLayoutDataEntriesBuilder<'a> {
  generator: &'a PipelineLayoutGenerator,
  bind_group_data: &'a BTreeMap<u32, GroupData<'a>>,
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
  options: &WgslBindgenOption,
  bind_group_data: &BTreeMap<u32, GroupData>,
) -> TokenStream {
  let bind_group_layouts: Vec<_> = bind_group_data
    .keys()
    .map(|group_no| {
      let group = options
        .wgpu_binding_generator
        .bind_group_layout
        .bind_group_name_ident(*group_no);
      quote!(#group::get_bind_group_layout(device))
    })
    .collect();

  let wgpu_pipeline_gen = &options.wgpu_binding_generator.pipeline_layout;
  let wgpu_pipeline_entries_struct =
    PipelineLayoutDataEntriesBuilder::new(wgpu_pipeline_gen, bind_group_data).build();

  let additional_pipeline_entries_struct =
    if let Some(a) = options.extra_binding_generator.as_ref() {
      PipelineLayoutDataEntriesBuilder::new(&a.pipeline_layout, bind_group_data).build()
    } else {
      quote!()
    };

  let pipeline_layout_name = format!("{}::PipelineLayout", entry_name);

  quote! {
    #additional_pipeline_entries_struct
    #wgpu_pipeline_entries_struct
      pub fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
          device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
              label: Some(#pipeline_layout_name),
              bind_group_layouts: &[
                  #(&#bind_group_layouts),*
              ],
              push_constant_ranges: &[],
          })
      }
  }
}
