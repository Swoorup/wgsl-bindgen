use std::collections::HashSet;

use crate::quote_gen::{RustSourceItem, RustSourceItemPath, RustStructBuilder};
use crate::{WgslBindgenOption, WgslTypeSerializeStrategy};
use naga::{Handle, Type};

/// Returns a list of Rust structs that represent the WGSL structs in the module.
pub fn structs_items(
  invoking_entry_module: &str,
  module: &naga::Module,
  options: &WgslBindgenOption,
) -> Vec<RustSourceItem> {
  // Initialize the layout calculator provided by naga.
  let mut layouter = naga::proc::Layouter::default();
  layouter.update(module.to_ctx()).unwrap();

  let mut global_variable_types = HashSet::new();
  for g in module.global_variables.iter() {
    add_types_recursive(&mut global_variable_types, module, g.1.ty);
  }

  // Create matching Rust structs for WGSL structs.
  // This is a UniqueArena, so each struct will only be generated once.
  module
    .types
    .iter()
    .filter(|(h, _)| {
      // Check if the struct will need to be used by the user from Rust.
      // This includes function inputs like vertex attributes and global variables.
      // Shader stage function outputs will not be accessible from Rust.
      // Skipping internal structs helps avoid issues deriving encase or bytemuck.
      !module
        .entry_points
        .iter()
        .any(|e| e.function.result.as_ref().map(|r| r.ty) == Some(*h))
        && module
          .entry_points
          .iter()
          .any(|e| e.function.arguments.iter().any(|a| a.ty == *h))
        || global_variable_types.contains(h)
    })
    .flat_map(|(t_handle, ty)| {
      if let naga::TypeInner::Struct { members, .. } = &ty.inner {
        let rust_item_path = RustSourceItemPath::from_mangled(
          ty.name.as_ref().unwrap(),
          invoking_entry_module,
        );

        // skip if using custom struct mapping
        if options.type_map.contains_key(&crate::WgslType::Struct {
          fully_qualified_name: rust_item_path.get_fully_qualified_name().into(),
        }) {
          Vec::new()
        } else {
          rust_struct(
            &rust_item_path,
            members,
            &layouter,
            t_handle,
            module,
            options,
            &global_variable_types,
          )
        }
      } else {
        Vec::new()
      }
    })
    .collect()
}

/// Returns a list of Rust structs that represent the WGSL structs in the module.
fn rust_struct(
  rust_item_path: &RustSourceItemPath,
  naga_members: &[naga::StructMember],
  layouter: &naga::proc::Layouter,
  t_handle: naga::Handle<naga::Type>,
  naga_module: &naga::Module,
  options: &WgslBindgenOption,
  global_variable_types: &HashSet<Handle<Type>>,
) -> Vec<RustSourceItem> {
  let layout = layouter[t_handle];

  // Assume types used in global variables are host shareable and require validation.
  // This includes storage, uniform, and workgroup variables.
  // This also means types that are never used will not be validated.
  // Structs used only for vertex inputs do not require validation on desktop platforms.
  // Vertex input layout is handled already by setting the attribute offsets and types.
  // This allows vertex input field types without padding like vec3 for positions.
  let is_host_sharable = global_variable_types.contains(&t_handle);

  let has_rts_array = struct_has_rts_array_member(naga_members, naga_module);
  let is_directly_sharable = options.serialization_strategy
    == WgslTypeSerializeStrategy::Bytemuck
    && is_host_sharable;

  let builder = RustStructBuilder::from_naga(
    rust_item_path,
    naga_members,
    naga_module,
    options,
    layout,
    is_directly_sharable,
    is_host_sharable,
    has_rts_array,
  );
  builder.build()
}

fn add_types_recursive(
  types: &mut HashSet<naga::Handle<naga::Type>>,
  module: &naga::Module,
  ty: Handle<Type>,
) {
  types.insert(ty);

  match &module.types[ty].inner {
    naga::TypeInner::Pointer { base, .. } => add_types_recursive(types, module, *base),
    naga::TypeInner::Array { base, .. } => add_types_recursive(types, module, *base),
    naga::TypeInner::Struct { members, .. } => {
      for member in members {
        add_types_recursive(types, module, member.ty);
      }
    }
    naga::TypeInner::BindingArray { base, .. } => {
      add_types_recursive(types, module, *base)
    }
    _ => (),
  }
}

fn struct_has_rts_array_member(
  members: &[naga::StructMember],
  module: &naga::Module,
) -> bool {
  members.iter().any(|m| {
    matches!(
      module.types[m.ty].inner,
      naga::TypeInner::Array {
        size: naga::ArraySize::Dynamic,
        ..
      }
    )
  })
}

#[cfg(test)]
mod tests {
  use indoc::indoc;
  use proc_macro2::TokenStream;
  use quote::quote;

  use super::*;
  use crate::{
    assert_tokens_snapshot, GlamWgslTypeMap, NalgebraWgslTypeMap, RustWgslTypeMap,
  };
  use crate::{
    WgslBindgenOption, WgslTypeMapBuild, WgslTypeSerializeStrategy, WgslTypeVisibility,
  };

  pub fn structs(module: &naga::Module, options: &WgslBindgenOption) -> Vec<TokenStream> {
    structs_items("", module, options)
      .into_iter()
      .map(|s| s.tokenstream)
      .collect()
  }

  #[test]
  fn write_all_structs_rust() {
    let source = indoc! {r#"
            enable f16;
            struct Scalars {
                a: u32,
                b: i32,
                c: f32,
            };
            var<uniform> a: Scalars;

            struct VectorsU32 {
                a: vec2<u32>,
                b: vec3<u32>,
                c: vec4<u32>,
            };
            var<uniform> b: VectorsU32;

            struct VectorsI32 {
                a: vec2<i32>,
                b: vec3<i32>,
                c: vec4<i32>,
            };
            var<uniform> c: VectorsI32;

            struct VectorsF32 {
                a: vec2<f32>,
                b: vec3<f32>,
                c: vec4<f32>,
            };
            var<uniform> d: VectorsF32;

            struct VectorsF64 {
                a: vec2<f64>,
                b: vec3<f64>,
                c: vec4<f64>,
            };
            var<uniform> e: VectorsF64;

            struct MatricesF32 {
                a: mat4x4<f32>,
                b: mat4x3<f32>,
                c: mat4x2<f32>,
                d: mat3x4<f32>,
                e: mat3x3<f32>,
                f: mat3x2<f32>,
                g: mat2x4<f32>,
                h: mat2x3<f32>,
                i: mat2x2<f32>,
            };
            var<uniform> f: MatricesF32;

            struct MatricesF64 {
                a: mat4x4<f64>,
                b: mat4x3<f64>,
                c: mat4x2<f64>,
                d: mat3x4<f64>,
                e: mat3x3<f64>,
                f: mat3x2<f64>,
                g: mat2x4<f64>,
                h: mat2x3<f64>,
                i: mat2x2<f64>,
            };
            var<uniform> g: MatricesF64;

            struct StaticArrays {
                a: array<u32, 5>,
                b: array<f32, 3>,
                c: array<mat4x4<f32>, 512>,
            };
            var<uniform> h: StaticArrays;

            struct Nested {
                a: MatricesF32,
                b: MatricesF64
            }
            var<uniform> i: Nested;

            struct VectorsF16 {
                a: vec2<f16>,
                b: vec4<f16>,
            };
            var<uniform> j: VectorsF16;

            struct MatricesF16 {
                a: mat4x4<f16>,
                b: mat4x3<f16>,
                c: mat4x2<f16>,
                d: mat3x4<f16>,
                e: mat3x3<f16>,
                f: mat3x2<f16>,
                g: mat2x4<f16>,
                h: mat2x3<f16>,
                i: mat2x2<f16>,
            };
            var<uniform> k: MatricesF16;

            struct Atomics {
                num: atomic<u32>,
                numi: atomic<i32>,
            };
            var <storage, read_write> atomics: Atomics;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(&module, &WgslBindgenOption::default());
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_all_structs_glam() {
    let source = indoc! {r#"
            struct Scalars {
                a: u32,
                b: i32,
                c: f32,
            };
            var<uniform> a: Scalars;

            struct VectorsU32 {
                a: vec2<u32>,
                b: vec3<u32>,
                c: vec4<u32>,
            };
            var<uniform> b: VectorsU32;

            struct VectorsI32 {
                a: vec2<i32>,
                b: vec3<i32>,
                c: vec4<i32>,
            };
            var<uniform> c: VectorsI32;

            struct VectorsF32 {
                a: vec2<f32>,
                b: vec3<f32>,
                c: vec4<f32>,
            };
            var<uniform> d: VectorsF32;

            struct MatricesF32 {
                a: mat4x4<f32>,
                b: mat4x3<f32>,
                c: mat4x2<f32>,
                d: mat3x4<f32>,
                e: mat3x3<f32>,
                f: mat3x2<f32>,
                g: mat2x4<f32>,
                h: mat2x3<f32>,
                i: mat2x2<f32>,
            };
            var<uniform> f: MatricesF32;

            struct StaticArrays {
                a: array<u32, 5>,
                b: array<f32, 3>,
                c: array<mat4x4<f32>, 512>,
            };
            var<uniform> h: StaticArrays;

            struct Nested {
                a: MatricesF32,
                b: VectorsF32
            }
            var<uniform> i: Nested;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        type_map: GlamWgslTypeMap.build(WgslTypeSerializeStrategy::Encase),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_all_structs_nalgebra() {
    let source = indoc! {r#"
            struct Scalars {
                a: u32,
                b: i32,
                c: f32,
            };
            var<uniform> a: Scalars;

            struct VectorsU32 {
                a: vec2<u32>,
                b: vec3<u32>,
                c: vec4<u32>,
            };
            var<uniform> b: VectorsU32;

            struct VectorsI32 {
                a: vec2<i32>,
                b: vec3<i32>,
                c: vec4<i32>,
            };
            var<uniform> c: VectorsI32;

            struct VectorsF32 {
                a: vec2<f32>,
                b: vec3<f32>,
                c: vec4<f32>,
            };
            var<uniform> d: VectorsF32;

            struct MatricesF32 {
                a: mat4x4<f32>,
                b: mat4x3<f32>,
                c: mat4x2<f32>,
                d: mat3x4<f32>,
                e: mat3x3<f32>,
                f: mat3x2<f32>,
                g: mat2x4<f32>,
                h: mat2x3<f32>,
                i: mat2x2<f32>,
            };
            var<uniform> f: MatricesF32;

            struct StaticArrays {
                a: array<u32, 5>,
                b: array<f32, 3>,
                c: array<mat4x4<f32>, 512>,
            };
            var<uniform> h: StaticArrays;

            struct Nested {
                a: MatricesF32,
                b: VectorsF32
            }
            var<uniform> i: Nested;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        type_map: NalgebraWgslTypeMap.build(WgslTypeSerializeStrategy::Encase),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_all_structs_encase() {
    let source = indoc! {r#"
            struct Input0 {
                a: u32,
                b: i32,
                c: f32,
            };

            struct Nested {
                a: Input0,
                b: f32
            }

            var<uniform> a: Input0;
            var<storage, read> b: Nested;

            @fragment
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Encase,
        derive_serde: false,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Encase),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_all_structs_serde_encase() {
    let source = indoc! {r#"
            struct Input0 {
                a: u32,
                b: i32,
                c: f32,
            };

            struct Nested {
                a: Input0,
                b: f32
            }

            var<workgroup> a: Input0;
            var<uniform> b: Nested;

            @compute
            @workgroup_size(64)
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Encase,
        derive_serde: true,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Encase),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_all_structs_skip_stage_outputs() {
    let source = indoc! {r#"
            struct Input0 {
                a: u32,
                b: i32,
                c: f32,
            };

            struct Output0 {
                a: f32
            }

            struct Unused {
                a: vec3<f32>
            }

            @fragment
            fn main(in: Input0) -> Output0 {
                var out: Output0;
                return out;
            }
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        derive_serde: false,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_all_structs_bytemuck_skip_input_layout_validation() {
    // Structs used only for vertex inputs don't require layout validation.
    // Correctly specifying the offsets is handled by the buffer layout itself.
    let source = indoc! {r#"
            struct Input0 {
                a: u32,
                b: i32,
                c: f32,
            };

            @vertex
            fn main(input: Input0) -> vec4<f32> {
                return vec4(0.0);
            }
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        derive_serde: false,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_all_structs_bytemuck_input_layout_validation() {
    // The struct is also used with a storage buffer and should be validated.
    let source = indoc! {r#"
            struct Input0 {
                @size(8)
                a: u32,
                b: i32,
                @align(32) c: f32,
                @builtin(vertex_index) d: u32,
            };

            var<storage, read_write> test: Input0;

            struct Outer {
                inner: Inner
            }

            struct Inner {
                a: f32
            }

            var<storage, read_write> test2: array<Outer>;

            @vertex
            fn main(input: Input0) -> vec4<f32> {
                return vec4(0.0);
            }
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        derive_serde: false,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_atomic_types() {
    let source = indoc! {r#"
            struct Atomics {
                num: atomic<u32>,
                numi: atomic<i32>,
            };

            @group(0) @binding(0)
            var <storage, read_write> atomics:Atomics;
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        type_map: NalgebraWgslTypeMap.build(WgslTypeSerializeStrategy::Encase),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  fn runtime_sized_array_module() -> naga::Module {
    let source = indoc! {r#"
            struct RtsStruct {
                other_data: i32,
                the_array: array<u32>,
            };

            @group(0) @binding(0)
            var <storage, read_write> rts:RtsStruct;
        "#};
    naga::front::wgsl::parse_str(source).unwrap()
  }

  #[test]
  fn write_runtime_sized_array() {
    let module = runtime_sized_array_module();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Encase,
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_runtime_sized_array_bytemuck() {
    let module = runtime_sized_array_module();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        ..Default::default()
      },
    );

    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual)
  }

  #[test]
  #[should_panic]
  fn write_runtime_sized_array_not_last_field() {
    let source = indoc! {r#"
            struct RtsStruct {
                other_data: i32,
                the_array: array<u32>,
                more_data: i32,
            };

            @group(0) @binding(0)
            var <storage, read_write> rts:RtsStruct;
        "#};
    let module = naga::front::wgsl::parse_str(source).unwrap();

    let _structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Encase,
        ..Default::default()
      },
    );
  }

  #[test]
  fn write_nonpower_of_2_mats_for_bytemuck_option() {
    let source = indoc! {r#"
        struct UniformsData {
          a: mat3x3<f32>,
        }

        @group(0) @binding(0)
            var <uniform> un:UniformsData;
      "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_nonpower_of_2_mats_for_bytemuck_glam_option() {
    let source = indoc! {r#"
        struct UniformsData {
          centered_mvp: mat3x3<f32>,
        }

        @group(0) @binding(0)
            var <uniform> un:UniformsData;
      "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        type_map: GlamWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_nonpower_of_2_mats() {
    let source = indoc! {r#"
          struct MatricesF32 {
            a: mat4x4<f32>,
            b: mat4x3<f32>,
            c: mat4x2<f32>,
            d: mat3x4<f32>,
        };
        @group(0) @binding(0)
        var<uniform> f: MatricesF32;
      "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        type_map: RustWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn write_shorter_constructor() {
    let source = indoc! {r#"
        struct Uniform {
            position_data: vec2<f32>,
        };
        @group(0) @binding(0) var<uniform> u: Uniform;
      "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        serialization_strategy: WgslTypeSerializeStrategy::Bytemuck,
        type_map: GlamWgslTypeMap.build(WgslTypeSerializeStrategy::Bytemuck),
        short_constructor: Some(1),
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }

  #[test]
  fn test_struct_visibility() {
    let source = indoc! {r#"
            struct Scalars {
                a: u32,
                b: i32,
                c: f32,
            };
            var<uniform> a: Scalars;
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let structs = structs(
      &module,
      &WgslBindgenOption {
        type_visibility: WgslTypeVisibility::RestrictedCrate,
        ..Default::default()
      },
    );
    let actual = quote!(#(#structs)*);

    assert_tokens_snapshot!(actual);
  }
}
