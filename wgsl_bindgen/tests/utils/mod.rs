use indexmap::indexset;
use miette::IntoDiagnostic;
use pretty_assertions::assert_eq;
use wgsl_bindgen::bevy_util::DependencyTree;
use wgsl_bindgen::SourceFilePath;

/// Build a dependency tree for the WESL feature test shaders.
/// `test_shader.wgsl` imports `shared.wgsl` via `import package::shared::apply_scale`.
fn build_wesl_deptree() -> DependencyTree {
  DependencyTree::try_build(
    "tests/shaders/features/wesl".into(),
    None,
    vec![SourceFilePath::new(
      "tests/shaders/features/wesl/test_shader.wgsl",
    )],
    vec![],
  )
  .into_diagnostic()
  .expect("build_wesl_deptree error")
}

#[test]
fn test_dependency_tree_file_enumeration() {
  let deptree = build_wesl_deptree();

  // Both the entry point and the imported module should be tracked.
  assert_eq!(
    indexset![
      SourceFilePath::new("tests/shaders/features/wesl/test_shader.wgsl"),
      SourceFilePath::new("tests/shaders/features/wesl/shared.wgsl"),
    ],
    deptree.all_files_including_dependencies(),
  );
}

#[test]
fn test_dependency_tree_full_dependencies() {
  let deptree = build_wesl_deptree();
  let results = deptree.get_source_files_with_full_dependencies();

  // Only one entry point; it should have `shared.wgsl` as a dependency.
  assert_eq!(1, results.len());
  let entry = &results[0];
  assert_eq!(
    SourceFilePath::new("tests/shaders/features/wesl/test_shader.wgsl"),
    entry.source_file.file_path
  );
  // The shared module should appear as a dependency.
  let dep_paths: Vec<_> = entry
    .full_dependencies
    .iter()
    .map(|d| d.file_path.clone())
    .collect();
  assert!(
    dep_paths.contains(&SourceFilePath::new(
      "tests/shaders/features/wesl/shared.wgsl"
    )),
    "Expected shared.wgsl in dependencies, got: {dep_paths:?}"
  );
}

#[test]
fn test_dependency_tree_order() {
  let deptree = build_wesl_deptree();
  let deps = deptree
    .get_full_dependency_for(&SourceFilePath::new(
      "tests/shaders/features/wesl/test_shader.wgsl",
    ))
    .into_iter()
    .collect::<Vec<_>>();

  // The shared module is a dependency of the entry point.
  assert!(deps.contains(&SourceFilePath::new(
    "tests/shaders/features/wesl/shared.wgsl"
  )));
}
