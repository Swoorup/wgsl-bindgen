use smallvec::SmallVec;

use super::parse_imports;
use super::parse_imports::ImportStatement;
use crate::{
  types::{FxIndexSet, SourceFilePath},
  SourceModuleName,
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

  pub fn get_imported_modules(&self) -> FxIndexSet<SourceModuleName> {
    self
      .imports
      .iter()
      .flat_map(|import_stmt| import_stmt.get_imported_modules())
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use indexmap::{indexset, IndexMap};
  use pretty_assertions::assert_eq;
  use smallvec::smallvec;

  use super::*;
  use crate::SourceLocation;

  const TEST_IMPORTS: &'static str = r#"
#import a::b::{c::{d, e}, f, g::{h as i, j}}
#import a::b c, d
#import a, b
#import "path//with\ all sorts of .stuff"::{a, b}
#import a::b::{
    c::{d, e}, 
    f, 
    g::{
        h as i, 
        j::k::l as m,
    }
}
"#;

  fn create_index_map(values: Vec<(&str, Vec<&str>)>) -> IndexMap<String, Vec<String>> {
    let mut m = IndexMap::default();
    for (k, v) in values {
      let _ = m.insert(k.to_string(), v.into_iter().map(String::from).collect());
    }
    m
  }

  #[test]
  fn test_parsing_from_contents() {
    let source_file =
      SourceFile::create(PathBuf::from("").into(), None, TEST_IMPORTS.to_owned());
    let actual = source_file.imports;

    let expected: SmallVec<[ImportStatement; 4]> = smallvec![
      ImportStatement {
        source_location: SourceLocation {
          line_number: 1,
          line_position: 1,
          offset: 1,
          length: 44,
        },
        item_to_module_paths: create_index_map(vec![
          ("d", vec!["a::b::c::d"]),
          ("e", vec!["a::b::c::e"]),
          ("f", vec!["a::b::f"]),
          ("i", vec!["a::b::g::h"]),
          ("j", vec!["a::b::g::j",]),
        ]),
      },
      ImportStatement {
        source_location: SourceLocation {
          line_number: 2,
          line_position: 1,
          offset: 46,
          length: 17,
        },
        item_to_module_paths: create_index_map(vec![
          ("c", vec!["a::b::c"]),
          ("d", vec!["a::b::d"]),
        ]),
      },
      ImportStatement {
        source_location: SourceLocation {
          line_number: 3,
          line_position: 1,
          offset: 64,
          length: 12,
        },
        item_to_module_paths: create_index_map(vec![("a", vec!["a"]), ("b", vec!["b"]),]),
      },
      ImportStatement {
        source_location: SourceLocation {
          line_number: 4,
          line_position: 1,
          offset: 77,
          length: 49,
        },
        item_to_module_paths: create_index_map(vec![
          ("a", vec!["\"path//with\\ all sorts of .stuff\"::a"]),
          ("b", vec!["\"path//with\\ all sorts of .stuff\"::b"]),
        ]),
      },
      ImportStatement {
        source_location: SourceLocation {
          line_number: 5,
          line_position: 1,
          offset: 127,
          length: 95,
        },
        item_to_module_paths: create_index_map(vec![
          ("d", vec!["a::b::c::d"]),
          ("e", vec!["a::b::c::e"]),
          ("f", vec!["a::b::f"]),
          ("i", vec!["a::b::g::h"]),
          ("m", vec!["a::b::g::j::k::l"]),
        ]),
      }
    ];

    assert_eq!(actual, expected);

    assert_eq!(&source_file.content[actual[1].range()], "#import a::b c, d");
  }

  #[test]
  fn test_parsing_imports_from_bevy_mesh_view_bindings() {
    let module_name = Some(SourceModuleName::new("bevy_pbr::mesh_view_bindings"));
    let source_path = SourceFilePath::new("mesh_view_bindings.wgsl");
    let source = SourceFile::create(
      source_path,
      module_name,
      include_str!("../../tests/bevy_pbr_wgsl/mesh_view_bindings.wgsl").to_owned(),
    );
    let actual = source.get_imported_modules();

    assert_eq!(
      actual,
      indexset! {
        SourceModuleName::new("bevy_pbr::mesh_view_types")
      }
    );
  }
}
