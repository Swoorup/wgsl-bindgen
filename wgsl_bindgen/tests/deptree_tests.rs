use std::sync::OnceLock;

use indexmap::{indexmap, indexset, IndexMap};
use miette::IntoDiagnostic;
use pretty_assertions::assert_eq;
use wgsl_bindgen::bevy_util::DependencyTree;
use wgsl_bindgen::SourceFilePath;

pub type SourceDependencyMap =
  IndexMap<SourceFilePath, IndexMap<&'static str, &'static str>>;

pub fn bevy_dependency_map() -> &'static SourceDependencyMap {
  static MEM: OnceLock<SourceDependencyMap> = OnceLock::new();
  MEM.get_or_init(|| {
    indexmap![
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh.wgsl") => indexmap![
        "bevy_pbr::mesh_view_bindings" => "tests/bevy_pbr_wgsl/mesh_view_bindings.wgsl",
        "bevy_pbr::mesh_view_types"    => "tests/bevy_pbr_wgsl/mesh_view_types.wgsl",
        "bevy_pbr::mesh_bindings"      => "tests/bevy_pbr_wgsl/mesh_bindings.wgsl",
        "bevy_pbr::mesh_types"         => "tests/bevy_pbr_wgsl/mesh_types.wgsl",
        "bevy_pbr::mesh_functions"     => "tests/bevy_pbr_wgsl/mesh_functions.wgsl",
        "bevy_pbr::mesh_vertex_output" => "tests/bevy_pbr_wgsl/mesh_vertex_output.wgsl",
      ],
      SourceFilePath::new("tests/bevy_pbr_wgsl/output_VERTEX_UVS.wgsl") => indexmap![],
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr.wgsl") => indexmap![
        "bevy_pbr::mesh_vertex_output"  => "tests/bevy_pbr_wgsl/mesh_vertex_output.wgsl",
        "bevy_pbr::pbr::functions"      => "tests/bevy_pbr_wgsl/pbr/functions.wgsl",
        "bevy_pbr::pbr::types"          => "tests/bevy_pbr_wgsl/pbr/types.wgsl",
        "bevy_pbr::mesh_types"          => "tests/bevy_pbr_wgsl/mesh_types.wgsl",
        "bevy_pbr::mesh_bindings"       => "tests/bevy_pbr_wgsl/mesh_bindings.wgsl",
        "bevy_pbr::mesh_view_types"     => "tests/bevy_pbr_wgsl/mesh_view_types.wgsl",
        "bevy_pbr::mesh_view_bindings"  => "tests/bevy_pbr_wgsl/mesh_view_bindings.wgsl",
        "bevy_pbr::lighting"       => "tests/bevy_pbr_wgsl/pbr/lighting.wgsl",
        "bevy_pbr::utils"               => "tests/bevy_pbr_wgsl/utils.wgsl",
        "bevy_pbr::clustered_forward"   => "tests/bevy_pbr_wgsl/clustered_forward.wgsl",
        "bevy_pbr::shadows"             => "tests/bevy_pbr_wgsl/shadows.wgsl",
        "bevy_pbr::pbr::bindings"       => "tests/bevy_pbr_wgsl/pbr/bindings.wgsl",
      ],
      SourceFilePath::new("tests/bevy_pbr_wgsl/wireframe.wgsl") => indexmap![
        "bevy_pbr::mesh_types"         => "tests/bevy_pbr_wgsl/mesh_types.wgsl",
        "bevy_pbr::mesh_view_bindings" => "tests/bevy_pbr_wgsl/mesh_view_bindings.wgsl",
        "bevy_pbr::mesh_view_types"    => "tests/bevy_pbr_wgsl/mesh_view_types.wgsl",
        "bevy_pbr::skinning"           => "tests/bevy_pbr_wgsl/skinning.wgsl",
        "bevy_pbr::mesh_functions"     => "tests/bevy_pbr_wgsl/mesh_functions.wgsl",
      ],
    ]
  })
}

fn build_bevy_deptree() -> DependencyTree {
  DependencyTree::try_build(
    Some("bevy_pbr".into()),
    vec![
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/output_VERTEX_UVS.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/wireframe.wgsl"),
    ],
  )
  .into_diagnostic()
  .expect("build_bevy_deptree error")
}

#[test]
fn test_bevy_all_files_enumeration() {
  let deptree = build_bevy_deptree();

  assert_eq!(
    deptree.all_files_including_dependencies(),
    indexset![
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_view_bindings.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_view_types.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_bindings.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_types.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_functions.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_vertex_output.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/output_VERTEX_UVS.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/functions.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/types.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/lighting.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/utils.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/clustered_forward.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/shadows.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/pbr/bindings.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/wireframe.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/skinning.wgsl"),
    ]
  )
}

#[test]
fn test_bevy_full_dependencies() {
  let expected = bevy_dependency_map();

  let deptree = build_bevy_deptree();
  let actual = deptree
    .get_source_files_with_full_dependencies()
    .into_iter()
    .map(|source| {
      let source_path = source.source_file.file_path.clone();
      let dependencies = source
        .full_dependencies
        .into_iter()
        .map(|dep| {
          let module_name = dep.module_name.as_ref().unwrap().as_str();
          let module_path = dep.file_path.to_str().unwrap();
          (module_name, module_path)
        })
        .collect::<IndexMap<_, _>>();
      (source_path, dependencies)
    })
    .collect::<IndexMap<_, _>>();

  assert_eq!(&actual, expected);
}

#[test]
fn test_example_wgsl_dep_tree() {
  let result = DependencyTree::try_build(
    None,
    vec![SourceFilePath::new("../example/src/shader/testbed.wgsl")],
  )
  .unwrap();

  assert_eq!(
    result.all_files_including_dependencies(),
    indexset![
      SourceFilePath::new("../example/src/shader/testbed.wgsl"),
      SourceFilePath::new("../example/src/shader/types.wgsl")
    ]
  )
}

#[test]
fn test_bevy_mesh_wgsl_dependency_order() {
  let deptree = build_bevy_deptree();
  let deps = deptree
    .get_full_dependency_for(&SourceFilePath::new("tests/bevy_pbr_wgsl/mesh.wgsl"))
    .into_iter()
    .collect::<Vec<_>>();

  assert_eq!(
    deps,
    vec![
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_view_types.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_view_bindings.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_types.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_bindings.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_functions.wgsl"),
      SourceFilePath::new("tests/bevy_pbr_wgsl/mesh_vertex_output.wgsl")
    ]
  );
}
