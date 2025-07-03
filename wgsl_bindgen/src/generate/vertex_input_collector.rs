//! Global vertex input struct collector for deduplication across multiple modules.

use naga::FastIndexMap;
use smol_str::SmolStr;
use std::collections::{BTreeMap, HashSet};

use crate::quote_gen::RustSourceItem;
use crate::wgsl::{get_all_vertex_input_structs, VertexInput};

/// Holds vertex input structs for a single shader entry/module
pub struct RawShaderVertexInputs<'a> {
  pub containing_module: SmolStr,
  pub vertex_inputs: Vec<VertexInput<'a>>,
}

/// Global collector for vertex input structs from all shader modules
pub struct RawShadersVertexInputs<'a> {
  shader_vertex_inputs: FastIndexMap<SmolStr, RawShaderVertexInputs<'a>>,
}

impl<'a> RawShadersVertexInputs<'a> {
  pub fn new() -> Self {
    Self {
      shader_vertex_inputs: FastIndexMap::default(),
    }
  }

  /// Add vertex input structs from a shader module
  pub fn add(&mut self, shader_vertex_inputs: RawShaderVertexInputs<'a>) {
    self
      .shader_vertex_inputs
      .insert(shader_vertex_inputs.containing_module.clone(), shader_vertex_inputs);
  }

  /// Create a global vertex input collector from a shader module
  pub fn from_module(
    mod_name: &str,
    naga_module: &'a naga::Module,
  ) -> RawShaderVertexInputs<'a> {
    let vertex_inputs = get_all_vertex_input_structs(mod_name, naga_module);
    RawShaderVertexInputs {
      containing_module: SmolStr::new(mod_name),
      vertex_inputs,
    }
  }

  /// Generate deduplicated vertex input struct implementations
  pub fn generate_vertex_input_impls(self) -> Vec<RustSourceItem> {
    // Global deduplication: collect unique vertex input structs by their actual content/path
    let mut unique_vertex_inputs = BTreeMap::new();

    // Collect all vertex input structs and deduplicate by their actual item path (not module path)
    for (_, shader) in &self.shader_vertex_inputs {
      for vertex_input in &shader.vertex_inputs {
        // Use the vertex input's actual item path (e.g., "types::VertexInput") as the deduplication key
        // This is the real path to the struct, not the module that imports it
        let item_path_key = vertex_input.item_path.get_fully_qualified_name();

        // Only add if we haven't seen this exact item path before
        unique_vertex_inputs
          .entry(item_path_key)
          .or_insert(vertex_input);
      }
    }

    // Generate implementations for all unique vertex input structs
    unique_vertex_inputs
      .into_values()
      .map(super::entry::generate_vertex_input_impl)
      .collect()
  }
}
