use std::ops::Range;
use std::sync::OnceLock;

use indexmap::IndexMap;
use regex::Regex;

use crate::{FxIndexSet, ImportPathPart, SourceLocation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportStatement {
  pub source_location: SourceLocation,
  pub item_to_import_paths: IndexMap<String, Vec<String>>,
}

impl ImportStatement {
  pub fn range(&self) -> Range<usize> {
    let start = self.source_location.offset;
    let end = start + self.source_location.length;
    start..end
  }

  pub fn get_import_path_parts(&self) -> FxIndexSet<ImportPathPart> {
    self
      .item_to_import_paths
      .values()
      .flatten()
      .map(ImportPathPart::new)
      .collect()
  }
}

fn import_prefix_regex() -> &'static Regex {
  static MEM: OnceLock<Regex> = OnceLock::new();
  MEM.get_or_init(|| Regex::new(r"(?m)^\s*(#import)").expect("Failed to compile regex"))
}

fn parse_import_stmt(input: &str) -> IndexMap<String, Vec<String>> {
  let mut declared_imports = IndexMap::default();
  naga_oil::compose::parse_imports::parse_imports(input, &mut declared_imports)
    .unwrap_or_else(|_| panic!("failed to parse imports: '{input}'"));
  declared_imports
}

fn build_newline_offsets(content: &str) -> Vec<usize> {
  let mut line_starts = vec![];
  for (offset, c) in content.char_indices() {
    if c == '\n' {
      line_starts.push(offset + 1)
    }
  }
  line_starts
}

fn get_line_and_column(offset: usize, newline_offsets: &[usize]) -> (usize, usize) {
  let line_idx = newline_offsets.partition_point(|&x| x <= offset);
  let line_start = if line_idx == 0 {
    0
  } else {
    newline_offsets[line_idx - 1]
  };
  (line_idx, offset - line_start + 1)
}

pub(crate) fn parse_import_statements_iter(
  wgsl_content: &str,
) -> impl Iterator<Item = ImportStatement> + '_ {
  let mut start = 0;
  let line_offsets = build_newline_offsets(wgsl_content);

  std::iter::from_fn(move || {
    if let Some(c) = import_prefix_regex().captures(&wgsl_content[start..]) {
      let m = c.get(1).unwrap();
      let pos = m.start();
      let mut end = start + m.end();

      let mut brace_level = 0;
      let mut in_quotes = false;
      let mut prev_char = '\0';

      while let Some((i, c)) = wgsl_content[end..].char_indices().next() {
        match c {
          '{' if !in_quotes => brace_level += 1,
          '}' if !in_quotes => brace_level -= 1,
          '"' if prev_char != '\\' => in_quotes = !in_quotes,
          '\n' if !in_quotes && brace_level == 0 => {
            end += i;
            break;
          }
          _ => {}
        }
        prev_char = c;
        end += c.len_utf8();
      }
      let range = start + pos..end;
      let (line_number, line_position) = get_line_and_column(start + pos, &line_offsets);

      // advance the cursor
      start = end;

      let source_location = SourceLocation {
        line_number,
        line_position,
        length: range.len(),
        offset: range.start,
      };

      let item_to_module_paths = parse_import_stmt(&wgsl_content[range.clone()]);

      let import_stmt = ImportStatement {
        source_location,
        item_to_import_paths: item_to_module_paths,
      };

      Some(import_stmt)
    } else {
      None
    }
  })
}

pub fn get_import_statements<B: FromIterator<ImportStatement>>(content: &str) -> B {
  parse_import_statements_iter(content)
    .chain(parse_wesl_import_statements_iter(content))
    .collect::<B>()
}

// ─── WESL import syntax (`import package::module::item;`) ───────────────────

/// The WESL package-root prefix used in `import package::...` statements.
///
/// This constant is shared between the import parser and the WESL compilation
/// path in `wgsl_bindgen_impl.rs` to avoid duplication.
pub(crate) const WESL_PACKAGE_PREFIX: &str = "package::";

fn wesl_import_prefix_regex() -> &'static Regex {
  static MEM: OnceLock<Regex> = OnceLock::new();
  // Match lines starting with `import package::` (WESL absolute-package imports).
  // `super::` relative imports are not yet handled here.
  MEM.get_or_init(|| {
    Regex::new(r"(?m)^\s*(import\s+package::)")
      .expect("Failed to compile WESL import regex")
  })
}

/// Extract the module path part from a WESL `import` statement string.
///
/// Examples:
/// - `"import package::shared::apply_scale;"` → `"shared::apply_scale"`
/// - `"import package::module::{item1, item2};"` → `"module"`
/// - `"  import package::utils;"` → `"utils"`
fn parse_wesl_import_path(import_stmt: &str) -> Option<String> {
  // Strip leading whitespace and `import`
  let content = import_stmt
    .trim_start()
    .strip_prefix("import")?
    .trim_start()
    .strip_prefix(WESL_PACKAGE_PREFIX)?;

  // Remove trailing semicolon and whitespace
  let content = content.trim_end_matches(';').trim();

  // If there is a `::{}` collection at the end (e.g. `module::{item1, item2}`),
  // keep only the module part before the `::{}`.
  let path = if let Some(pos) = content.find("::{") {
    content[..pos].trim()
  } else {
    content
  };

  if path.is_empty() {
    return None;
  }

  Some(path.to_string())
}

/// Iterate over WESL-style `import package::...;` statements in shader source,
/// yielding an [`ImportStatement`] for each one.
///
/// This enables the dependency tree to track WESL-syntax shader imports for hot-reload
/// and caching purposes, without requiring naga-oil's `#import` parser.
fn parse_wesl_import_statements_iter(
  wgsl_content: &str,
) -> impl Iterator<Item = ImportStatement> + '_ {
  let mut start = 0;
  let line_offsets = build_newline_offsets(wgsl_content);

  std::iter::from_fn(move || {
    let remaining = &wgsl_content[start..];
    let cap = wesl_import_prefix_regex().captures(remaining)?;

    let m = cap.get(1).unwrap();
    let stmt_start = start + m.start();
    let mut end = start + m.end();

    // Scan to the end of the statement (semicolon).
    // Handle collections `import package::m::{a, b};` by counting braces.
    let mut brace_level: i32 = 0;
    for (i, ch) in wgsl_content[end..].char_indices() {
      match ch {
        '{' => brace_level += 1,
        '}' => {
          brace_level -= 1;
          if brace_level < 0 {
            // Malformed import — skip to the end of the input to prevent an
            // infinite loop on repeated calls with the same malformed content.
            end = wgsl_content.len();
            break;
          }
        }
        ';' if brace_level == 0 => {
          end += i + 1;
          break;
        }
        '\n' if brace_level == 0 => {
          // Unterminated single-line import – treat newline as end
          end += i;
          break;
        }
        _ => {}
      }
    }

    start = end;

    let import_text = &wgsl_content[stmt_start..end];
    let path = parse_wesl_import_path(import_text)?;

    let (line_number, line_position) = get_line_and_column(stmt_start, &line_offsets);
    let length = end - stmt_start;

    let mut item_to_import_paths = IndexMap::default();
    // Store the module path as both key and value; the dependency-tree resolver
    // will try successive path prefixes to locate the file on disk.
    item_to_import_paths.insert(path.clone(), vec![path]);

    Some(ImportStatement {
      source_location: SourceLocation {
        line_number,
        line_position,
        length,
        offset: stmt_start,
      },
      item_to_import_paths,
    })
  })
}

#[cfg(test)]
mod tests {
  use indexmap::indexset;
  use pretty_assertions::{assert_eq, assert_str_eq};
  use smallvec::{smallvec, SmallVec};

  use super::*;

  const TEST_IMPORTS: &str = r#"
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
    let test_imports = TEST_IMPORTS.replace("\r\n", "\n").replace("\r", "\n");
    let actual = parse_import_statements_iter(&test_imports)
      .collect::<SmallVec<[ImportStatement; 4]>>();

    let expected: SmallVec<[ImportStatement; 4]> = smallvec![
      ImportStatement {
        source_location: SourceLocation {
          line_number: 1,
          line_position: 1,
          offset: 1,
          length: 44,
        },
        item_to_import_paths: create_index_map(vec![
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
        item_to_import_paths: create_index_map(vec![
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
        item_to_import_paths: create_index_map(vec![("a", vec!["a"]), ("b", vec!["b"]),]),
      },
      ImportStatement {
        source_location: SourceLocation {
          line_number: 4,
          line_position: 1,
          offset: 77,
          length: 49,
        },
        item_to_import_paths: create_index_map(vec![
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
        item_to_import_paths: create_index_map(vec![
          ("d", vec!["a::b::c::d"]),
          ("e", vec!["a::b::c::e"]),
          ("f", vec!["a::b::f"]),
          ("i", vec!["a::b::g::h"]),
          ("m", vec!["a::b::g::j::k::l"]),
        ]),
      }
    ];

    assert_eq!(expected, actual);

    assert_str_eq!("#import a::b c, d", &test_imports[actual[1].range()]);
  }

  #[test]
  fn test_parsing_imports_from_bevy_mesh_view_bindings() {
    let contents =
      include_str!("../../tests/shaders/bevy_pbr_wgsl/mesh_view_bindings.wgsl");
    let actual = parse_import_statements_iter(contents)
      .flat_map(|x| x.get_import_path_parts())
      .collect::<Vec<_>>();

    assert_eq!(vec![ImportPathPart::new("bevy_pbr::mesh_view_types")], actual);
  }

  // ── WESL import parsing tests ──────────────────────────────────────────────

  #[test]
  fn test_wesl_parse_simple_item_import() {
    let source = "import package::shared::apply_scale;\n";
    let actual = parse_wesl_import_statements_iter(source)
      .collect::<SmallVec<[ImportStatement; 4]>>();
    assert_eq!(actual.len(), 1);
    assert_eq!(
      actual[0].get_import_path_parts(),
      indexset! { ImportPathPart::new("shared::apply_scale") }
    );
  }

  #[test]
  fn test_wesl_parse_module_import() {
    let source = "import package::utils;\n";
    let actual = parse_wesl_import_statements_iter(source)
      .collect::<SmallVec<[ImportStatement; 4]>>();
    assert_eq!(actual.len(), 1);
    assert_eq!(
      actual[0].get_import_path_parts(),
      indexset! { ImportPathPart::new("utils") }
    );
  }

  #[test]
  fn test_wesl_parse_collection_import() {
    let source = "import package::utils::{func_a, func_b};\n";
    let actual = parse_wesl_import_statements_iter(source)
      .collect::<SmallVec<[ImportStatement; 4]>>();
    assert_eq!(actual.len(), 1);
    // Collection imports strip the `::{}` part, yielding just the module path
    assert_eq!(
      actual[0].get_import_path_parts(),
      indexset! { ImportPathPart::new("utils") }
    );
  }

  #[test]
  fn test_wesl_parse_multiple_imports() {
    let source = "\
import package::shared::apply_scale;
import package::math::{dot, cross};
import package::common;
";
    let paths: Vec<_> = parse_wesl_import_statements_iter(source)
      .flat_map(|s| s.get_import_path_parts())
      .collect();
    assert_eq!(
      paths,
      vec![
        ImportPathPart::new("shared::apply_scale"),
        ImportPathPart::new("math"),
        ImportPathPart::new("common"),
      ]
    );
  }

  #[test]
  fn test_wesl_parse_does_not_match_naga_oil_imports() {
    let source = "#import naga_oil_style::module\nimport package::wesl_style;\n";
    // The naga-oil `#import` parser and the WESL `import` parser are independent.
    // The WESL parser should only find the WESL-style import.
    let wesl_paths: Vec<_> = parse_wesl_import_statements_iter(source)
      .flat_map(|s| s.get_import_path_parts())
      .collect();
    assert_eq!(wesl_paths, vec![ImportPathPart::new("wesl_style")]);
  }

  #[test]
  fn test_get_import_statements_combines_both_syntaxes() {
    let source = "\
#import naga_oil_module::item
import package::wesl_module::func;
";
    let all: SmallVec<[ImportStatement; 4]> = get_import_statements(source);
    // Should contain one #import and one WESL import
    assert_eq!(all.len(), 2);
  }
}
