use std::borrow::Cow;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use wesl::{Resolver, ResolveError, StandardResolver};

use crate::bevy_util::parse_imports::WESL_PACKAGE_PREFIX;
use crate::bevy_util::source_file::SourceFile;
use crate::bevy_util::DependencyTree;
use crate::{
  create_rust_bindings, ShaderDefValue, SourceFilePath, SourceWithFullDependenciesResult,
  WgslBindgenError, WgslBindgenOption, WgslEntryResult, WgslShaderIrCapabilities,
};

/// Returns the regex that matches a `var<immediate> name :` declaration.
fn immediate_var_re() -> &'static Regex {
  static RE: OnceLock<Regex> = OnceLock::new();
  RE.get_or_init(|| Regex::new(r"var<immediate>\s+(\w+)\s*:").unwrap())
}

/// A WESL source resolver that replaces `var<immediate>` (naga push-constant extension)
/// with `var<private>` so the `wgsl-parse` grammar can accept the source.
///
/// After WESL compilation the caller is responsible for substituting `var<private>`
/// back to `var<immediate>` for the tracked variable names (see
/// [`collect_immediate_var_names`]).
struct ImmediateSubstResolver(StandardResolver);

impl ImmediateSubstResolver {
  fn new(base: &Path) -> Self {
    Self(StandardResolver::new(base))
  }
}

impl Resolver for ImmediateSubstResolver {
  fn resolve_source<'a>(
    &'a self,
    path: &wesl::syntax::ModulePath,
  ) -> Result<Cow<'a, str>, ResolveError> {
    let source = self.0.resolve_source(path)?;
    if !source.contains("var<immediate>") {
      return Ok(source);
    }
    Ok(Cow::Owned(source.replace("var<immediate>", "var<private>")))
  }

  fn display_name(&self, path: &wesl::syntax::ModulePath) -> Option<String> {
    self.0.display_name(path)
  }

  fn fs_path(&self, path: &wesl::syntax::ModulePath) -> Option<PathBuf> {
    self.0.fs_path(path)
  }
}

/// Scan `source` for `var<immediate>` declarations and return the variable names.
/// These are used after WESL compilation to restore `var<private>` → `var<immediate>`.
fn collect_immediate_var_names(source: &str) -> HashSet<String> {
  immediate_var_re()
    .captures_iter(source)
    .map(|cap| cap[1].to_string())
    .collect()
}

/// In `wgsl_source`, replace `var<private> name` with `var<immediate> name` for each
/// variable name that was originally `var<immediate>` (collected before WESL compilation).
fn restore_immediate_vars(mut wgsl_source: String, immediate_names: &HashSet<String>) -> String {
  for name in immediate_names {
    let from = format!("var<private> {name}");
    let to = format!("var<immediate> {name}");
    wgsl_source = wgsl_source.replace(&from, &to);
  }
  wgsl_source
}

const PKG_VER: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

pub struct WGSLBindgen {
  dependency_tree: DependencyTree,
  options: WgslBindgenOption,
  content_hash: String,
}

impl WGSLBindgen {
  pub(crate) fn new(options: WgslBindgenOption) -> Result<Self, WgslBindgenError> {
    let entry_points = options
      .entry_points
      .iter()
      .cloned()
      .map(SourceFilePath::new)
      .collect();

    let dependency_tree = DependencyTree::try_build(
      options.workspace_root.clone(),
      options.module_import_root.clone(),
      entry_points,
      options.additional_scan_dirs.clone(),
    )?;

    let content_hash = Self::get_contents_hash(&options, &dependency_tree);

    // For WESL source types, emit_rerun_if_changed is handled per-entry inside
    // generate_entry (using the WESL compiler's authoritative module list).
    // We still emit the dependency-tree files here for non-WESL-based watching.
    if options.emit_rerun_if_change {
      for file in Self::iter_files_to_watch(&dependency_tree) {
        println!("cargo::rerun-if-changed={file}");
      }
    }

    Ok(Self {
      dependency_tree,
      options,
      content_hash,
    })
  }

  fn iter_files_to_watch(dep_tree: &DependencyTree) -> impl Iterator<Item = String> {
    dep_tree
      .all_files_including_dependencies()
      .into_iter()
      .map(|path| path.to_string())
  }

  fn get_contents_hash(options: &WgslBindgenOption, dep_tree: &DependencyTree) -> String {
    let mut hasher = blake3::Hasher::new();

    hasher.update(format!("{options:?}").as_bytes());
    hasher.update(PKG_VER.as_bytes());

    for SourceFile { content, .. } in dep_tree.parsed_files() {
      hasher.update(content.as_bytes());
    }

    hasher.finalize().to_string()
  }

  /// Compile an entry point using the WESL compiler and return a `WgslEntryResult`.
  ///
  /// The WESL compiler resolves `import package::` statements and evaluates `@if`
  /// conditional translation, producing a self-contained WGSL string that is then
  /// parsed by naga to extract the IR module used for Rust binding generation.
  ///
  /// When `emit_rerun_if_change` is `true` this also emits
  /// `cargo::rerun-if-changed` for every file the WESL compiler loaded, giving
  /// accurate incremental-build tracking for WESL-syntax shaders.
  fn generate_entry<'a>(
    ir_capabilities: Option<WgslShaderIrCapabilities>,
    entry: SourceWithFullDependenciesResult<'a>,
    workspace_root: &std::path::Path,
    shader_defs: &[(String, ShaderDefValue)],
    emit_rerun_if_change: bool,
  ) -> Result<WgslEntryResult<'a>, WgslBindgenError> {
    let source = entry.source_file;
    let entry_path = source.file_path.as_path();

    // Derive the WESL module path from the entry point path relative to the workspace root.
    // e.g. `shaders/effects/glow.wgsl` with root `shaders/` → `package::effects::glow`
    let relative = entry_path
      .strip_prefix(workspace_root)
      .unwrap_or(entry_path);
    let without_ext = relative.with_extension("");
    let module_components: Vec<String> = without_ext
      .components()
      .map(|c| c.as_os_str().to_string_lossy().into_owned())
      .collect();
    let module_path_str = format!("{}{}", WESL_PACKAGE_PREFIX, module_components.join("::"));
    let module_path: wesl::syntax::ModulePath = module_path_str.parse().map_err(|e| {
      WgslBindgenError::WeslCompileError {
        entry: entry_path.display().to_string(),
        msg: format!("failed to parse WESL module path `{module_path_str}`: {e}"),
      }
    })?;

    // Set up the WESL compiler with the workspace root as the file-resolver base.
    // We use a custom `ImmediateSubstResolver` that transparently replaces
    // `var<immediate>` (a naga push-constant extension not in the WGSL/WESL spec)
    // with `var<private>` so the `wgsl-parse` grammar can parse the source.
    // After WESL compilation we restore the original address space in the output.
    //
    // We also collect `var<immediate>` variable names from the entry-point source
    // before calling the compiler, so we can do the targeted reverse substitution.
    let immediate_names: HashSet<String> = std::fs::read_to_string(entry_path)
      .map(|src| collect_immediate_var_names(&src))
      .unwrap_or_default();

    let mut compiler = wesl::Wesl::new(workspace_root)
      .set_custom_resolver(ImmediateSubstResolver::new(workspace_root));

    // Convert ShaderDefValue::Bool defs into WESL feature flags.
    // Int and UInt values are not supported by WESL conditional translation.
    for (name, def) in shader_defs {
      if let ShaderDefValue::Bool(enabled) = def {
        compiler.set_feature(name, *enabled);
      }
    }

    let compile_result = compiler.compile(&module_path).map_err(|e| {
      WgslBindgenError::WeslCompileError {
        entry: entry_path.display().to_string(),
        msg: e.to_string(),
      }
    })?;

    // Emit cargo rerun-if-changed for every file the WESL compiler loaded.
    // This is more accurate than the naga-oil dependency-tree path because it
    // captures all transitive imports directly from the compiler.
    if emit_rerun_if_change {
      wesl::emit_rerun_if_changed(&compile_result.modules, compiler.resolver());
    }

    // Restore `var<immediate>` for any push-constant variables that were temporarily
    // substituted as `var<private>` for WESL parsing compatibility.
    let wgsl_source = restore_immediate_vars(compile_result.to_string(), &immediate_names);

    // Parse the compiled WGSL with naga to obtain the IR module used for binding
    // generation (type layout, entry points, etc.).
    let mut module = naga::front::wgsl::parse_str(&wgsl_source).map_err(|e| {
      WgslBindgenError::WeslCompileError {
        entry: entry_path.display().to_string(),
        msg: format!("naga failed to parse WESL-compiled WGSL: {e}"),
      }
    })?;

    if let Some(capabilities) = ir_capabilities {
      let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        capabilities,
      );
      validator.validate(&module).map_err(|e| {
        WgslBindgenError::WeslCompileError {
          entry: entry_path.display().to_string(),
          msg: format!("naga validation failed: {e}"),
        }
      })?;
    }

    // Inject explicit @id attributes for pipeline overrides so they survive the
    // naga WGSL round-trip.  We start from the highest existing user-defined ID
    // to avoid collisions.
    let mut next_id = module
      .overrides
      .iter()
      .filter_map(|(_, o)| o.id)
      .max()
      .unwrap_or(0)
      + 1;
    for (_, o) in module.overrides.iter_mut() {
      if o.id.is_none() {
        o.id = Some(next_id);
        next_id += 1;
      }
    }

    Ok(WgslEntryResult {
      mod_name: source.file_path.module_path(workspace_root),
      naga_module: module,
      source_including_deps: entry,
      wgsl_source,
      wesl_module_path: module_path_str,
    })
  }

  pub fn header_texts(&self) -> String {
    use std::fmt::Write;
    let mut text = String::new();
    if !self.options.skip_header_comments {
      writeln!(text, "// File automatically generated by {PKG_NAME}^").unwrap();
      writeln!(text, "//").unwrap();
      writeln!(text, "// ^ {PKG_NAME} version {PKG_VER}",).unwrap();
      writeln!(text, "// Changes made to this file will not be saved.").unwrap();
      writeln!(text, "// SourceHash: {}", self.content_hash).unwrap();
      writeln!(text).unwrap();
    }
    text
  }

  fn generate_output(&self) -> Result<String, WgslBindgenError> {
    let ir_capabilities = self.options.ir_capabilities;
    let emit_rerun_if_change = self.options.emit_rerun_if_change;

    let entry_results = self
      .dependency_tree
      .get_source_files_with_full_dependencies()
      .into_iter()
      .map(|it| {
        Self::generate_entry(
          ir_capabilities,
          it,
          &self.options.workspace_root,
          &self.options.shader_defs,
          emit_rerun_if_change,
        )
      })
      .collect::<Result<Vec<_>, _>>()?;

    Ok(create_rust_bindings(entry_results, &self.options)?)
  }

  pub fn generate_string(&self) -> Result<String, WgslBindgenError> {
    let mut text = self.header_texts();
    text += &self.generate_output()?;
    Ok(text)
  }

  pub fn generate(&self) -> Result<(), WgslBindgenError> {
    let out = self
      .options
      .output
      .as_ref()
      .ok_or(WgslBindgenError::OutputFileNotSpecified)?;

    let old_content = std::fs::read_to_string(out).unwrap_or_else(|_| String::new());

    let old_hashstr_comment = old_content
      .lines()
      .find(|line| line.starts_with("// SourceHash:"))
      .unwrap_or("");

    let is_hash_changed =
      || old_hashstr_comment != format!("// SourceHash: {}", &self.content_hash);

    if self.options.skip_hash_check || is_hash_changed() {
      let content = self.generate_string()?;

      // Create parent directories if they don't exist
      if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
      }

      std::fs::File::create(out)?.write_all(content.as_bytes())?
    }

    Ok(())
  }
}
