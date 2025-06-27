use std::path::{Path, PathBuf};

use derive_more::Constructor;
use smallvec::SmallVec;

use super::escape_os_path;
use crate::{
  AdditionalScanDirectory, FxIndexSet, ImportPathPart, SourceFilePath, SourceModuleName,
};

#[derive(Debug, Constructor, Clone, Default)]
pub(crate) struct ModulePathResolver {
  workspace_root: PathBuf,
  entry_module_prefix: Option<String>,
  additional_scan_dirs: Vec<AdditionalScanDirectory>,
}

impl ModulePathResolver {
  fn create_path(
    module_prefix: &Option<String>,
    root_dir: &Path,
    path_fragments: &[&str],
  ) -> Option<(SourceModuleName, SourceFilePath)> {
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
      let module_name = module_prefix
        .as_slice()
        .iter()
        .map(|s| s.as_str())
        .chain(module_name_builder)
        .collect::<Vec<_>>()
        .join("::");

      let module_name = SourceModuleName::new(module_name);
      let source_path = SourceFilePath::new(path);
      Some((module_name, source_path))
    }
  }

  fn generate_paths_for_dir<'a>(
    module_prefix: &'a Option<String>,
    import_parts: SmallVec<[&'a str; 10]>,
    from_dir: &'a Path,
    current_source_path: &'a SourceFilePath,
  ) -> impl Iterator<Item = (SourceModuleName, SourceFilePath)> + 'a {
    (0..import_parts.len())
      .filter_map(move |i| {
        Self::create_path(module_prefix, from_dir, &import_parts[0..=i])
      })
      .filter(|(_, path)| path.as_ref() != current_source_path.as_path())
      .rev()
  }

  /// Generates possible import paths for a given import path fragment.
  pub fn generate_best_possible_paths(
    &self,
    import_path_part: &ImportPathPart,
    source_path: &SourceFilePath,
  ) -> FxIndexSet<(SourceModuleName, SourceFilePath)> {
    let import_parts: SmallVec<[&str; 10]> = import_path_part
      .split("::")
      .enumerate()
      .skip_while(|(i, part)| {
        *i == 0 && self.entry_module_prefix == Some(part.to_string()) // skip the first part
      })
      .map(|(_, part)| part)
      .filter(|part| !part.is_empty())
      .collect();

    if import_parts.is_empty() {
      panic!("import module is empty")
    }

    let source_dir = source_path.parent().unwrap_or(Path::new(""));

    let mut paths = Self::generate_paths_for_dir(
      &self.entry_module_prefix,
      import_parts.clone(),
      &self.workspace_root,
      source_path,
    )
    .chain(Self::generate_paths_for_dir(
      &self.entry_module_prefix,
      import_parts.clone(),
      source_dir,
      source_path,
    ))
    .collect::<FxIndexSet<_>>();

    for scan_dir in &self.additional_scan_dirs {
      let scan_path = Path::new(&scan_dir.directory);
      paths.extend(Self::generate_paths_for_dir(
        &scan_dir.module_import_root,
        import_parts.clone(),
        scan_path,
        source_path,
      ))
    }

    paths
  }
}

#[cfg(test)]
mod tests {
  use indexmap::indexset;
  use pretty_assertions::assert_eq;

  use crate::bevy_util::ModulePathResolver;
  use crate::{ImportPathPart, SourceFilePath, SourceModuleName};

  #[test]
  fn should_generate_single_import_path() {
    let module_prefix = None;
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let import_path_part = ImportPathPart::new("Fragment");

    let result = ModulePathResolver::new("mydir".into(), module_prefix, vec![])
      .generate_best_possible_paths(&import_path_part, &source_path);

    let expected = indexset![(
      SourceModuleName::new("Fragment"),
      SourceFilePath::new("mydir/Fragment.wgsl")
    )];

    assert_eq!(expected, result);
  }

  #[test]
  fn should_generate_single_import_path_when_module_prefix_match() {
    let module_prefix = Some("mymod".to_string());
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let import_path_part = ImportPathPart::new("mymod::Fragment");

    let result = ModulePathResolver::new("mydir".into(), module_prefix, vec![])
      .generate_best_possible_paths(&import_path_part, &source_path);

    let expected = indexset![(
      SourceModuleName::new("mymod::Fragment"),
      SourceFilePath::new("mydir/Fragment.wgsl")
    )];

    assert_eq!(expected, result);
  }

  // Should generate import paths with correct extensions
  #[test]
  fn should_generate_import_paths_with_correct_extensions() {
    let module_prefix = Some("prefix".to_string());
    let source_path = SourceFilePath::new("mydir/source");
    let import_path_part = ImportPathPart::new("Module::Submodule::Fragment");

    let actual = ModulePathResolver::new("mydir".into(), module_prefix, vec![])
      .generate_best_possible_paths(&import_path_part, &source_path);

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
    assert_eq!(expected, actual);
  }

  #[test]
  #[should_panic]
  fn should_panic_when_import_module_is_empty() {
    let module_prefix = None;
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let import_path_part = ImportPathPart::new("");

    let result = ModulePathResolver::new("mydir".into(), module_prefix, vec![])
      .generate_best_possible_paths(&import_path_part, &source_path);

    let expected = indexset![];

    assert_eq!(expected, result);
  }

  // Should return an empty SmallVec when import_module has only the module prefix
  #[test]
  #[should_panic]
  fn should_return_empty_smallvec_when_import_module_has_only_module_prefix() {
    let module_prefix = Some("prefix".to_string());
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let import_path_part = ImportPathPart::new("prefix");

    let result = ModulePathResolver::new("mydir".into(), module_prefix, vec![])
      .generate_best_possible_paths(&import_path_part, &source_path);

    let expected = indexset![];

    assert_eq!(expected, result);
  }

  #[test]
  fn should_return_smallvec_when_import_module() {
    let module_prefix = Some("prefix".to_string());
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let import_path_part = ImportPathPart::new("Fragment");

    let result = ModulePathResolver::new("mydir".into(), module_prefix, vec![])
      .generate_best_possible_paths(&import_path_part, &source_path);

    let expected = indexset![(
      SourceModuleName::new("prefix::Fragment"),
      SourceFilePath::new("mydir/Fragment.wgsl")
    )];

    assert_eq!(expected, result);
  }

  #[test]
  fn should_return_valid_pbr_paths_from_repeated_part() {
    let module_prefix = Some("bevy_pbr".to_string());
    let source_path = SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/functions.wgsl");
    let import_path_part = ImportPathPart::new("bevy_pbr::pbr::types");

    let result =
      ModulePathResolver::new("tests/bevy_pbr_wgsl/pbr".into(), module_prefix, vec![])
        .generate_best_possible_paths(&import_path_part, &source_path);

    let expected = indexset![(
      SourceModuleName::new("bevy_pbr::types"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/types.wgsl")
    )];

    assert_eq!(expected, result);
  }

  #[test]
  fn should_return_valid_pbr_paths_back_to_current_dir() {
    let module_prefix = Some("bevy_pbr".to_string());
    let source_path = SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/functions.wgsl");
    let import_path_part = ImportPathPart::new("bevy_pbr::mesh_types");

    let result =
      ModulePathResolver::new("tests/bevy_pbr_wgsl".into(), module_prefix, vec![])
        .generate_best_possible_paths(&import_path_part, &source_path);

    let expected = indexset![
      (
        SourceModuleName::new("bevy_pbr::mesh_types"),
        SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_types.wgsl"),
      ),
      (
        SourceModuleName::new("bevy_pbr::mesh_types"),
        SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/mesh_types.wgsl")
      )
    ];

    assert_eq!(expected, result);
  }
}
