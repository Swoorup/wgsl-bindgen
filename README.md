# wgsl-bindgen

[![Latest Version](https://img.shields.io/crates/v/wgsl_bindgen.svg)](https://crates.io/crates/wgsl_bindgen) [![docs.rs](https://docs.rs/wgsl_bindgen/badge.svg)](https://docs.rs/wgsl_bindgen) ![License](https://img.shields.io/crates/l/wgsl_bindgen) ![Rust Version](https://img.shields.io/badge/rust-1.70+-blue)

🚀 **Generate typesafe Rust bindings from [WGSL](https://www.w3.org/TR/WGSL/) shaders for [wgpu](https://github.com/gfx-rs/wgpu)** 

wgsl_bindgen transforms your WGSL shader development workflow by automatically generating Rust types, constants, and boilerplate code that perfectly match your shaders. Powered by [naga-oil](https://github.com/bevyengine/naga_oil), it integrates seamlessly into your build process to catch shader-related errors at compile time rather than runtime.

## 🎯 Why wgsl_bindgen?

**Before**: Manual, error-prone shader bindings
```rust
// ❌ Easy to make mistakes - no compile-time verification
let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    entries: &[
        wgpu::BindGroupEntry {
            binding: 0, // Is this the right binding index?
            resource: texture_view.as_binding(), // Is this the right type?
        },
        wgpu::BindGroupEntry {
            binding: 1, // What if you change the shader?
            resource: sampler.as_binding(),
        },
    ],
    // ... more boilerplate
});
```

**After**: Typesafe, auto-generated bindings
```rust
// ✅ Compile-time safety - generated from your actual shaders
let bind_group = my_shader::WgpuBindGroup0::from_bindings(
    device,
    my_shader::WgpuBindGroup0Entries::new(my_shader::WgpuBindGroup0EntriesParams {
        my_texture: &texture_view,  // Type-checked parameter names
        my_sampler: &sampler,       // Matches your WGSL exactly
    })
);
bind_group.set(&mut render_pass); // Simple, safe usage
```

## ✨ Key Benefits

- 🛡️ **Type Safety**: Catch shader binding mismatches at compile time
- 🔄 **Automatic Sync**: Changes to WGSL automatically update Rust bindings  
- 📝 **Reduced Boilerplate**: Generate tedious wgpu setup code automatically
- 🎮 **Shader-First Workflow**: Design in WGSL, get Rust bindings for free
- 🔧 **Flexible**: Works with bytemuck, encase, serde, and custom types
- ⚡ **Fast**: Build-time generation with intelligent caching

## Features

### General:

-   Generates either new or enum-like short constructors to ease creating the generated types, especially ones that require to be padded when using with bytemuck.
-   More strongly typed [bind group and bindings](#bind-groups) initialization
-   Generate your own binding entries for non-wgpu types. This is a work in progress feature to target other non-wgpu frameworks.

### Shader Handling:

-   Supports import syntax and many more features from naga oil flavour.
-   Add shader defines dynamically when using either `WgslShaderSourceType::EmbedWithNagaOilComposer`, `WgslShaderSourceType::HardCodedFilePathWithNagaOilComposer`, or `WgslShaderSourceType::ComposerWithRelativePath` source output type.

    The `WgslShaderSourceType::HardCodedFilePathWithNagaOilComposer` could be used for hot reloading.
    
    The `WgslShaderSourceType::ComposerWithRelativePath` provides full control over file I/O without requiring nightly Rust, making it ideal for integration with custom asset systems.

-   Shader registry utility to dynamically call `create_shader` variants depending on the variant. This is useful when trying to keep cache of entry to shader modules. Also remember to add shader defines to accomodate for different permutation of the shader modules.
-   Ability to add additional scan directories for shader imports when defining the workflow.

### Type Handling:

-   BYO - **B**ring **Y**our **O**wn **T**ypes for Wgsl matrix, vector types. Bindgen will automatically include assertions to test alignment and sizes for your types at compile time.
-   Override generated struct types either entirely or just particular field of struct from your crate, which is handy for small primitive types. You can also use this to overcome the limitation of uniform buffer type restrictions in wgsl.
-   Rust structs for vertex, storage, and uniform buffers.
-   Either use encase or bytemuck derives, and optionally serde for generated structs.
-   Const validation of [WGSL memory layout](#memory-layout) for provided vector and matrix types and generated structs when using bytemuck
-   Override the alignment for the struct generated. This also affects the size of the struct generated.

## 🚀 Quick Start

### 1. Add to your `Cargo.toml`

```toml
[build-dependencies]
wgsl_bindgen = "0.19"

[dependencies]
wgpu = "25"
bytemuck = { version = "1.0", features = ["derive"] }
# Optional: for additional features
# encase = "0.8"
# serde = { version = "1.0", features = ["derive"] }

# Note: When using ComposerWithRelativePath, enable naga-ir feature for optimal performance:
# wgpu = { version = "25", features = ["naga-ir"] }
```

### 2. Create your WGSL shader (`shaders/my_shader.wgsl`)

```wgsl
struct Uniforms {
    transform: mat4x4<f32>,
    time: f32,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var my_texture: texture_2d<f32>;
@group(0) @binding(2) var my_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = uniforms.transform * vec4<f32>(input.position, 1.0);
    output.uv = input.uv;
    return output;
}

@fragment  
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(my_texture, my_sampler, input.uv);
}
```

### 3. Set up build script (`build.rs`)

```rust
use wgsl_bindgen::{WgslBindgenOptionBuilder, WgslTypeSerializeStrategy, GlamWgslTypeMap};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    WgslBindgenOptionBuilder::default()
        .workspace_root("shaders")
        .add_entry_point("shaders/my_shader.wgsl")
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .type_map(GlamWgslTypeMap) // Use glam for math types
        .output("src/shader_bindings.rs")
        .build()?
        .generate()?;
    Ok(())
}
```

### 4. Use the generated bindings

```rust
// Include the generated bindings
mod shader_bindings;
use shader_bindings::my_shader;

fn setup_render_pipeline(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    // Create shader module from generated code
    let shader = my_shader::create_shader_module_embed_source(device);
    
    // Use generated pipeline layout
    let pipeline_layout = my_shader::create_pipeline_layout(device);
    
    // Use generated vertex entry with proper buffer layout
    let vertex_entry = my_shader::vs_main_entry(wgpu::VertexStepMode::Vertex);
    
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(&pipeline_layout),
        vertex: my_shader::vertex_state(&shader, &vertex_entry),
        fragment: Some(my_shader::fragment_state(&shader, &my_shader::fs_main_entry([
            Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })
        ]))),
        // ... other pipeline state
    })
}

fn setup_bind_group(device: &wgpu::Device, texture_view: &wgpu::TextureView, sampler: &wgpu::Sampler) -> my_shader::WgpuBindGroup0 {
    // Create uniform buffer with generated struct
    let uniforms = my_shader::Uniforms::new(
        glam::Mat4::IDENTITY,  // transform
        0.0,                   // time
    );
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    
    // Create bind group using generated types - fully type-safe!
    my_shader::WgpuBindGroup0::from_bindings(
        device,
        my_shader::WgpuBindGroup0Entries::new(my_shader::WgpuBindGroup0EntriesParams {
            uniforms: wgpu::BufferBinding {
                buffer: &uniform_buffer,
                offset: 0,
                size: None,
            },
            my_texture: texture_view,
            my_sampler: sampler,
        })
    )
}
```

🎉 **That's it!** Your shader bindings are now fully type-safe and will automatically update when you modify your WGSL files.

> 📚 **See the [example project](./example) for a complete working demo with multiple shaders, including advanced features like texture arrays and overlay rendering.**

## 🔧 Advanced Configuration

### Serialization Strategies

Choose how your WGSL types are serialized to Rust:

```rust
// For zero-copy, compile-time verified layouts (recommended)
.serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)

// For runtime padding/alignment handling
.serialization_strategy(WgslTypeSerializeStrategy::Encase)
```

### Type Mapping

Use your preferred math library:

```rust
// glam (recommended for games)
.type_map(GlamWgslTypeMap)

// nalgebra (recommended for scientific computing)  
.type_map(NalgebraWgslTypeMap)

// Use built-in Rust arrays (no external dependencies)
.type_map(RustWgslTypeMap)
```

### Custom Types

Override specific types or structs:

```rust
.override_struct_field_type([
    ("MyStruct", "my_field", quote!(MyCustomType))
])
.add_override_struct_mapping(("MyWgslStruct", quote!(my_crate::MyRustStruct)))
```

### Shader Source Options

Control how shaders are embedded:

```rust
// Embed shader source directly (recommended for most cases)
.shader_source_type(WgslShaderSourceType::EmbedSource)

// Use file paths for hot-reloading during development
.shader_source_type(WgslShaderSourceType::HardCodedFilePath)

// Use naga-oil composer for advanced import features
.shader_source_type(WgslShaderSourceType::EmbedWithNagaOilComposer)

// Use relative paths with custom file loading (no nightly Rust required)
// Requires wgpu "naga-ir" feature for optimal performance
.shader_source_type(WgslShaderSourceType::ComposerWithRelativePath)
```

### Using Custom File Loading

The `ComposerWithRelativePath` option allows you to provide your own file loading logic, which is perfect for integrating with custom asset systems.

**Performance Note**: This mode uses wgpu's `naga-ir` feature to pass Naga IR modules directly to the GPU instead of converting back to WGSL source. This provides better performance by avoiding the round-trip conversion process. Make sure to enable the feature in your dependencies:

```toml
[dependencies]
wgpu = { version = "25", features = ["naga-ir"] }
```

```rust
// In your build.rs
.shader_source_type(WgslShaderSourceType::ComposerWithRelativePath)

// In your application code
let module = main::load_naga_module_from_path(
    "assets/shaders",  // Base directory
    ShaderEntry::Main, // Entry point enum variant
    &mut composer,
    shader_defs,
    |path| std::fs::read_to_string(path), // Your custom file loader
)?;

// Or use your own asset system
let module = main::load_naga_module_from_path(
    "shaders",
    ShaderEntry::Main,
    &mut composer,
    shader_defs,
    |path| asset_manager.load_text_file(path), // Custom asset manager
)?;
```

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
4. If not, the second possible path it would have tried would be `src/shaders/utils.wgsl` treating `math` as an item within `utils.wgsl` had it existed.

This strategy allows `wgsl_bindgen` to handle a variety of import statement formats and directory structures, providing flexibility in how you organize your WGSL source files.

## Memory Layout

WGSL structs have different memory layout requirements than Rust structs or standard layout algorithms like `repr(C)` or `repr(packed)`. Matching the expected layout to share data between the CPU and GPU can be tedious and error prone. wgsl_bindgen offers options to add derives for [encase](https://crates.io/crates/encase) to handle padding and alignment at runtime or [bytemuck](https://crates.io/crates/bytemuck) for enforcing padding and alignment at compile time.

When deriving bytemuck, wgsl_bindgen will use naga's layout calculations to add const assertions to ensure that all fields of host-shareable types (structs for uniform and storage buffers) have the correct offset, size, and alignment expected by WGSL.

## Bind Groups

wgpu uses resource bindings organized into bind groups to define global shader resources like textures and buffers. Shaders can have many resource bindings organized into up to 4 bind groups. wgsl_bindgen will generate types and functions for initializing and setting these bind groups in a more typesafe way. Adding, removing, or changing bind groups in the WGSl shader will typically result in a compile error instead of a runtime error when compiling the code without updating the code for creating or using these bind groups.

While bind groups can easily be set all at once using the `set_bind_groups` function, it's recommended to organize bindings into bindgroups based on their update frequency. Bind group 0 will change the least frequently like per frame resources with bind group 3 changing most frequently like per draw resources. Bind groups can be set individually using their `set(render_pass)` method. This can provide a small performance improvement for scenes with many draw calls. See [descriptor table frequency (DX12)](https://learn.microsoft.com/en-us/windows/win32/direct3d12/advanced-use-of-descriptor-tables#changing-descriptor-table-entries-between-rendering-calls) and [descriptor set frequency (Vulkan)](https://vkguide.dev/docs/chapter-4/descriptors/#mental-model) for details.

Organizing bind groups in this way can also help to better organize rendering resources in application code instead of redundantly storing all resources with each object. The `BindGroup0` may only need to be stored once while `WgpuBindGroup3` may be stored for each mesh in the scene. Note that bind groups store references to their underlying resource bindings, so it is not necessary to recreate a bind group if the only the uniform or storage buffer contents change. Avoid creating new bind groups during rendering if possible for best performance.

## 🔍 Best Practices

### Performance Tips

1. **Organize bind groups by update frequency**:
   ```rust
   // Bind group 0: Per-frame data (transforms, time)
   // Bind group 1: Per-material data (textures, material properties)  
   // Bind group 2: Per-object data (model matrices, instance data)
   ```

2. **Use RenderBundles for static geometry**:
   ```rust
   let render_bundle = device.create_render_bundle_encoder(&descriptor);
   bind_group.set(&mut render_bundle);
   render_bundle.draw(0..vertex_count, 0..1);
   let bundle = render_bundle.finish(&descriptor);
   ```

3. **Prefer bytemuck for zero-copy performance**:
   ```rust
   .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
   ```

### Development Workflow

1. **Start with your WGSL shaders** - design your rendering pipeline in the shader language
2. **Configure wgsl_bindgen** - set up your build script with appropriate options
3. **Use generated types** - let the compiler guide you to correct usage
4. **Iterate safely** - modify shaders and let Rust catch any breaking changes

### Common Patterns

```rust
// Generated structs work seamlessly with wgpu
let vertices = vec![
    my_shader::VertexInput::new(glam::Vec3::ZERO, glam::Vec2::ZERO),
    my_shader::VertexInput::new(glam::Vec3::X, glam::Vec2::X),
    my_shader::VertexInput::new(glam::Vec3::Y, glam::Vec2::Y),
];

// Update uniforms safely with type checking
let uniforms = my_shader::Uniforms::new(
    camera.view_projection_matrix(),
    time.elapsed_secs(),
);
queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
```

## ⚠️ Current Limitations

- Some advanced WGSL features may not be fully supported yet - please [file an issue](https://github.com/Swoorup/wgsl-bindgen/issues) for missing features
- Vertex attributes currently assume standard float types rather than normalized integer formats
- All textures are assumed to be filterable (can be resolved with `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`)
- Generated code prioritizes safety and convenience over maximum performance (you can optimize specific hotspots manually when needed)

## 🤝 Contributing

We welcome contributions! Please see our [contribution guidelines](CONTRIBUTING.md) for details on:

- Reporting bugs and requesting features  
- Setting up the development environment
- Running tests and adding new test cases
- Code style and documentation standards

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [naga-oil](https://github.com/bevyengine/naga_oil) - WGSL import system and preprocessing
- [wgpu](https://github.com/gfx-rs/wgpu) - WebGPU implementation for Rust
- [naga](https://github.com/gfx-rs/naga) - Shader translation and validation
- [wgsl_to_wgpu](https://github.com/ScanMountGoat/wgsl_to_wgpu/) - Original inspiration
- The WebGPU working group for the WGSL specification

## Differences from the [wgsl_to_wgpu](https://github.com/ScanMountGoat/wgsl_to_wgpu/) fork.

-   Supports WGSL import syntax and many more features from naga oil flavour.
-   You can only choose either bytemuck or encase for serialization
-   Bytemuck mode supports Runtime-Sized-Array as generic const array in rust.
-   Bytemuck mode correctly adds padding for mat3x3, vec3, whereas original would fail at compile assertions.
    (The fork was mostly born out of reason to use bytemuck and ensure it works in all cases instead of [refusing certain types](https://github.com/ScanMountGoat/wgsl_to_wgpu/pull/52).)
-   User can provide their own wgsl type mappings using `quote` library
-   Expect small api surface breaking change.

## Publishing Crates

The provided example project outputs the generated bindings to the `src/` directory for documentation purposes.
This approach is also fine for applications. Published crates should follow the recommendations for build scripts in the [Cargo Book](https://doc.rust-lang.org/cargo/reference/build-scripts.html#case-study-code-generation).

```rust
use miette::{IntoDiagnostic, Result};
use wgsl_bindgen::{WgslTypeSerializeStrategy, WgslBindgenOptionBuilder, GlamWgslTypeMap};

// src/build.rs
fn main() -> Result<()> {
    WgslBindgenOptionBuilder::default()
        .workspace_root("src/shader")
        .add_entry_point("src/shader/testbed.wgsl")
        .add_entry_point("src/shader/triangle.wgsl")
        .serialization_strategy(WgslTypeSerializeStrategy::Bytemuck)
        .type_map(GlamWgslTypeMap)
        .derive_serde(false)
        .output("src/shader.rs")
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
