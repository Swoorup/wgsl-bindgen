use std::path::{Path, PathBuf};

use colored::*;
use indexmap::map::Entry;
use miette::{Diagnostic, NamedSource, SourceSpan};
use smallvec::SmallVec;
use thiserror::Error;
use DependencyTreeError::*;

use super::{parse_imports::ImportStatement, source_file::SourceFile, escape_os_path};
use crate::{FxIndexMap, FxIndexSet, SourceFileDir, SourceFilePath, SourceModuleName};

#[derive(Debug, Error, Diagnostic)]
pub enum DependencyTreeError {
  #[error("Source file not found: {path}")]
  SourceNotFound { path: SourceFilePath },
  #[error("Cannot find import `{path}` in this scope")]
  #[diagnostic(help("Maybe a typo or a missing file."))]
  ImportPathNotFound {
    path: String,
    stmt: ImportStatement,

    #[source_code]
    src: NamedSource<String>,

    #[label("Import statement")]
    import_bit: SourceSpan,
  },
}

#[derive(Default)]
struct MaxRecursionLimiter {
  files_visited: Vec<(String, usize, String)>, // (file_path, line_number, import_str)
}

impl MaxRecursionLimiter {
  const MAX_RECURSION_DEPTH: usize = 16;

  fn push(&mut self, import_stmt: &ImportStatement, source: &SourceFile) -> &mut Self {
    let import_str = &source.content[import_stmt.range()];
    self.files_visited.push((
      source.file_path.to_string(),
      import_stmt.source_location.line_number,
      import_str.to_string(),
    ));
    self
  }

  fn pop(&mut self) -> &mut Self {
    self.files_visited.pop();
    self
  }

  fn check_depth(&self) {
    if self.files_visited.len() > Self::MAX_RECURSION_DEPTH {
      let visited_files = self
        .files_visited
        .iter()
        .map(|(path, line, import)| {
          format!(
            "\n{}:{}: {}",
            path.to_string().cyan(),
            line.to_string().purple(),
            import.to_string().yellow()
          )
        })
        .rev()
        .collect::<String>();

      panic!(
        "{}\n{}\n{}\n",
        "Recursion limit exceeded".red(),
        "This error may be due to a circular dependency. The files visited during the recursion were:".red(),
        visited_files
       );
    }
  }
}

#[derive(Debug, Clone)]
pub struct SourceWithFullDependenciesResult<'a> {
  pub source_file: &'a SourceFile,
  pub full_dependencies: SmallVec<[&'a SourceFile; 16]>,
}

#[derive(Debug)]
pub struct DependencyTree {
  module_prefix: Option<String>,
  parsed_sources: FxIndexMap<SourceFilePath, SourceFile>,
  entry_points: FxIndexSet<SourceFilePath>,
}

/// Represents a dependency tree for tracking the dependencies between source files.
///
/// The `DependencyTree` struct provides methods for generating possible import paths,
/// crawling import statements, crawling source files, building the dependency tree,
/// and retrieving all files including dependencies and the full dependency set for a given source file.
impl DependencyTree {
  /// Tries to build a dependency tree for the given entry points.
  ///
  /// This method takes a module prefix and a list of entry points (source file paths) and
  /// attempts to build a dependency tree by crawling the source files and resolving import
  /// statements. It returns a `Result` indicating whether the dependency tree was successfully
  /// built or an error occurred.
  ///
  /// # Arguments
  ///
  /// * `module_prefix` - An optional module prefix to be used when generating import paths.
  /// * `entry_points` - A vector of source file paths representing the entry points of the
  ///   dependency tree.
  ///
  /// # Returns
  ///
  /// A `Result` containing the built `DependencyTree` if successful, or a `DependencyTreeError`
  /// if an error occurred during the build process.
  pub fn try_build(
    module_prefix: Option<String>,
    entry_points: Vec<SourceFilePath>, // path to entry points
  ) -> Result<Self, DependencyTreeError> {
    let mut tree = Self {
      module_prefix,
      parsed_sources: Default::default(),
      entry_points: Default::default(),
    };

    for entry_point in entry_points {
      tree.entry_points.insert(entry_point.clone());
      tree.crawl_source(
        &entry_point.dir(),
        entry_point,
        None,
        &mut MaxRecursionLimiter::default(),
      )?
    }

    Ok(tree)
  }

  /// Generates possible import paths for a given import path fragment.
  fn generate_possible_module_paths(
    module_prefix: Option<&str>,
    entry_dir: &SourceFileDir,
    import_module: &SourceModuleName,
    current_source_path: &SourceFilePath,
  ) -> FxIndexSet<SourceFilePath> {
    let import_parts: SmallVec<[&str; 10]> = import_module
      .split("::")
      .enumerate()
      .skip_while(|(i, part)| {
        *i == 0 && module_prefix == Some(part) // skip the first part
      })
      .map(|(_, part)| part)
      .filter(|part| !part.is_empty())
      .collect();

    if import_parts.is_empty() {
      panic!("import module is empty")
    }

    let create_path = |root_dir: &Path, path_fragments: &[&str]| {
      let mut path = PathBuf::from(root_dir);
      for fragment in path_fragments {

        // Allow to use paths directly
        let fragment = escape_os_path(fragment); 
        
        // avoid duplicates repeated patterns
        if !path.ends_with(&fragment) {
          path.push(fragment);
        }
      }

      if path.extension().is_none() {
        path.set_extension("wgsl");
      }

      SourceFilePath::new(path)
    };

    let current_source_dir = current_source_path.parent().unwrap_or(Path::new(""));
    let paths = (0..import_parts.len())
      .map(|i| create_path(&current_source_dir, &import_parts[0..=i]))
      .filter(|path| path.as_ref() != current_source_path.as_path())
      .rev();

    // obvious path are directly appended from the entry root, and first prioritized
    let obvious_path = [create_path(&entry_dir, &import_parts)];

    obvious_path.into_iter().chain(paths).collect()
  }

  /// Crawls an import statement and resolves the import paths.
  fn crawl_import_module(
    &mut self,
    entry_dir: &SourceFileDir,
    parent_source_path: &SourceFilePath,
    import_stmt: &ImportStatement,
    import_module: &SourceModuleName,
    limiter: &mut MaxRecursionLimiter,
  ) -> Result<(), DependencyTreeError> {
    let possible_import_path = Self::generate_possible_module_paths(
      self.module_prefix.as_deref(),
      &entry_dir,
      &import_module,
      parent_source_path,
    )
    .into_iter()
    .find(|path| path.is_file()); // make sure this is not reimporting itself

    let Some(parent_source) = self.parsed_sources.get_mut(parent_source_path) else {
      unreachable!("{:?} source code as not parsed", parent_source_path)
    };

    let Some(import_path) = possible_import_path else {
      return Err(ImportPathNotFound {
        stmt: import_stmt.clone(),
        path: import_module.to_string(),
        import_bit: (&import_stmt.source_location).into(),
        src: NamedSource::new(
          parent_source_path.to_string(),
          parent_source.content.clone(),
        ),
      });
    };

    // add self as a dependency to the parent
    parent_source.add_direct_dependency(import_path.clone());

    limiter.push(import_stmt, parent_source).check_depth();

    // if not crawled, crawl this import file
    if !self.parsed_sources.contains_key(&import_path) {
      self
        .crawl_source(entry_dir, import_path, Some(import_module).cloned(), limiter)
        .expect("failed to crawl import path");
    }

    limiter.pop();

    Ok(())
  }

  /// Crawls a source file and its dependencies.
  fn crawl_source(
    &mut self,
    entry_dir: &SourceFileDir,
    source_path: SourceFilePath,
    module_name: Option<SourceModuleName>,
    limiter: &mut MaxRecursionLimiter,
  ) -> Result<(), DependencyTreeError> {
    match self.parsed_sources.entry(source_path.clone()) {
      Entry::Occupied(_) => {} // do nothing
      Entry::Vacant(entry) => {
        let content = entry.key().read_contents().or(Err(SourceNotFound {
          path: entry.key().clone(),
        }))?;

        let source_file =
          SourceFile::create(entry.key().clone(), module_name.clone(), content);
        entry.insert(source_file);
      }
    };

    let source_file = self.parsed_sources.get(&source_path).unwrap();

    for import_stmt in &source_file.imports.clone() {
      for import_module in import_stmt.get_imported_modules() {
        self.crawl_import_module(
          entry_dir,
          &source_path,
          &import_stmt,
          &import_module,
          limiter,
        )?
      }
    }

    Ok(())
  }

  /// Returns all the source files including their dependencies in the dependency tree.
  pub fn all_files_including_dependencies(&self) -> FxIndexSet<SourceFilePath> {
    self.parsed_sources.keys().cloned().collect()
  }

  pub fn parsed_files(&self) -> Vec<&SourceFile> {
    self.parsed_sources.values().collect()
  }

  /// Returns the full set of dependencies for a given source file.
  pub fn get_full_dependency_for(
    &self,
    source_path: &SourceFilePath,
  ) -> FxIndexSet<SourceFilePath> {
    self
      .parsed_sources
      .get(source_path)
      .iter()
      .flat_map(|source| {
        source
          .direct_dependencies
          .iter()
          .flat_map(|dep| {
            let mut deps = FxIndexSet::default();
            let sub_deps = self.get_full_dependency_for(dep);
            // insert the imported deps first
            deps.extend(sub_deps);

            // insert the current dep last
            deps.insert(dep.clone());

            deps
          })
          .collect::<FxIndexSet<_>>()
      })
      .collect()
  }

  /// Returns the source files with their full dependencies in the dependency tree.
  ///
  /// This method returns a vector of `SourceWithFullDependenciesResult` structs, each containing
  /// a source file and its full set of dependencies. The full set of dependencies includes both
  /// direct and transitive dependencies.
  ///
  /// # Returns
  ///
  /// A vector of `SourceWithFullDependenciesResult` structs, each representing a source file
  /// along with its full set of dependencies.
  pub fn get_source_files_with_full_dependencies(
    &self,
  ) -> Vec<SourceWithFullDependenciesResult<'_>> {
    self
      .entry_points
      .iter()
      .map(|entry_point| {
        let source_file = self.parsed_sources.get(entry_point).unwrap();
        let full_dependencies = self
          .get_full_dependency_for(entry_point)
          .iter()
          .map(|dep| self.parsed_sources.get(dep).unwrap())
          .collect();

        SourceWithFullDependenciesResult {
          source_file,
          full_dependencies,
        }
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use indexmap::indexset;
  use pretty_assertions::assert_eq;

  use crate::bevy_util::DependencyTree;
  use crate::{SourceFileDir, SourceFilePath, SourceModuleName};

  #[test]
  fn should_generate_single_import_path() {
    let module_prefix = None;
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let import_module = SourceModuleName::new("Fragment");

    let result = DependencyTree::generate_possible_module_paths(
      module_prefix,
      &source_path.dir(),
      &import_module,
      &source_path,
    );

    let expected = indexset![SourceFilePath::new("mydir/Fragment.wgsl")];

    assert_eq!(result, expected);
  }

  #[test]
  fn should_generate_single_import_path_when_module_prefix_match() {
    let module_prefix = Some("mymod");
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let import_module = SourceModuleName::new("mymod::Fragment");

    let result = DependencyTree::generate_possible_module_paths(
      module_prefix,
      &source_path.dir(),
      &import_module,
      &source_path,
    );

    let expected = indexset![SourceFilePath::new("mydir/Fragment.wgsl")];

    assert_eq!(result, expected);
  }

  // Should generate import paths with correct extensions
  #[test]
  fn should_generate_import_paths_with_correct_extensions() {
    let module_prefix = Some("prefix");
    let source_path = SourceFilePath::new("mydir/source");
    let import_module = SourceModuleName::new("Module::Submodule::Fragment");

    let actual = DependencyTree::generate_possible_module_paths(
      module_prefix,
      &source_path.dir(),
      &import_module,
      &source_path,
    );

    let expected = indexset![
      SourceFilePath::new("mydir/Module/Submodule/Fragment.wgsl"),
      SourceFilePath::new("mydir/Module/Submodule.wgsl"),
      SourceFilePath::new("mydir/Module.wgsl"),
    ];
    assert_eq!(actual, expected);
  }

  #[test]
  #[should_panic]
  fn should_panic_when_import_module_is_empty() {
    let module_prefix = None;
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let import_module = SourceModuleName::new("");

    let result = DependencyTree::generate_possible_module_paths(
      module_prefix,
      &source_path.dir(),
      &import_module,
      &source_path,
    );

    let expected = indexset![];

    assert_eq!(result, expected);
  }

  // Should return an empty SmallVec when import_module has only the module prefix
  #[test]
  #[should_panic]
  fn should_return_empty_smallvec_when_import_module_has_only_module_prefix() {
    let module_prefix = Some("prefix");
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let import_module = SourceModuleName::new("prefix");

    let result = DependencyTree::generate_possible_module_paths(
      module_prefix,
      &source_path.dir(),
      &import_module,
      &source_path,
    );

    let expected = indexset![];

    assert_eq!(result, expected);
  }

  #[test]
  fn should_return_smallvec_when_import_module() {
    let module_prefix = Some("prefix");
    let source_path = SourceFilePath::new("mydir/source.wgsl");
    let import_module = SourceModuleName::new("Fragment");

    let result = DependencyTree::generate_possible_module_paths(
      module_prefix,
      &source_path.dir(),
      &import_module,
      &source_path,
    );

    let expected = indexset![SourceFilePath::new("mydir/Fragment.wgsl")];

    assert_eq!(result, expected);
  }

  #[test]
  fn should_return_valid_pbr_paths_from_repeated_part() {
    let module_prefix = Some("bevy_pbr");
    let source_path = SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/functions.wgsl");
    let import_module = SourceModuleName::new("bevy_pbr::pbr::types");

    let result = DependencyTree::generate_possible_module_paths(
      module_prefix,
      &source_path.dir(),
      &import_module,
      &source_path,
    );

    let expected = indexset![
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/types.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr.wgsl")
    ];

    assert_eq!(result, expected);
  }

  #[test]
  fn should_return_valid_pbr_paths_back_to_current_dir() {
    let module_prefix = Some("bevy_pbr");
    let entry_dir = SourceFileDir::new("tests/bevy_pbr_wgsl");
    let source_path = SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/functions.wgsl");
    let import_module = SourceModuleName::new("bevy_pbr::mesh_types");

    let result = DependencyTree::generate_possible_module_paths(
      module_prefix,
      &entry_dir,
      &import_module,
      &source_path,
    );

    let expected = indexset![
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_types.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/mesh_types.wgsl")
    ];

    assert_eq!(result, expected);
  }
}
