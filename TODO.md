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

## WESL tracking

WESL ([spec](https://github.com/wgsl-tooling-wg/wesl-spec)) is a community-driven superset
of WGSL that standardises the import and conditional-compilation features currently
provided by naga-oil's custom preprocessor (`#import`, `#ifdef`, etc.).

Initial WESL support has been added via `WgslShaderSourceType::EmbedWithWesl` (enabled
with the `wesl` crate feature).  At this time naga-oil remains the default because:

* The WESL `to_naga` direct-IR path is still marked unfinished upstream.
* WESL requires a different import syntax (`import package::module;`) so existing
  naga-oil shaders cannot be used with `EmbedWithWesl` without migration.
* WESL conditional compilation only supports boolean feature flags, while naga-oil also
  supports integer and unsigned-integer defines.

Areas to watch as WESL matures:
- Direct naga IR output from WESL (tracked in the `wesl` crate's `to_naga.rs`).
- A migration path / compatibility layer for naga-oil `#import` syntax.
- Support for integer/float shader defines in WESL conditional translation.
