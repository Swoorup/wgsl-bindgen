# wgsl-bindgen
[![Latest Version](https://img.shields.io/crates/v/wgsl_bindgen.svg)](https://crates.io/crates/wgsl_bindgen) [![docs.rs](https://docs.rs/wgsl_bindgen/badge.svg)](https://docs.rs/wgsl_bindgen)  
An experimental library for generating typesafe Rust bindings from [WGSL](https://www.w3.org/TR/WGSL/) shaders to [wgpu](https://github.com/gfx-rs/wgpu).

wgsl_bindgen, powered by [naga-oil](https://github.com/bevyengine/naga_oil), is a tool that integrates into your Rust build process. It parses WGSL shaders and generates corresponding Rust modules. These modules contain type definitions and boilerplate code that match your shaders, reducing the risk of runtime errors by catching mismatches at compile time.

The tool facilitates a shader-focused workflow. When you modify your WGSL shaders, the changes are automatically reflected in the Rust code. This immediate feedback helps catch errors early, making it easier to work with shaders.

## Features
- Supports import syntax and many more features from naga oil flavour.
- More strongly typed [bind group and bindings](#bind-groups) initialization
- Shader module initialization
- Rust structs for vertex, storage, and uniform buffers
- Conditionally derives for encase, bytemuck, and optionally serde
- Const validation of [WGSL memory layout](#memory-layout) for provided vector and matrix types and generated structs when using bytemuck

## Usage
When enabling derives for crates like bytemuck, serde, or encase, these dependencies should also be added to the `Cargo.toml` with the appropriate derive features. See the provided [example project](https://github.com/Swoorup/wgsl-bindgen/tree/main/example) for basic usage.

```toml
[dependencies]
bytemuck = "..."

[build-dependencies]
wgsl_bindgen = "..."
```

Then, in your build.rs:

```rust
use wgsl_bindgen::WgslBindgenOptionBuilder;

fn main() {
  let options = WgslBindgenOptionBuilder::default()
    .add_entry_point("src/pbr.wgsl")
    .add_entry_point("src/pfx.wgsl")
    .output_file("src/shader.rs")
    .build()
    .unwrap();

  let bindgen = options.build().unwrap();
  bindgen.generate().unwrap();
}
```

This will generate Rust bindings for the WGSL shader at `src/pbr.wgsl`, `src/pfx.wgsl` and write them to `src/shader.rs`.
See the example crate for how to use the generated code. Run the example with `cargo run`.

## Wgsl Import Resolution

wgsl_bindgen uses a specific strategy to resolve the import paths in your WGSL source code. This process is handled by the [ModulePathResolver::generate_possible_paths](https://github.com/Swoorup/wgsl-bindgen/blob/3e581089e21b245bd85feecdc94f3f1d9310aacc/wgsl_bindgen/src/bevy_util/module_path_resolver.rs#L32) function.

Consider the following directory structure:

```
/my_project
├── src
│   ├── shaders
│   │   ├── main.wgsl
│   │   ├── utils
│   │   │   ├── math.wgsl
│   ├── main.rs
├── Cargo.toml
```

And the following import statement in main.wgsl:

```
import utils::math;
```

Here's how wgsl_bindgen resolves the import path:

1. The function first checks if the import module name (`utils::math`) starts with the module prefix. If a module prefix is set and matches, it removes the prefix and treats the rest of the import module name as a relative path from the entry source directory converting the double semicolor `::` to forward slash `/` from the directory of the current source file (`src/shaders`).
2. If the import module name does not start with the module prefix, it treats the entire import module name as a relative path from the directory of the current source file. In this case, it will look for `utils/math.wgsl` in the same directory as `main.wgsl`.
3. The function then returns a set of possible import paths. The actual file that the import statement refers to is the first file in this set that exists. In this case, it would successfully find and import `src/shaders/utils/math.wgsl`. 
2. If not, the second possible path it would have tried would be `src/shaders/utils.wgsl` treating `math` as an item within `utils.wgsl` had it existed.

This strategy allows `wgsl_bindgen` to handle a variety of import statement formats and directory structures, providing flexibility in how you organize your WGSL source files.

## Memory Layout
WGSL structs have different memory layout requirements than Rust structs or standard layout algorithms like `repr(C)` or `repr(packed)`. Matching the expected layout to share data between the CPU and GPU can be tedious and error prone. wgsl_bindgen offers options to add derives for [encase](https://crates.io/crates/encase) to handle padding and alignment at runtime or [bytemuck](https://crates.io/crates/bytemuck) for enforcing padding and alignment at compile time. 

When deriving bytemuck, wgsl_bindgen will use naga's layout calculations to add const assertions to ensure that all fields of host-shareable types (structs for uniform and storage buffers) have the correct offset, size, and alignment expected by WGSL. 

## Bind Groups
wgpu uses resource bindings organized into bind groups to define global shader resources like textures and buffers. Shaders can have many resource bindings organized into up to 4 bind groups. wgsl_bindgen will generate types and functions for initializing and setting these bind groups in a more typesafe way. Adding, removing, or changing bind groups in the WGSl shader will typically result in a compile error instead of a runtime error when compiling the code without updating the code for creating or using these bind groups.

While bind groups can easily be set all at once using the `bind_groups::set_bind_groups` function, it's recommended to organize bindings into bindgroups based on their update frequency. Bind group 0 will change the least frequently like per frame resources with bind group 3 changing most frequently like per draw resources. Bind groups can be set individually using their `set(render_pass)` method. This can provide a small performance improvement for scenes with many draw calls. See [descriptor table frequency (DX12)](https://learn.microsoft.com/en-us/windows/win32/direct3d12/advanced-use-of-descriptor-tables#changing-descriptor-table-entries-between-rendering-calls) and [descriptor set frequency (Vulkan)](https://vkguide.dev/docs/chapter-4/descriptors/#mental-model) for details.

Organizing bind groups in this way can also help to better organize rendering resources in application code instead of redundantly storing all resources with each object. The `bindgroups::BindGroup0` may only need to be stored once while `bindgroups::BindGroup3` may be stored for each mesh in the scene. Note that bind groups store references to their underlying resource bindings, so it is not necessary to recreate a bind group if the only the uniform or storage buffer contents change. Avoid creating new bind groups during rendering if possible for best performance.

## Limitations
- It may be necessary to disable running this function for shaders with unsupported types or features.
Please make an issue if any new or existing WGSL syntax is unsupported.
- This library is not a rendering library and will not generate any high level abstractions like a material or scene graph. 
The goal is just to generate most of the tedious and error prone boilerplate required to use WGSL shaders with wgpu.
- The generated code will not prevent accidentally calling a function from an unrelated generated module.
- Most but not all WGSL types are currently supported.
- Vertex attributes using floating point types in WGSL like `vec2<f32>` are assumed to use float inputs instead of normalized attributes like unorm or snorm integers.
- All textures are assumed to be filterable and all samplers are assumed to be filtering. This may lead to compatibility issues. This can usually be resolved by requesting the native only feature TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES.
- It's possible to achieve slightly better performance than the generated code in some cases like avoiding redundant bind group bindings or adjusting resource shader stage visibility. This should be addressed by using some handwritten code where appropriate.

## Differences from the [fork](https://github.com/ScanMountGoat/wgsl_to_wgpu/) 
- Supports WGSL import syntax and many more features from naga oil flavour.
- You can only choose either bytemuck or encase for serialization
- Bytemuck mode supports Runtime-Sized-Array as generic const array in rust. 
- Bytemuck mode automatically adds padding for mat3x3, vec3, whereas original would fail at compile assertions.
- User can provide their own wgsl type mappings using `quote` library
- Expect breaking changes

## Publishing Crates
The provided example project outputs the generated bindings to the `src/` directory for documentation purposes. 
This approach is also fine for applications. Published crates should follow the recommendations for build scripts in the [Cargo Book](https://doc.rust-lang.org/cargo/reference/build-scripts.html#case-study-code-generation).

```rust
use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{WgslTypeSerializeStrategy, WgslBindgenOptionBuilder, GlamWgslTypeMap};

// src/build.rs
fn main() -> Result<()> {
    WgslBindgenOptionBuilder::default()
        .add_entry_point("src/shader/testbed.wgsl")
        .add_entry_point("src/shader/triangle.wgsl")
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .wgsl_type_map(GlamWgslTypeMap)
        .derive_serde(false)
        .output_file("src/shader.rs")
        .build()?
        .generate()
        .into_diagnostic()
}
```

The generated code will need to be included in one of the normal source files. This includes adding any nested modules as needed.

```rust
// src/lib.rs
mod shader;
```
