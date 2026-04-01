use std::borrow::Cow;

use smallvec::SmallVec;
use wesl::syntax::PathOrigin;
use wesl::{EscapeMangler, Mangler};

use crate::quote_gen::RustSourceItemPath;

impl RustSourceItemPath {
  /// Demangles a WESL-mangled identifier (e.g. `package__1global_bindings_GlobalUniforms`)
  /// into a `RustSourceItemPath`.
  ///
  /// If the name is already a plain identifier (no mangling), the `default_module_path` is
  /// used as the module and the full string as the item name.
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

/// Attempt to demangle a WESL `EscapeMangler`-mangled identifier.
///
/// Returns `Some("module::path::Item")` when successful, `None` when the string does
/// not look like a WESL-mangled name (no `::` path components encoded).
fn wesl_demangle(mangled: &str) -> Option<String> {
  let (path, item) = EscapeMangler.unmangle(mangled)?;

  // Only handle package-absolute paths (origin == PathOrigin::Absolute).
  // These correspond to `import package::...` – the only form we emit.
  let components = match &path.origin {
    PathOrigin::Absolute => &path.components,
    // For relative/external-package paths fall back to the default behaviour.
    _ => return None,
  };

  if components.is_empty() {
    // Top-level item in the root package – no sub-module prefix.
    // Return None so the caller uses the invoking entry module instead.
    None
  } else {
    Some(format!("{}::{}", components.join("::"), item))
  }
}

/// Demangle a WGSL identifier produced by the WESL compiler.
///
/// * For WESL-mangled names (e.g. `package__1global_bindings_GlobalUniforms`) the
///   function returns the demangled `"module::Item"` form.
/// * For plain identifiers the input is returned unchanged.
pub fn demangle_str(string: &str) -> Cow<'_, str> {
  if let Some(demangled) = wesl_demangle(string) {
    Cow::Owned(demangled)
  } else {
    Cow::Borrowed(string)
  }
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

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;

  use crate::bevy_util::make_valid_rust_import;
  use crate::quote_gen::RustSourceItemPath;

  use super::demangle_str;
  use wesl::Mangler;

  #[test]
  fn test_make_valid_rust_import() {
    assert_eq!(make_valid_rust_import("\"../types\"::RtsStruct"), "types::RtsStruct");
    assert_eq!(make_valid_rust_import("../more-shader-files/reachme"), "reachme");
  }

  #[test]
  fn test_wesl_demangle_with_module() {
    // `package::global_bindings::GlobalUniforms`
    // EscapeMangler: "global_bindings" has 1 underscore → "_1global_bindings"
    // Mangle result: "package__1global_bindings_GlobalUniforms"
    assert_eq!(
      RustSourceItemPath::from_mangled("package__1global_bindings_GlobalUniforms", "entry"),
      RustSourceItemPath {
        module: "global_bindings".into(),
        name: "GlobalUniforms".into(),
      }
    );
  }

  #[test]
  fn test_wesl_demangle_nested_module() {
    // `package::compute_demo::particle_physics::Particle`
    let mangled = wesl::EscapeMangler
      .mangle(&"package::compute_demo::particle_physics".parse().unwrap(), "Particle");
    let result = RustSourceItemPath::from_mangled(&mangled, "entry");
    assert_eq!(result.module.as_str(), "compute_demo::particle_physics");
    assert_eq!(result.name.as_str(), "Particle");
  }

  #[test]
  fn test_plain_name_uses_default_module() {
    // A name with no mangling stays in the default module
    assert_eq!(
      RustSourceItemPath::from_mangled("Uniforms", "my_shader"),
      RustSourceItemPath {
        module: "my_shader".into(),
        name: "Uniforms".into(),
      }
    );
  }

  #[test]
  fn test_demangle_str_wesl() {
    // Sub-module item: demangled to "module::Item"
    assert_eq!(
      demangle_str("package__1global_bindings_GlobalUniforms"),
      "global_bindings::GlobalUniforms"
    );
    // Plain identifier passes through unchanged
    assert_eq!(demangle_str("Uniforms"), "Uniforms");
  }
}
