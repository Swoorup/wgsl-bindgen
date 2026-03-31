# Rough ideas. 

* Allow injecting dynamic shader defines at build-time (we already have a runtime mechanism from generation)

* Allow generation directly from shader strings.

* proc_macro as an option alongside of build.rs.

* Use struct like this instead directly using the array.
  * ```rust
    #[repr(C)]
    struct PaddedField<const N: usize, T> {
        field: T,
        padding: [u8; N],
    }

    impl<const N: usize, T> PaddedField<N, T> {
      pub fn new(value: T) -> Self {
        Self {
          field: value,
          padding: [0; N],
        }
      }
    }
    ```

  - https://github.com/rust-lang/rust/issues/73557
  - https://www.reddit.com/r/rust/comments/16e18kp/how_to_set_alignment_of_individual_struct_members/

* Add a way to encode variant types in wgsl?. 
  * Maybe a seperate binary that accepts rust source. 
  * Generates accessors, setters in wgsl
  * Struct fields are efficiently utilised.

## WESL tracking (replacing naga_oil with WESL)

WESL ([spec](https://github.com/wgsl-tooling-wg/wesl-spec)) is a community-driven superset
of WGSL that standardises the import and conditional-compilation features currently
provided by naga-oil's custom preprocessor (`#import`, `#ifdef`, etc.).

### What has been done

* `WgslShaderSourceType::EmbedWithWesl` added (enabled with the `wesl` crate feature):
  compiles WESL shaders at build time and embeds the resulting WGSL — no WESL or
  naga-oil dependency needed at runtime.
* WESL `import package::module::item;` syntax is now understood by the dependency-tree
  builder (`parse_imports.rs`), so `cargo::rerun-if-changed` is correctly emitted for
  all transitive WESL imports during hot-reload workflows.
* The WESL backend uses the WESL compiler's own module list to emit
  `cargo::rerun-if-changed` for all files it loaded (including transitive imports), 
  independent of the naga-oil based dependency tree.
* `ShaderDefValue` is now defined by wgsl_bindgen itself (not re-exported from
  naga-oil), so `build.rs` files that only use `EmbedWithWesl` no longer need naga-oil
  as a direct build dependency.

### Why naga-oil remains the default

* WESL requires different import syntax (`import package::module;`), so existing
  naga-oil shaders need manual migration.
* `EmbedWithNagaOilComposer` and `ComposerWithRelativePath` provide runtime shader
  composition with `#ifdef`-style defines and live file loading — WESL currently has no
  equivalent runtime path (it is a build-time compiler only).
* WESL conditional compilation only supports boolean feature flags; naga-oil supports
  integer and unsigned-integer defines as well.

### Remaining steps toward full naga-oil replacement

- Make naga-oil an **optional** feature flag (gated behind `cfg(feature = "naga-oil")`).
  The main blocker is `parse_imports.rs`'s use of
  `naga_oil::compose::parse_imports::parse_imports` for `#import` syntax.  A custom
  `#import` parser (or dropping `#import` support in favour of WESL syntax) would unblock
  this.
- Add a WESL-based runtime shader loading path (equivalent to
  `ComposerWithRelativePath` but for WESL import syntax).
- Track WESL's direct naga IR output path (`to_naga.rs` upstream) to remove the
  naga WGSL-roundtrip in `EmbedWithWesl`.
- Document the migration guide from naga-oil `#import` / `#ifdef` to WESL
  `import` / `@if`.
