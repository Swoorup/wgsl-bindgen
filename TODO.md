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

## WESL — naga_oil replacement (completed)

WESL ([spec](https://github.com/wgsl-tooling-wg/wesl-spec)) is a community-driven superset
of WGSL that standardises the import and conditional-compilation features previously
provided by naga-oil's custom preprocessor (`#import`, `#ifdef`, etc.).

### What has been done

* **naga-oil removed** — `wgsl_bindgen` no longer depends on `naga_oil`. All shader
  compilation at build time is now done by the WESL compiler.
* `WgslShaderSourceType` now has two variants:
  - `EmbedSource` — compiles WESL shaders at build time using the WESL compiler and
    embeds the resulting WGSL; no WESL or naga-oil dependency needed at runtime.
  - `WeslWithRelativePath` — generates a `create_shader_module_from_path(device, base_dir)`
    function that uses the WESL compiler **at runtime** to load and compile shaders from
    disk; requires the `wesl` crate (with `naga-ext`) as a runtime dependency.  This is
    the WESL-native equivalent of the old `ComposerWithRelativePath`.
* `EmbedWithNagaOilComposer` removed (was an embed variant that required naga-oil).
* `EmbedWithWesl` removed (merged into `EmbedSource`).
* WESL `import package::module::item;` syntax is understood by the dependency-tree
  builder (`parse_imports.rs`), so `cargo::rerun-if-changed` is correctly emitted for
  all transitive WESL imports during hot-reload workflows.
* The WESL backend uses the WESL compiler's own module list to emit
  `cargo::rerun-if-changed` for all files it loaded (including transitive imports).
* `ShaderDefValue` is defined by wgsl_bindgen itself (not re-exported from naga-oil).

### Migration guide (naga-oil → WESL)

| Old (naga-oil) | New (WESL) |
|---|---|
| `#import module::item` | `import package::module::item;` |
| `#ifdef FEATURE` / `#endif` | `@if(FEATURE)` on the item |
| `EmbedWithNagaOilComposer` | `EmbedSource` |
| `ComposerWithRelativePath` | `WeslWithRelativePath` (runtime) or `EmbedSource` (compile-time) |
| `EmbedWithWesl` | `EmbedSource` |

> **Note:** WESL conditional compilation only supports boolean feature flags.
> Integer/unsigned-integer shader defs are accepted by the API but silently ignored.

### Remaining / future work

- Track WESL's direct naga IR output path to remove the naga WGSL round-trip in
  the WESL compilation step.
