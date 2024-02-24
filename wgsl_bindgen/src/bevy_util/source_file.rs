use smallvec::SmallVec;

use super::parse_imports;
use super::parse_imports::ImportStatement;
use crate::{
  types::{FxIndexSet, SourceFilePath},
  ImportedPath, SourceModuleName,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
  pub file_path: SourceFilePath,
  pub module_name: Option<SourceModuleName>,
  pub content: String,
  pub imports: SmallVec<[ImportStatement; 4]>,
  pub direct_dependencies: FxIndexSet<SourceFilePath>,
}

impl SourceFile {
  pub fn create(
    file_path: SourceFilePath,
    module_name: Option<SourceModuleName>,
    content: String,
  ) -> Self {
    let normalized_content = content.replace("\r\n", "\n").replace("\r", "\n");
    let mut source = Self {
      file_path,
      module_name,
      content: normalized_content,
      imports: SmallVec::default(),
      direct_dependencies: FxIndexSet::default(),
    };

    source.imports =
      parse_imports::get_import_statements::<SmallVec<_>>(&source.content.as_ref());
    source
  }

  pub fn add_direct_dependency(&mut self, dependency: SourceFilePath) {
    self.direct_dependencies.insert(dependency);
  }

  pub fn get_imported_paths(&self) -> FxIndexSet<ImportedPath> {
    self
      .imports
      .iter()
      .flat_map(|import_stmt| import_stmt.get_imported_paths())
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use indexmap::indexset;
  use pretty_assertions::assert_eq;

  use super::*;

  #[test]
  fn test_parsing_imports_from_bevy_mesh_view_bindings() {
    let module_name = Some(SourceModuleName::new("bevy_pbr::mesh_view_bindings"));
    let source_path = SourceFilePath::new("mesh_view_bindings.wgsl");
    let source = SourceFile::create(
      source_path,
      module_name,
      include_str!("../../tests/shaders/bevy_pbr_wgsl/mesh_view_bindings.wgsl").to_owned(),
    );
    let actual = source.get_imported_paths();

    assert_eq!(
      actual,
      indexset! {
        ImportedPath::new("bevy_pbr::mesh_view_types")
      }
    );
  }
}
