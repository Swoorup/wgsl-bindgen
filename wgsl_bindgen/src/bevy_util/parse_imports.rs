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

/// Collect all WESL import statements from shader content.
///
/// Only WESL-style `import package::module::item;` syntax is recognised.
/// The legacy naga-oil `#import` syntax is not supported.
pub fn get_import_statements<B: FromIterator<ImportStatement>>(content: &str) -> B {
  parse_wesl_import_statements_iter(content).collect::<B>()
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
  use smallvec::{smallvec, SmallVec};

  use super::*;

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
  fn test_get_import_statements_wesl_only() {
    // get_import_statements now only recognises WESL-style `import package::` statements.
    let source = "import package::wesl_module::func;\n";
    let all: SmallVec<[ImportStatement; 4]> = get_import_statements(source);
    assert_eq!(all.len(), 1);
  }
}
