use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use naga::FastIndexMap;
use smol_str::SmolStr;

use super::single_bind_group::SingleBindGroupEntry;
use crate::bind_group::{
  CommonShaderBindGroups, ReusableShaderBindGroups, ShaderBindGroupRef,
  ShaderBindGroupRefKind, ShaderEntryBindGroups, SingleBindGroupData,
};
use crate::{CreateModuleError, WgslBindgenOption};

pub struct RawShaderEntryBindGroups<'a> {
  pub containing_module: SmolStr,
  pub shader_stages: wgpu::ShaderStages,
  pub bind_group_data: BTreeMap<u32, SingleBindGroupData<'a>>,
}

pub struct RawShadersBindGroups<'a> {
  entrypoint_bindgroups: FastIndexMap<SmolStr, RawShaderEntryBindGroups<'a>>,
  options: &'a WgslBindgenOption,
}

impl<'a> RawShadersBindGroups<'a> {
  pub fn new(options: &'a WgslBindgenOption) -> Self {
    Self {
      entrypoint_bindgroups: FastIndexMap::default(),
      options,
    }
  }

  pub fn add(&mut self, mut shader: RawShaderEntryBindGroups<'a>) {
    // patch up the module name for bind group entrys
    for entry in shader
      .bind_group_data
      .values_mut()
      .flat_map(|v| v.bindings.iter_mut())
    {
      let target_patch_module = self
        .options
        .override_bind_group_entry_module_path
        .iter()
        .find_map(|o| {
          let matched = o
            .bind_group_entry_regex
            .is_match(&entry.item_path.get_fully_qualified_name());
          matched.then_some(SmolStr::new(o.target_path.clone()))
        });

      if let Some(target_patch_module) = target_patch_module {
        entry.item_path.module = target_patch_module;
      }
    }

    self
      .entrypoint_bindgroups
      .insert(shader.containing_module.clone(), shader);
  }

  pub fn create_reusable_shader_bind_groups(self) -> ReusableShaderBindGroups<'a> {
    fn merge_bind_groups<'a>(
      existing_group: &SingleBindGroupData<'a>,
      new_group: &SingleBindGroupData<'a>,
    ) -> SingleBindGroupData<'a> {
      let mut merged_bindings = existing_group.bindings.clone();
      for binding in new_group.bindings.iter() {
        merged_bindings.push(binding.clone());
      }
      merged_bindings.sort_by(|a, b| a.binding_index.cmp(&b.binding_index));
      merged_bindings.dedup_by(|a, b| {
        a.binding_index == b.binding_index
          && a.item_path == b.item_path
          && a.name == b.name
      });
      SingleBindGroupData {
        bindings: merged_bindings,
        naga_module: existing_group.naga_module,
      }
    }

    // Create a common binding group for all shaders.
    let mut common_bind_groups = BTreeMap::new();
    for shader in self.entrypoint_bindgroups.values() {
      for (&group_no, group) in &shader.bind_group_data {
        // Check if all entry have the same module.
        let first_module = group.first_module();
        let all_same_module = group.are_all_same_module();

        // if all the bindings are in the same module, and of this shader, skip it.
        if all_same_module && first_module == shader.containing_module {
          continue;
        }

        match common_bind_groups.entry(group_no) {
          Entry::Vacant(vacant_entry) => {
            vacant_entry.insert((shader.shader_stages, group.clone()));
          }
          Entry::Occupied(mut occupied_entry) => {
            let merged_group = merge_bind_groups(&occupied_entry.get().1, group);
            let merged_stages = occupied_entry.get().0 | shader.shader_stages;
            occupied_entry.insert((merged_stages, merged_group));
          }
        };
      }
    }

    // Remove all the bind groups that are not reusable.
    common_bind_groups.retain(|&_group_no, (_, group)| group.are_all_same_module());

    // Update common bind groups with combined shader stages for correct visibility
    let mut updated_common_bind_groups = BTreeMap::new();
    for (&group_no, (merged_stages, group)) in &common_bind_groups {
      let common_module = group.first_module();
      let updated_group = group.with_updated_shader_stages(
        &common_module,
        self.options,
        *merged_stages,
        group_no,
      );
      updated_common_bind_groups.insert(group_no, (merged_stages, updated_group));
    }

    // Create the reusable shader bind groups
    let mut reusable_shader_bind_groups = ReusableShaderBindGroups::new();
    for (&group_no, (_, group)) in &updated_common_bind_groups {
      let common_module = group.first_module();

      reusable_shader_bind_groups
        .common_bind_groups
        .entry(common_module.clone())
        .or_insert_with(|| CommonShaderBindGroups {
          containing_module: common_module.clone(),
          bind_group_data: BTreeMap::new(),
        })
        .bind_group_data
        .insert(group_no, group.clone());
    }

    // Add the shader entries to the reusable shader bind groups
    for (_, shader) in &self.entrypoint_bindgroups {
      // force create an entry even though bind groups might be empty.
      // this is required for other parts of the pipeline to work
      let shader_entry_bindgroups = reusable_shader_bind_groups
        .entrypoint_bindgroups
        .entry(shader.containing_module.clone())
        .or_insert_with(|| ShaderEntryBindGroups {
          containing_module: shader.containing_module.clone(),
          shader_stages: shader.shader_stages,
          bind_group_ref: BTreeMap::new(),
          original_bind_group: shader.bind_group_data.clone(),
        });

      for (group_no, group) in &shader.bind_group_data {
        let common_bindgroup = updated_common_bind_groups
          .get(group_no)
          .map(|(_, group)| group);
        let is_common = Some(group.first_module())
          == common_bindgroup.map(|group| group.first_module());
        let reusable_bindgroup = is_common.then_some(common_bindgroup).flatten();

        if let Some(reusable_bindgroup) = reusable_bindgroup {
          shader_entry_bindgroups.bind_group_ref.insert(
            *group_no,
            ShaderBindGroupRef {
              kind: ShaderBindGroupRefKind::Common,
              data: reusable_bindgroup.clone(),
            },
          );
        } else {
          shader_entry_bindgroups.bind_group_ref.insert(
            *group_no,
            ShaderBindGroupRef {
              kind: ShaderBindGroupRefKind::Entrypoint,
              data: group.clone(),
            },
          );
        }
      }
    }

    reusable_shader_bind_groups
  }
}

/// Context for bind group data generation
pub struct BindGroupGenerationContext<'a> {
  pub module: &'a naga::Module,
  pub shader_stages: wgpu::ShaderStages,
  pub options: &'a WgslBindgenOption,
  pub module_name: &'a str,
}

impl<'a> BindGroupGenerationContext<'a> {
  pub fn new(
    module: &'a naga::Module,
    shader_stages: wgpu::ShaderStages,
    options: &'a WgslBindgenOption,
    module_name: &'a str,
  ) -> Self {
    Self {
      module,
      shader_stages,
      options,
      module_name,
    }
  }
}

pub fn get_bind_group_data_for_entry<'a>(
  module: &'a naga::Module,
  shader_stages: wgpu::ShaderStages,
  options: &'a WgslBindgenOption,
  module_name: &'a str,
) -> Result<RawShaderEntryBindGroups<'a>, CreateModuleError> {
  let context =
    BindGroupGenerationContext::new(module, shader_stages, options, module_name);
  get_bind_group_data_for_entry_with_context(&context)
}

/// Generate bind group data using a context struct for better organization
fn get_bind_group_data_for_entry_with_context<'a>(
  context: &BindGroupGenerationContext<'a>,
) -> Result<RawShaderEntryBindGroups<'a>, CreateModuleError> {
  // Use a BTree to sort type and field names by group index.
  // This isn't strictly necessary but makes the generated code cleaner.
  let mut bind_group_data = BTreeMap::new();

  for global_handle in context.module.global_variables.iter() {
    let global = &context.module.global_variables[global_handle.0];
    if let Some(binding) = &global.binding {
      let group = bind_group_data
        .entry(binding.group)
        .or_insert(SingleBindGroupData {
          bindings: Vec::new(),
          naga_module: context.module,
        });
      let binding_type =
        &context.module.types[context.module.global_variables[global_handle.0].ty];

      let group_binding = SingleBindGroupEntry::new(
        global.name.clone(),
        context.module_name,
        context.options,
        context.module,
        context.shader_stages,
        binding.binding,
        binding_type,
        global.space,
      );

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
  if bind_group_data
    .keys()
    .map(|i| *i as usize)
    .eq(0..bind_group_data.len())
  {
    Ok(RawShaderEntryBindGroups {
      containing_module: context.module_name.into(),
      shader_stages: context.shader_stages,
      bind_group_data: bind_group_data.clone(),
    })
  } else {
    Err(CreateModuleError::NonConsecutiveBindGroups)
  }
}
