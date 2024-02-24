use std::path::{Path, PathBuf};

use smallvec::SmallVec;

use super::escape_os_path;
use crate::{FxIndexSet, ImportedPath, SourceFileDir, SourceFilePath, SourceModuleName};

#[derive(Debug, Clone)]
pub(crate) struct ModulePathResolver<'a> {
  module_prefix: Option<&'a str>,
  entry_dir: &'a SourceFileDir,
  imported_path: &'a ImportedPath,
  current_source_path: &'a SourceFilePath,
}

impl<'a> ModulePathResolver<'a> {
  pub fn new(
    module_prefix: Option<&'a str>,
    entry_dir: &'a SourceFileDir,
    imported_path: &'a ImportedPath,
    current_source_path: &'a SourceFilePath,
  ) -> Self {
    Self {
      module_prefix,
      entry_dir,
      imported_path,
      current_source_path,
    }
  }

  /// Generates possible import paths for a given import path fragment.
  pub fn generate_best_possible_paths(
    &self,
  ) -> FxIndexSet<(SourceModuleName, SourceFilePath)> {
    let import_parts: SmallVec<[&str; 10]> = self
      .imported_path
      .split("::")
      .enumerate()
      .skip_while(|(i, part)| {
        *i == 0 && self.module_prefix == Some(part) // skip the first part
      })
      .map(|(_, part)| part)
      .filter(|part| !part.is_empty())
      .collect();

    if import_parts.is_empty() {
      panic!("import module is empty")
    }

    let create_path = |root_dir: &Path, path_fragments: &[&str]| {
      let mut path = PathBuf::from(root_dir);
      let mut module_name_builder = Vec::new();

      for fragment in path_fragments {
        // Allow to use paths directly
        let normalized_fragment = escape_os_path(fragment);

        // avoid duplicates repeated patterns
        if !path.ends_with(&normalized_fragment) {
          path.push(&normalized_fragment);
          module_name_builder.push(*fragment);
        }
      }

      if path.extension().is_none() {
        path.set_extension("wgsl");
      }

      if module_name_builder.is_empty() {
        None
      } else {
        let module_name = self
          .module_prefix
          .as_slice()
          .into_iter()
          .cloned()
          .chain(module_name_builder.into_iter())
          .collect::<Vec<_>>()
          .join("::");

        let module_name = SourceModuleName::new(module_name);
        let source_path = SourceFilePath::new(path);
        Some((module_name, source_path))
      }
    };

    let current_source_dir = self.current_source_path.parent().unwrap_or(Path::new(""));

    let paths = (0..import_parts.len())
      .filter_map(|i| create_path(&current_source_dir, &import_parts[0..=i]))
      .filter(|(_, path)| path.as_ref() != self.current_source_path.as_path())
      .rev();

    let obvious_paths = (0..import_parts.len())
      .filter_map(|i| create_path(&self.entry_dir, &import_parts[0..=i]))
      .filter(|(_, path)| path.as_ref() != self.current_source_path.as_path())
      .rev();

    obvious_paths.chain(paths).collect()
  }
}

#[cfg(test)]
mod tests {
  use indexmap::indexset;
  use pretty_assertions::assert_eq;

  use crate::bevy_util::ModulePathResolver;
  use crate::{ImportedPath, SourceFileDir, SourceFilePath, SourceModuleName};

  #[test]
  fn should_generate_single_import_path() {
    let module_prefix = None;
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let imported_path = ImportedPath::new("Fragment");

    let result = ModulePathResolver::new(
      module_prefix,
      &source_path.dir(),
      &imported_path,
      &source_path,
    )
    .generate_best_possible_paths();

    let expected = indexset![(
      SourceModuleName::new("Fragment"),
      SourceFilePath::new("mydir/Fragment.wgsl")
    )];

    assert_eq!(result, expected);
  }

  #[test]
  fn should_generate_single_import_path_when_module_prefix_match() {
    let module_prefix = Some("mymod");
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let imported_path = ImportedPath::new("mymod::Fragment");

    let result = ModulePathResolver::new(
      module_prefix,
      &source_path.dir(),
      &imported_path,
      &source_path,
    )
    .generate_best_possible_paths();

    let expected = indexset![(
      SourceModuleName::new("mymod::Fragment"),
      SourceFilePath::new("mydir/Fragment.wgsl")
    )];

    assert_eq!(result, expected);
  }

  // Should generate import paths with correct extensions
  #[test]
  fn should_generate_import_paths_with_correct_extensions() {
    let module_prefix = Some("prefix");
    let source_path = SourceFilePath::new("mydir/source");
    let imported_path = ImportedPath::new("Module::Submodule::Fragment");

    let actual = ModulePathResolver::new(
      module_prefix,
      &source_path.dir(),
      &imported_path,
      &source_path,
    )
    .generate_best_possible_paths();

    let expected = indexset![
      (
        SourceModuleName::new("prefix::Module::Submodule::Fragment"),
        SourceFilePath::new("mydir/Module/Submodule/Fragment.wgsl")
      ),
      (
        SourceModuleName::new("prefix::Module::Submodule"),
        SourceFilePath::new("mydir/Module/Submodule.wgsl")
      ),
      (
        SourceModuleName::new("prefix::Module"),
        SourceFilePath::new("mydir/Module.wgsl")
      ),
    ];
    assert_eq!(actual, expected);
  }

  #[test]
  #[should_panic]
  fn should_panic_when_import_module_is_empty() {
    let module_prefix = None;
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let imported_path = ImportedPath::new("");

    let result = ModulePathResolver::new(
      module_prefix,
      &source_path.dir(),
      &imported_path,
      &source_path,
    )
    .generate_best_possible_paths();

    let expected = indexset![];

    assert_eq!(result, expected);
  }

  // Should return an empty SmallVec when import_module has only the module prefix
  #[test]
  #[should_panic]
  fn should_return_empty_smallvec_when_import_module_has_only_module_prefix() {
    let module_prefix = Some("prefix");
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let imported_path = ImportedPath::new("prefix");

    let result = ModulePathResolver::new(
      module_prefix,
      &source_path.dir(),
      &imported_path,
      &source_path,
    )
    .generate_best_possible_paths();

    let expected = indexset![];

    assert_eq!(result, expected);
  }

  #[test]
  fn should_return_smallvec_when_import_module() {
    let module_prefix = Some("prefix");
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let imported_path = ImportedPath::new("Fragment");

    let result = ModulePathResolver::new(
      module_prefix,
      &source_path.dir(),
      &imported_path,
      &source_path,
    )
    .generate_best_possible_paths();

    let expected = indexset![(
      SourceModuleName::new("prefix::Fragment"),
      SourceFilePath::new("mydir/Fragment.wgsl")
    )];

    assert_eq!(result, expected);
  }

  #[test]
  fn should_return_valid_pbr_paths_from_repeated_part() {
    let module_prefix = Some("bevy_pbr");
    let source_path = SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/functions.wgsl");
    let imported_path = ImportedPath::new("bevy_pbr::pbr::types");

    let result = ModulePathResolver::new(
      module_prefix,
      &source_path.dir(),
      &imported_path,
      &source_path,
    )
    .generate_best_possible_paths();

    let expected = indexset![(
      SourceModuleName::new("bevy_pbr::types"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/types.wgsl")
    )];

    assert_eq!(result, expected);
  }

  #[test]
  fn should_return_valid_pbr_paths_back_to_current_dir() {
    let module_prefix = Some("bevy_pbr");
    let entry_dir = SourceFileDir::new("tests/bevy_pbr_wgsl");
    let source_path = SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/functions.wgsl");
    let imported_path = ImportedPath::new("bevy_pbr::mesh_types");

    let result =
      ModulePathResolver::new(module_prefix, &entry_dir, &imported_path, &source_path)
        .generate_best_possible_paths();

    let expected = indexset![
      (
        SourceModuleName::new("bevy_pbr::mesh_types"),
        SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_types.wgsl")
      ),
      (
        SourceModuleName::new("bevy_pbr::mesh_types"),
        SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/mesh_types.wgsl")
      )
    ];

    assert_eq!(result, expected);
  }
}
