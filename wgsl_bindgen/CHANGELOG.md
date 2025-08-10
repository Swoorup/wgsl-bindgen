# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.21.1](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.21.0...wgsl_bindgen-v0.21.1) - 2025-08-10

### Fixed

- fixed snapshots
- fixed texture_2d_array, added basic usage in demo
- fixed test support

### Other

- texture support

### Removed

- removed compile error in favour of panic
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.21.0](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.20.1...wgsl_bindgen-v0.21.0) - 2025-07-07

### Added

- create parent directories for output file

### Other

- Optimize running integration compilation tests to use the same target directory
- Implement shader_defs with shared bind group improvements
- Reorganize integration tests into logical modules for better maintainability
- Clean up example, buffer_binding_fix

## [0.20.1](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.20.0...wgsl_bindgen-v0.20.1) - 2025-07-06

### Added

- Add Init struct generation and improve field documentation

### Fixed

- Generate correct module paths and ShaderEntry variants for nested shader files

### Other

- Added notes to wgsl binding calculation

## [0.20.0](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.19.1...wgsl_bindgen-v0.20.0) - 2025-07-03

### Added

- Dynamic shader loading with `ComposerWithRelativePath` ([#45](https://github.com/Swoorup/wgsl-bindgen/issues/45))
- Support for rust compilation tests to catch regressions ([#70](https://github.com/Swoorup/wgsl-bindgen/issues/70))
- Helper function `visit_shader_files` for traversing shader dependency trees
- Helper function `load_naga_module_from_path` for loading modules with relative paths

### Fixed

- **BREAKING**: Fixed duplicate content error when importing same vertex input across multiple shader files ([#74](https://github.com/Swoorup/wgsl-bindgen/issues/74))
- **BREAKING**: Replaced `glam::Vec3A` with `glam::Vec3` and corrected padding calculations to fix compilation errors ([#47](https://github.com/Swoorup/wgsl-bindgen/issues/47))
- Fixed shader stage visibility for shared bindings across multiple shader types ([#27](https://github.com/Swoorup/wgsl-bindgen/issues/27))
- Fixed path handling on Windows for full path imports
- Fixed vertex buffer layout generation for builtin fields

### Changed

- **BREAKING**: Removed `HardCodedFilePathWithNagaOilComposer` in favor of relative path composer
- Improved test coverage with compilation validation
- Updated example to demonstrate dynamic shader loading

## [0.19.1](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.19.0...wgsl_bindgen-v0.19.1) - 2025-06-28

### Fixed

- improve vertex input handling with builtin field support

## [0.19.0](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.18.2...wgsl_bindgen-v0.19.0) - 2025-06-27

### Other

- Fix cargo clippy failures blocking CI release
- Added support for BufferArray, SamplerArray, TextureArray bindings
- Added more testing for f16
- Add support for Acceleration structure.
- Replace inline rust source testing with insta snapshot
- Update to use insta for snapshot testing instead.
- Update to use wgpu 25

## [0.18.2](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.18.1...wgsl_bindgen-v0.18.2) - 2025-05-28

### Other

- Use the fully qualified path for core::mem::size_of
- Added an example to patch bindgroup entry module path.

## [0.18.1](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.18.0...wgsl_bindgen-v0.18.1) - 2025-03-21

### Other

- Add support for overriding bind group entry module paths
- Fixed WgpuBindGroups generation in shader entries

## [0.18.0](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.17.0...wgsl_bindgen-v0.18.0) - 2025-03-20

### Other

- Added support for reusable bind groups. Bindgroups defined in shared wgsl, are generated once on the shared module and not on entrypoint

## [0.15.2](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.15.1...wgsl_bindgen-v0.15.2) - 2024-12-13

### Other

- Fix compute shader generation for wgpu 23
- relaxed lifetime of &self in BindGroup::set
- updated to wgpu 23

## [0.15.1](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.15.0...wgsl_bindgen-v0.15.1) - 2024-08-20

### Other
- Fix missing cache property

## [0.15.0](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.14.1...wgsl_bindgen-v0.15.0) - 2024-08-10

### Other
- Support push constants output in naga_oil composition strategy
- Upgrade to wgpu 22
- Rename `RustItemKind` => `RustItemType`
- Treat built-in fields as padding [#34](https://github.com/Swoorup/wgsl-bindgen/pull/34)

## [0.14.1](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.14.0...wgsl_bindgen-v0.14.1) - 2024-07-27

### Other
- Fix vertex type in module [#35](https://github.com/Swoorup/wgsl-bindgen/pull/35)
- Update README.md

## [0.14.0](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.13.2...wgsl_bindgen-v0.14.0) - 2024-07-06

### Other
- Remove unnecessary bindgroups module
- Rename EntryCollection to Entries and cleanup examples

## [0.13.2](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.13.1...wgsl_bindgen-v0.13.2) - 2024-07-05

### Other
- Bind Group Entry Collections can be cloned

## [0.13.1](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.13.0...wgsl_bindgen-v0.13.1) - 2024-07-05

### Other
- Fix `min_binding_size` when invoking entry module contains symbols.

## [0.13.0](https://github.com/Swoorup/wgsl-bindgen/compare/wgsl_bindgen-v0.12.0...wgsl_bindgen-v0.13.0) - 2024-07-05

### Other
- Added release-plz workflow
- Reference version from workspace root
- For bindgroup generation, rename Layout to EntryCollection with helper structs
- Added `min_binding_size` for buffer types where possible
- Allow to fully qualify or use relative name
- Added a way to skip generating `_root_`

## v0.12.0 (2024-06-10)

<csr-id-e52a9dbe660a417afa371f480be161d58f1dd642/>

### Other

 - <csr-id-e52a9dbe660a417afa371f480be161d58f1dd642/> format builder error message into bindgen error

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 51 commits contributed to the release over the course of 125 calendar days.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Added changelog ([`cd55d10`](https://github.com/Swoorup/wgsl-bindgen/commit/cd55d10c57f1e159a0c31988c67559b559a68ace))
    - Release wgsl_bindgen v0.12.0 ([`d61fd9e`](https://github.com/Swoorup/wgsl-bindgen/commit/d61fd9e174877500ba86d089101ecba7c1b5886f))
    - Fix typo ([`22adeec`](https://github.com/Swoorup/wgsl-bindgen/commit/22adeece762ad8835a812fc448a3281ae6ce42f9))
    - Added non-working support for overridable constants ([`e1937d6`](https://github.com/Swoorup/wgsl-bindgen/commit/e1937d661f920812e3587d2cb70362cad15a613f))
    - Initial upgrade to wgpu 0.20 ([`92bf827`](https://github.com/Swoorup/wgsl-bindgen/commit/92bf8274c3bdc39e4332f558a653647be61c3d95))
    - Make the texture sample type filterable ([`0660ee1`](https://github.com/Swoorup/wgsl-bindgen/commit/0660ee19a21e65f6da14835fd9cd85924ae762b1))
    - Consolidate specifying versions in the root manifest ([`42d2822`](https://github.com/Swoorup/wgsl-bindgen/commit/42d2822da5a85e1964b4442db090a6991a5b30c3))
    - Added option to change the visibily of the export types ([`88fd877`](https://github.com/Swoorup/wgsl-bindgen/commit/88fd877fc2c75c35dee3d313d93d93e22ffcb75b))
    - Fix issues with texture_2d of type i32 or u32 ([`53c0c63`](https://github.com/Swoorup/wgsl-bindgen/commit/53c0c63f6e4ea2a2569182bea2e99874ca64461e))
    - Use the renamed crate include_absolute_path ([`6f485bf`](https://github.com/Swoorup/wgsl-bindgen/commit/6f485bf0beb05992d8d2a2ee1950738fd2e434fe))
    - Make SHADER_STRING public ([`ce4f68b`](https://github.com/Swoorup/wgsl-bindgen/commit/ce4f68b418241c3224240bab42e9cbe0bae52905))
    - Regex for all overrides ([`8ea7ffd`](https://github.com/Swoorup/wgsl-bindgen/commit/8ea7ffd65871af95aaeaff8da9d4589f20ff049c))
    - Simplify also for bulk options ([`d45d6f0`](https://github.com/Swoorup/wgsl-bindgen/commit/d45d6f0898c52fa7f8ad41abb7f466e6ae2aec25))
    - Adding custom padding field support ([`998f7a8`](https://github.com/Swoorup/wgsl-bindgen/commit/998f7a8f60b83424fff93e471f04adf7130a8f83))
    - Adjust size if custom alignment is specified. ([`a4b61c7`](https://github.com/Swoorup/wgsl-bindgen/commit/a4b61c7d52496499b92b029a3604053d2420b147))
    - Ability to override alignment for structs ([`cd26b91`](https://github.com/Swoorup/wgsl-bindgen/commit/cd26b91be29870ac629a1674a8a43ba98d46b6d6))
    - Use Result type for create_shader* when using `NagaOilComposer` ([`80a7f95`](https://github.com/Swoorup/wgsl-bindgen/commit/80a7f9594330b6e982bb91bb12991df8b79cba70))
    - Seperate types, assertions, impls in generated output ([`c2c4dc9`](https://github.com/Swoorup/wgsl-bindgen/commit/c2c4dc956925aedef11d706cd7024c8b25593a66))
    - RustSourceItem => RustItem ([`ce2a91e`](https://github.com/Swoorup/wgsl-bindgen/commit/ce2a91eca61507ba237fd9828a84a5d00a6e2d99))
    - Pass entry point name to builders ([`4fc895b`](https://github.com/Swoorup/wgsl-bindgen/commit/4fc895bef6ce8a29b32611fc363ea68a40b60405))
    - Export quote, syn functions and macros ([`782f481`](https://github.com/Swoorup/wgsl-bindgen/commit/782f481c70bb5d8ae8381c0ddf83ec4ddc6a2a79))
    - Added extra bindings generator as prep for targetting non-wgpu libs ([`9b6204d`](https://github.com/Swoorup/wgsl-bindgen/commit/9b6204d62b4daa5f45c7d9a0ee05d41380f37650))
    - Added custom field mappings ([`4132659`](https://github.com/Swoorup/wgsl-bindgen/commit/4132659692ea4a34a7cf510829a470dc3390b269))
    - Avoid HashMap for more consitent shader bindings generation ([`fd6d144`](https://github.com/Swoorup/wgsl-bindgen/commit/fd6d144dafbcc6e234d479f5c7e5c53c93f0816c))
    - Rename ShaderRegistry to ShaderEntry in output ([`1461393`](https://github.com/Swoorup/wgsl-bindgen/commit/1461393b0710e23a028478f1df131191f2398c2e))
    - Added mandatory workspace root option used for resolving imports ([`d20d3d5`](https://github.com/Swoorup/wgsl-bindgen/commit/d20d3d5176984f305d4a3e190500c4601671af85))
    - Add shader labels ([`c8a129b`](https://github.com/Swoorup/wgsl-bindgen/commit/c8a129bc5529a468eb29687b20ce4c40e6fa647f))
    - Feature shader registry and shader defines ([`187c7f4`](https://github.com/Swoorup/wgsl-bindgen/commit/187c7f417ec9be4543168c462ed6d171ba3180c6))
    - Added multiple shader source option ([`db90739`](https://github.com/Swoorup/wgsl-bindgen/commit/db90739cec926b464eb6fafb8f1254c42ad91201))
    - Add ability to override struct and path based source type ([`1d4ee0a`](https://github.com/Swoorup/wgsl-bindgen/commit/1d4ee0a552ffe4e6a9298f183bd3c9b617635908))
    - Short const constructors and fix demangle in comments ([`a49be89`](https://github.com/Swoorup/wgsl-bindgen/commit/a49be89ca98ca65ca296717b0f98e24530ad11b0))
    - Rename Capabilities to WgslShaderIRCapabilities, and update test ([`1cad0cb`](https://github.com/Swoorup/wgsl-bindgen/commit/1cad0cbe5ff581810b770c6fb95940f1472c7fd1))
    - Reexport Capabilities ([`7262606`](https://github.com/Swoorup/wgsl-bindgen/commit/7262606a6d0880c9f8aa8872197a3e151a16975b))
    - Allow setting capabilities ([`b6df117`](https://github.com/Swoorup/wgsl-bindgen/commit/b6df117b40909cfeb803c6a7782ab2d2dc906176))
    - Release new version ([`ec3d554`](https://github.com/Swoorup/wgsl-bindgen/commit/ec3d55412002d27c48200261b8e9853e9bfe8af2))
    - Make naga oil's error more useful ([`6a1bc45`](https://github.com/Swoorup/wgsl-bindgen/commit/6a1bc45524ffeb4386ff18f846588cf6c1ea0e1b))
    - Format builder error message into bindgen error ([`e52a9db`](https://github.com/Swoorup/wgsl-bindgen/commit/e52a9dbe660a417afa371f480be161d58f1dd642))
    - Ignore snake case warnings if struct is not camel case ([`54c563e`](https://github.com/Swoorup/wgsl-bindgen/commit/54c563eb3d89d9815d7391b599c1a86de3a14d25))
    - Minor corrections ([`194b3e4`](https://github.com/Swoorup/wgsl-bindgen/commit/194b3e4a66bfaad0ebc577670b50eec372701e35))
    - Added a mechanism to scan additional source directory ([`300a3d7`](https://github.com/Swoorup/wgsl-bindgen/commit/300a3d7aec20556712bd835d71a42ca375ae1da9))
    - Allow to use naga_oil compose in the generated output ([`f32c279`](https://github.com/Swoorup/wgsl-bindgen/commit/f32c279c02ea7760ce901533013f6d0da51674c5))
    - Fix direct item wgsl imports. ([`3e58108`](https://github.com/Swoorup/wgsl-bindgen/commit/3e581089e21b245bd85feecdc94f3f1d9310aacc))
    - Added failing test for direct path import for nested type ([`e014d4b`](https://github.com/Swoorup/wgsl-bindgen/commit/e014d4b6c5326a40d59291be96e24a3fd150d746))
    - Demangle bindgroup struct fields if imported from other wgsl files ([`7231f78`](https://github.com/Swoorup/wgsl-bindgen/commit/7231f78806e75a18af9f78005c3b016f16dcf1dc))
    - Add support for scalar types in bindings ([`4af047a`](https://github.com/Swoorup/wgsl-bindgen/commit/4af047aa976252211f31f882db8b5006fecb1977))
    - Add support for path based import. ([`d1e861d`](https://github.com/Swoorup/wgsl-bindgen/commit/d1e861dacd5cb04f1b74065448fde980cfc696b6))
    - Demangle name for consts items ([`5ec2c1a`](https://github.com/Swoorup/wgsl-bindgen/commit/5ec2c1a22c2b4c1855dee3d2d88fa0b46ad88d6c))
    - Updated docs, use stable features only ([`06401c5`](https://github.com/Swoorup/wgsl-bindgen/commit/06401c5eb0c5d867bee4aedf4b339f9cd373f9a5))
    - Support naga oil flavour of wgsl ([`99ea17c`](https://github.com/Swoorup/wgsl-bindgen/commit/99ea17c17bf682dd1ed9990341fb1a3aa119a6f6))
    - Enable Runtime Sized Array, Padding for bytemuck mode ([`9e21d1d`](https://github.com/Swoorup/wgsl-bindgen/commit/9e21d1dbe084f1588d7e03e2c93642ca3ffb2c05))
    - Create a fork ([`1c99e10`](https://github.com/Swoorup/wgsl-bindgen/commit/1c99e103625154dde0e357419f064e941e156f54))
</details>

