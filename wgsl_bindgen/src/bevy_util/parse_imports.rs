use std::{ops::Range, sync::OnceLock};

use indexmap::IndexMap;
use regex::Regex;

use crate::{FxIndexSet, SourceLocation, SourceModuleName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportStatement {
  pub source_location: SourceLocation,
  pub item_to_module_paths: IndexMap<String, Vec<String>>,
}

impl ImportStatement {
  pub fn range(&self) -> Range<usize> {
    let start = self.source_location.offset;
    let end = start + self.source_location.length;
    start..end
  }

  pub fn get_imported_modules(&self) -> FxIndexSet<SourceModuleName> {
    self
      .item_to_module_paths
      .values()
      .flatten()
      .map(SourceModuleName::new)
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
    .expect(format!("failed to parse imports: '{}'", input).as_str());
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

pub(crate) fn iter_import_statements(
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
        item_to_module_paths,
      };

      Some(import_stmt)
    } else {
      None
    }
  })
}

pub fn get_import_statements<B: FromIterator<ImportStatement>>(content: &str) -> B {
  iter_import_statements(content).collect::<B>()
}

#[cfg(test)]
mod tests {
  use crate::bevy_util::parse_imports;
  use crate::SourceModuleName;

  #[test]
  fn test_parsing_imports_from_bevy_mesh_view_bindings() {
    let contents = include_str!("../../tests/bevy_pbr_wgsl/mesh_view_bindings.wgsl");
    let actual = parse_imports::iter_import_statements(contents)
      .flat_map(|x| x.get_imported_modules())
      .collect::<Vec<_>>();

    assert_eq!(actual, vec![SourceModuleName::new("bevy_pbr::mesh_view_types")]);
  }
}
