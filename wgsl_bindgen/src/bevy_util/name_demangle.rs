use std::borrow::Cow;
use std::sync::OnceLock;

use regex::Regex;
use smallvec::SmallVec;

use crate::quote_gen::RustItemPath;

const DECORATION_PRE: &str = "X_naga_oil_mod_X";
const DECORATION_POST: &str = "X";

impl RustItemPath {
  /// Demangles a string representing a module path and item name, splitting them into separate parts.
  pub fn from_mangled(string: &str, default_module_path: &str) -> Self {
    let demangled = demangle_str(string);
    let mut parts = demangled
      .as_ref()
      .split("::")
      .collect::<SmallVec<[&str; 4]>>();

    let (mod_path, item) = if parts.len() == 1 {
      (default_module_path.into(), parts[0])
    } else {
      let item = parts.pop().unwrap();
      let mod_path = parts.join("::");
      (mod_path.into(), item)
    };

    Self {
      module: mod_path,
      name: item.into(),
    }
  }
}

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
pub fn demangle_str(string: &str) -> Cow<str> {
  undecorate_regex().replace_all(string, |caps: &regex::Captures| {
    format!(
      "{}{}::{}",
      caps.get(1).map(|cc| cc.as_str()).unwrap_or(""),
      make_valid_rust_import(&decode(caps.get(3).unwrap().as_str())),
      caps.get(2).unwrap().as_str()
    )
  })
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;

  use crate::bevy_util::make_valid_rust_import;
  use crate::quote_gen::RustItemPath;

  #[test]
  fn test_make_valid_rust_import() {
    assert_eq!(make_valid_rust_import("\"../types\"::RtsStruct"), "types::RtsStruct");
    assert_eq!(make_valid_rust_import("../more-shader-files/reachme"), "reachme");
  }

  #[test]
  fn test_demangle_mod_names() {
    assert_eq!(
      RustItemPath::from_mangled("SnehaDataX_naga_oil_mod_XOM5DU5DZOBSXGX", ""),
      RustItemPath {
        module: "s::types".into(),
        name: "SnehaData".into()
      }
    );

    assert_eq!(
      RustItemPath::from_mangled("UniformsX_naga_oil_mod_XOR4XAZLTX", ""),
      RustItemPath {
        module: "types".into(),
        name: "Uniforms".into()
      }
    );
  }
}
