use std::borrow::Cow;
use std::sync::OnceLock;

use regex::Regex;
use smallvec::SmallVec;

const DECORATION_PRE: &str = "X_naga_oil_mod_X";
const DECORATION_POST: &str = "X";

fn undecorate_regex() -> &'static Regex {
  static MEM: OnceLock<Regex> = OnceLock::new();

  MEM.get_or_init(|| {
    // https://github.com/bevyengine/naga_oil/blob/master/src/compose/mod.rs#L355-L363
    Regex::new(
      format!(
        r"(\x1B\[\d+\w)?([\w\d_]+){}([A-Z0-9]*){}",
        regex_syntax::escape(DECORATION_PRE),
        regex_syntax::escape(DECORATION_POST)
      )
      .as_str(),
    )
    .unwrap()
  })
}

// https://github.com/bevyengine/naga_oil/blob/master/src/compose/mod.rs#L417-L419
fn decode(from: &str) -> String {
  String::from_utf8(data_encoding::BASE32_NOPAD.decode(from.as_bytes()).unwrap()).unwrap()
}

pub fn escape_os_path(path: &str) -> String {
  path.replace("\"", "")
}

/// Converts
///   * "\"../types\"::RtsStruct" => "types::RtsStruct"
///   * "../more-shader-files/reachme" => "reachme"
pub fn make_valid_rust_import(value: &str) -> String {
  let v = value.replace("\"../", "").replace("\"", "");
  std::path::Path::new(&v)
    .file_stem()
    .and_then(|name| name.to_str())
    .unwrap_or(&v)
    .to_string()
}

// https://github.com/bevyengine/naga_oil/blob/master/src/compose/mod.rs#L421-L431
pub fn demangle(string: &str) -> Cow<str> {
  undecorate_regex().replace_all(string, |caps: &regex::Captures| {
    format!(
      "{}{}::{}",
      caps.get(1).map(|cc| cc.as_str()).unwrap_or(""),
      make_valid_rust_import(&decode(caps.get(3).unwrap().as_str())),
      caps.get(2).unwrap().as_str()
    )
  })
}

/// Demangles a string representing a module path and item name, splitting them into separate parts.
pub fn demangle_splitting_mod_path_and_item(string: &str) -> (Option<String>, String) {
  let demangled = demangle(string);
  let mut parts = demangled
    .as_ref()
    .split("::")
    .collect::<SmallVec<[&str; 4]>>();
  if parts.len() == 1 {
    (None, parts[0].into())
  } else {
    let item = parts.pop().unwrap();
    let mod_path = parts.join("::");
    (Some(mod_path), item.to_string())
  }
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;

  use crate::bevy_util::{demangle_splitting_mod_path_and_item, make_valid_rust_import};

  #[test]
  fn test_make_valid_rust_import() {
    assert_eq!(make_valid_rust_import("\"../types\"::RtsStruct"), "types::RtsStruct");
    assert_eq!(make_valid_rust_import("../more-shader-files/reachme"), "reachme");
  }

  #[test]
  fn test_demangle_mod_names() {
    assert_eq!(
      demangle_splitting_mod_path_and_item("SnehaDataX_naga_oil_mod_XOM5DU5DZOBSXGX"),
      (Some("s::types".into()), "SnehaData".into())
    );

    assert_eq!(
      demangle_splitting_mod_path_and_item("UniformsX_naga_oil_mod_XOR4XAZLTX"),
      (Some("types".into()), "Uniforms".into())
    );
  }
}
