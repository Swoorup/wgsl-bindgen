use case::CaseExt;
use naga::ShaderStage;
use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::{Ident, Index};

use crate::quote_gen::{RustItem, RustItemKind};
use crate::wgsl;

fn fragment_target_count(module: &naga::Module, f: &naga::Function) -> usize {
  match &f.result {
    Some(r) => match &r.binding {
      Some(b) => {
        // Builtins don't have render targets.
        if matches!(b, naga::Binding::Location { .. }) {
          1
        } else {
          0
        }
      }
      None => {
        // Fragment functions should return a single variable or a struct.
        match &module.types[r.ty].inner {
          naga::TypeInner::Struct { members, .. } => members
            .iter()
            .filter(|m| matches!(m.binding, Some(naga::Binding::Location { .. })))
            .count(),
          _ => 0,
        }
      }
    },
    None => 0,
  }
}

pub fn entry_point_constants(module: &naga::Module) -> TokenStream {
  let entry_points: Vec<TokenStream> = module
    .entry_points
    .iter()
    .map(|entry_point| {
      let entry_name = Literal::string(&entry_point.name);
      let const_name = Ident::new(
        &format!("ENTRY_{}", &entry_point.name.to_uppercase()),
        Span::call_site(),
      );
      quote! {
          pub const #const_name: &str = #entry_name;
      }
    })
    .collect();

  quote! {
      #(#entry_points)*
  }
}

pub fn vertex_states(invoking_entry_module: &str, module: &naga::Module) -> TokenStream {
  let vertex_input_structs =
    wgsl::get_vertex_input_structs(invoking_entry_module, module);

  let mut step_mode_params = vec![];
  let layout_expressions: Vec<TokenStream> = vertex_input_structs
    .iter()
    .map(|input| {
      let struct_ref = input.item_path.short_token_stream(invoking_entry_module);
      let step_mode = Ident::new(&input.item_path.name.to_snake(), Span::call_site());
      step_mode_params.push(quote!(#step_mode: wgpu::VertexStepMode));
      quote!(#struct_ref::vertex_buffer_layout(#step_mode))
    })
    .collect();

  let vertex_entries: Vec<TokenStream> = module
    .entry_points
    .iter()
    .filter_map(|entry_point| match &entry_point.stage {
      ShaderStage::Vertex => {
        let fn_name =
          Ident::new(&format!("{}_entry", &entry_point.name), Span::call_site());

        let const_name = Ident::new(
          &format!("ENTRY_{}", &entry_point.name.to_uppercase()),
          Span::call_site(),
        );

        let n = vertex_input_structs.len();
        let n = Literal::usize_unsuffixed(n);

        let overrides = if !module.overrides.is_empty() {
          Some(quote!(overrides: &OverrideConstants))
        } else {
          None
        };

        let constants = if !module.overrides.is_empty() {
          quote!(overrides.constants())
        } else {
          quote!(Default::default())
        };

        let params = if step_mode_params.is_empty() {
          quote!(#overrides)
        } else {
          quote!(#(#step_mode_params),*, #overrides)
        };

        Some(quote! {
            pub fn #fn_name(#params) -> VertexEntry<#n> {
                VertexEntry {
                    entry_point: #const_name,
                    buffers: [
                        #(#layout_expressions),*
                    ],
                    constants: #constants
                }
            }
        })
      }
      _ => None,
    })
    .collect();

  // Don't generate unused code.
  if vertex_entries.is_empty() {
    quote!()
  } else {
    quote! {
        #[derive(Debug)]
        pub struct VertexEntry<const N: usize> {
            pub entry_point: &'static str,
            pub buffers: [wgpu::VertexBufferLayout<'static>; N],
            pub constants: std::collections::HashMap<String, f64>,
        }

        pub fn vertex_state<'a, const N: usize>(
            module: &'a wgpu::ShaderModule,
            entry: &'a VertexEntry<N>,
        ) -> wgpu::VertexState<'a> {
            wgpu::VertexState {
                module,
                entry_point: entry.entry_point,
                buffers: &entry.buffers,
                compilation_options: wgpu::PipelineCompilationOptions {
                  constants: &entry.constants,
                  ..Default::default()
                },
            }
        }

        #(#vertex_entries)*
    }
  }
}

pub fn vertex_struct_impls(
  invoking_entry_module: &str,
  module: &naga::Module,
) -> Vec<RustItem> {
  let structs = vertex_input_structs_impls(invoking_entry_module, module);
  structs
}

fn vertex_input_structs_impls(
  invoking_entry_module: &str,
  module: &naga::Module,
) -> Vec<RustItem> {
  let vertex_inputs = wgsl::get_vertex_input_structs(invoking_entry_module, module);
  vertex_inputs.iter().map(|input|  {
        let name = Ident::new(&input.item_path.name, Span::call_site());

        // Use index to avoid adding prefix to literals.
        let count = Index::from(input.fields.len());
        let attributes: Vec<_> = input
            .fields
            .iter()
            .map(|(location, m)| {
                let field_name: TokenStream = m.name.as_ref().unwrap().parse().unwrap();
                let location = Index::from(*location as usize);
                let format = wgsl::vertex_format(&module.types[m.ty]);
                // TODO: Will the debug implementation always work with the macro?
                let format = Ident::new(&format!("{format:?}"), Span::call_site());

                quote! {
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::#format,
                        offset: std::mem::offset_of!(Self, #field_name) as u64,
                        shader_location: #location,
                    }
                }
            })
            .collect();


        // The vertex_attr_array! macro doesn't account for field alignment.
        // Structs with glam::Vec4 and glam::Vec3 fields will not be tightly packed.
        // Manually calculate the Rust field offsets to support using bytemuck for vertices.
        // This works since we explicitly mark all generated structs as repr(C).
        // Assume elements are in Rust arrays or slices, so use size_of for stride.
        // TODO: Should this enforce WebGPU alignment requirements for compatibility?
        // https://gpuweb.github.io/gpuweb/#abstract-opdef-validating-gpuvertexbufferlayout

        // TODO: Support vertex inputs that aren't in a struct.
        let ts = quote! {
            impl #name {
                pub const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; #count] = [#(#attributes),*];

                pub const fn vertex_buffer_layout(step_mode: wgpu::VertexStepMode) -> wgpu::VertexBufferLayout<'static> {
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Self>() as u64,
                        step_mode,
                        attributes: &Self::VERTEX_ATTRIBUTES
                    }
                }
            }
        };

    RustItem { kind: RustItemKind::TypeImpls, path: input.item_path.clone(), item: ts }
    }).collect()
}

pub fn fragment_states(module: &naga::Module) -> TokenStream {
  let entries: Vec<TokenStream> = module
    .entry_points
    .iter()
    .filter_map(|entry_point| match &entry_point.stage {
      ShaderStage::Fragment => {
        let fn_name =
          Ident::new(&format!("{}_entry", &entry_point.name), Span::call_site());

        let const_name = Ident::new(
          &format!("ENTRY_{}", &entry_point.name.to_uppercase()),
          Span::call_site(),
        );

        // Use index to avoid adding prefix to literals.
        let target_count =
          Index::from(fragment_target_count(module, &entry_point.function));

        let overrides = if !module.overrides.is_empty() {
          Some(quote!(overrides: &OverrideConstants))
        } else {
          None
        };

        let constants = if !module.overrides.is_empty() {
          quote!(overrides.constants())
        } else {
          quote!(Default::default())
        };

        Some(quote! {
            pub fn #fn_name(
                targets: [Option<wgpu::ColorTargetState>; #target_count],
                #overrides
            ) -> FragmentEntry<#target_count> {
                FragmentEntry {
                    entry_point: #const_name,
                    targets,
                    constants: #constants
                }
            }
        })
      }
      _ => None,
    })
    .collect();

  // Don't generate unused code.
  if entries.is_empty() {
    quote!()
  } else {
    quote! {
        #[derive(Debug)]
        pub struct FragmentEntry<const N: usize> {
            pub entry_point: &'static str,
            pub targets: [Option<wgpu::ColorTargetState>; N],
            pub constants: std::collections::HashMap<String, f64>,
        }

        pub fn fragment_state<'a, const N: usize>(
            module: &'a wgpu::ShaderModule,
            entry: &'a FragmentEntry<N>,
        ) -> wgpu::FragmentState<'a> {
            wgpu::FragmentState {
                module,
                entry_point: entry.entry_point,
                targets: &entry.targets,
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &entry.constants,
                    ..Default::default()
                },
            }
        }

        #(#entries)*
    }
  }
}

#[cfg(test)]
mod test {
  use indoc::indoc;

  use super::*;
  use crate::assert_tokens_eq;

  #[test]
  fn write_vertex_module_empty() {
    let source = indoc! {r#"
            @vertex
            fn main() {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = vertex_struct_impls("test", &module)
      .into_iter()
      .map(|it| it.item)
      .collect::<TokenStream>();

    assert_tokens_eq!(quote!(), actual);
  }

  #[test]
  fn write_vertex_module_single_input_float32() {
    let source = indoc! {r#"
            struct VertexInput0 {
                @location(0) a: f32,
                @location(1) b: vec2<f32>,
                @location(2) c: vec3<f32>,
                @location(3) d: vec4<f32>,
            };

            @vertex
            fn main(in0: VertexInput0) {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = vertex_struct_impls("test", &module)
      .into_iter()
      .map(|it| it.item)
      .collect::<TokenStream>();

    assert_tokens_eq!(
      quote! {
          impl VertexInput0 {
              pub const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Float32,
                      offset: std::mem::offset_of!(Self, a) as u64,
                      shader_location: 0,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Float32x2,
                      offset: std::mem::offset_of!(Self, b) as u64,
                      shader_location: 1,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Float32x3,
                      offset: std::mem::offset_of!(Self, c) as u64,
                      shader_location: 2,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Float32x4,
                      offset: std::mem::offset_of!(Self, d) as u64,
                      shader_location: 3,
                  },
              ];
              pub const fn vertex_buffer_layout(
                  step_mode: wgpu::VertexStepMode,
              ) -> wgpu::VertexBufferLayout<'static> {
                  wgpu::VertexBufferLayout {
                      array_stride: std::mem::size_of::<Self>() as u64,
                      step_mode,
                      attributes: &Self::VERTEX_ATTRIBUTES,
                  }
              }
          }
      },
      actual
    );
  }

  #[test]
  fn write_vertex_module_single_input_float64() {
    let source = indoc! {r#"
            struct VertexInput0 {
                @location(0) a: f64,
                @location(1) b: vec2<f64>,
                @location(2) c: vec3<f64>,
                @location(3) d: vec4<f64>,
            };

            @vertex
            fn main(in0: VertexInput0) {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = vertex_struct_impls("test", &module)
      .into_iter()
      .map(|it| it.item)
      .collect::<TokenStream>();

    assert_tokens_eq!(
      quote! {
          impl VertexInput0 {
              pub const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Float64,
                      offset: std::mem::offset_of!(Self, a) as u64,
                      shader_location: 0,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Float64x2,
                      offset: std::mem::offset_of!(Self, b) as u64,
                      shader_location: 1,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Float64x3,
                      offset: std::mem::offset_of!(Self, c) as u64,
                      shader_location: 2,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Float64x4,
                      offset: std::mem::offset_of!(Self, d) as u64,
                      shader_location: 3,
                  },
              ];
              pub const fn vertex_buffer_layout(
                  step_mode: wgpu::VertexStepMode,
              ) -> wgpu::VertexBufferLayout<'static> {
                  wgpu::VertexBufferLayout {
                      array_stride: std::mem::size_of::<Self>() as u64,
                      step_mode,
                      attributes: &Self::VERTEX_ATTRIBUTES,
                  }
              }
          }
      },
      actual
    );
  }

  #[test]
  fn write_vertex_module_single_input_sint32() {
    let source = indoc! {r#"
            struct VertexInput0 {
                @location(0) a: i32,
                @location(1) a: vec2<i32>,
                @location(2) a: vec3<i32>,
                @location(3) a: vec4<i32>,

            };

            @vertex
            fn main(in0: VertexInput0) {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = vertex_struct_impls("test", &module)
      .into_iter()
      .map(|it| it.item)
      .collect::<TokenStream>();

    assert_tokens_eq!(
      quote! {
          impl VertexInput0 {
              pub const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Sint32,
                      offset: std::mem::offset_of!(Self, a) as u64,
                      shader_location: 0,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Sint32x2,
                      offset: std::mem::offset_of!(Self, a) as u64,
                      shader_location: 1,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Sint32x3,
                      offset: std::mem::offset_of!(Self, a) as u64,
                      shader_location: 2,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Sint32x4,
                      offset: std::mem::offset_of!(Self, a) as u64,
                      shader_location: 3,
                  },
              ];
              pub const fn vertex_buffer_layout(
                  step_mode: wgpu::VertexStepMode,
              ) -> wgpu::VertexBufferLayout<'static> {
                  wgpu::VertexBufferLayout {
                      array_stride: std::mem::size_of::<Self>() as u64,
                      step_mode,
                      attributes: &Self::VERTEX_ATTRIBUTES,
                  }
              }
          }
      },
      actual
    );
  }

  #[test]
  fn write_vertex_module_single_input_uint32() {
    let source = indoc! {r#"
            struct VertexInput0 {
                @location(0) a: u32,
                @location(1) b: vec2<u32>,
                @location(2) c: vec3<u32>,
                @location(3) d: vec4<u32>,
            };

            @vertex
            fn main(in0: VertexInput0) {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = vertex_struct_impls("test", &module)
      .into_iter()
      .map(|it| it.item)
      .collect::<TokenStream>();

    assert_tokens_eq!(
      quote! {
          impl VertexInput0 {
              pub const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Uint32,
                      offset: std::mem::offset_of!(Self, a) as u64,
                      shader_location: 0,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Uint32x2,
                      offset: std::mem::offset_of!(Self, b) as u64,
                      shader_location: 1,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Uint32x3,
                      offset: std::mem::offset_of!(Self, c) as u64,
                      shader_location: 2,
                  },
                  wgpu::VertexAttribute {
                      format: wgpu::VertexFormat::Uint32x4,
                      offset: std::mem::offset_of!(Self, d) as u64,
                      shader_location: 3,
                  },
              ];
              pub const fn vertex_buffer_layout(
                  step_mode: wgpu::VertexStepMode,
              ) -> wgpu::VertexBufferLayout<'static> {
                  wgpu::VertexBufferLayout {
                      array_stride: std::mem::size_of::<Self>() as u64,
                      step_mode,
                      attributes: &Self::VERTEX_ATTRIBUTES,
                  }
              }
          }
      },
      actual
    );
  }

  #[test]
  fn write_entry_constants() {
    let source = indoc! {r#"
            @vertex
            fn vs_main() {}

            @vertex
            fn another_vs() {}

            @fragment
            fn fs_main() {}

            @fragment
            fn another_fs() {}
        "#
    };

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = entry_point_constants(&module);

    assert_tokens_eq!(
      quote! {
          pub const ENTRY_VS_MAIN: &str = "vs_main";
          pub const ENTRY_ANOTHER_VS: &str = "another_vs";
          pub const ENTRY_FS_MAIN: &str = "fs_main";
          pub const ENTRY_ANOTHER_FS: &str = "another_fs";
      },
      actual
    )
  }

  #[test]
  fn write_vertex_shader_entry_no_buffers() {
    let source = indoc! {r#"
            @vertex
            fn vs_main() {}
        "#
    };

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = vertex_states("test", &module);

    assert_tokens_eq!(
      quote! {
          #[derive(Debug)]
          pub struct VertexEntry<const N: usize> {
              pub entry_point: &'static str,
              pub buffers: [wgpu::VertexBufferLayout<'static>; N],
              pub constants: std::collections::HashMap<String, f64>,
          }
          pub fn vertex_state<'a, const N: usize>(
              module: &'a wgpu::ShaderModule,
              entry: &'a VertexEntry<N>,
          ) -> wgpu::VertexState<'a> {
              wgpu::VertexState {
                  module,
                  entry_point: entry.entry_point,
                  buffers: &entry.buffers,
                  compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &entry.constants,
                    ..Default::default()
                  },
              }
          }
          pub fn vs_main_entry() -> VertexEntry<0> {
              VertexEntry {
                  entry_point: ENTRY_VS_MAIN,
                  buffers: [],
                  constants: Default::default(),
              }
          }
      },
      actual
    )
  }

  #[test]
  fn write_vertex_shader_multiple_entries() {
    let source = indoc! {r#"
            struct VertexInput {
                @location(0) position: vec4<f32>,
            };
            @vertex
            fn vs_main_1(in: VertexInput) {}

            @vertex
            fn vs_main_2(in: VertexInput) {}
        "#
    };

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = vertex_states("test", &module);

    assert_tokens_eq!(
      quote! {
          #[derive(Debug)]
          pub struct VertexEntry<const N: usize> {
              pub entry_point: &'static str,
              pub buffers: [wgpu::VertexBufferLayout<'static>; N],
              pub constants: std::collections::HashMap<String, f64>,
          }
          pub fn vertex_state<'a, const N: usize>(
              module: &'a wgpu::ShaderModule,
              entry: &'a VertexEntry<N>,
          ) -> wgpu::VertexState<'a> {
              wgpu::VertexState {
                  module,
                  entry_point: entry.entry_point,
                  buffers: &entry.buffers,
                  compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &entry.constants,
                    ..Default::default()
                  },
              }
          }
          pub fn vs_main_1_entry(vertex_input: wgpu::VertexStepMode) -> VertexEntry<1> {
              VertexEntry {
                  entry_point: ENTRY_VS_MAIN_1,
                  buffers: [VertexInput::vertex_buffer_layout(vertex_input)],
                  constants: Default::default()
              }
          }
          pub fn vs_main_2_entry(vertex_input: wgpu::VertexStepMode) -> VertexEntry<1> {
              VertexEntry {
                  entry_point: ENTRY_VS_MAIN_2,
                  buffers: [VertexInput::vertex_buffer_layout(vertex_input)],
                  constants: Default::default()
              }
          }
      },
      actual
    )
  }

  #[test]
  fn write_vertex_shader_entry_multiple_buffers() {
    let source = indoc! {r#"
            override tests: bool = false;
            struct Input0 {
                @location(0) position: vec4<f32>,
            };
            struct Input1 {
                @location(1) some_data: vec2<f32>
            }
            @vertex
            fn vs_main(in0: Input0, in1: Input1) {}
        "#
    };

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = vertex_states("test", &module);

    assert_tokens_eq!(
      quote! {
          #[derive(Debug)]
          pub struct VertexEntry<const N: usize> {
              pub entry_point: &'static str,
              pub buffers: [wgpu::VertexBufferLayout<'static>; N],
              pub constants: std::collections::HashMap<String, f64>
          }
          pub fn vertex_state<'a, const N: usize>(
              module: &'a wgpu::ShaderModule,
              entry: &'a VertexEntry<N>,
          ) -> wgpu::VertexState<'a> {
              wgpu::VertexState {
                  module,
                  entry_point: entry.entry_point,
                  buffers: &entry.buffers,
                  compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &entry.constants,
                    ..Default::default()
                  },
              }
          }
          pub fn vs_main_entry(
            input0: wgpu::VertexStepMode,
            input1: wgpu::VertexStepMode,
            overrides: &OverrideConstants
          ) -> VertexEntry<2> {
              VertexEntry {
                  entry_point: ENTRY_VS_MAIN,
                  buffers: [
                      Input0::vertex_buffer_layout(input0),
                      Input1::vertex_buffer_layout(input1),
                  ],
                  constants: overrides.constants(),
              }
          }
      },
      actual
    )
  }

  #[test]
  fn write_vertex_states_no_entries() {
    let source = indoc! {r#"
            struct Input {
                @location(0) position: vec4<f32>,
            };
            @fragment
            fn main(in: Input) {}
        "#
    };

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = vertex_states("test", &module);

    assert_tokens_eq!(quote!(), actual)
  }

  #[test]
  fn write_fragment_states_multiple_entries() {
    let source = indoc! {r#"
          struct Output {
              @location(0) col0: vec4<f32>,
              @builtin(frag_depth) depth: f32,
              @location(1) col1: vec4<f32>,
          };
          @fragment
          fn fs_multiple() -> Output {}
          @fragment
          fn fs_single() -> @location(0) vec4<f32> {}
          @fragment
          fn fs_single_builtin() -> @builtin(frag_depth) f32 {}
          @fragment
          fn fs_empty() {}
      "#
    };

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = fragment_states(&module);

    assert_tokens_eq!(
      quote! {
          #[derive(Debug)]
          pub struct FragmentEntry<const N: usize> {
              pub entry_point: &'static str,
              pub targets: [Option<wgpu::ColorTargetState>; N],
              pub constants: std::collections::HashMap<String, f64>,
          }
          pub fn fragment_state<'a, const N: usize>(
              module: &'a wgpu::ShaderModule,
              entry: &'a FragmentEntry<N>,
          ) -> wgpu::FragmentState<'a> {
              wgpu::FragmentState {
                  module,
                  entry_point: entry.entry_point,
                  targets: &entry.targets,
                  compilation_options: wgpu::PipelineCompilationOptions {
                      constants: &entry.constants,
                      ..Default::default()
                  },
              }
          }
          pub fn fs_multiple_entry(
              targets: [Option<wgpu::ColorTargetState>; 2]
          ) -> FragmentEntry<2> {
              FragmentEntry {
                  entry_point: ENTRY_FS_MULTIPLE,
                  targets,
                  constants: Default::default(),
              }
          }
          pub fn fs_single_entry(
              targets: [Option<wgpu::ColorTargetState>; 1]
          ) -> FragmentEntry<1> {
              FragmentEntry {
                  entry_point: ENTRY_FS_SINGLE,
                  targets,
                  constants: Default::default(),
              }
          }
          pub fn fs_single_builtin_entry(
              targets: [Option<wgpu::ColorTargetState>; 0]
          ) -> FragmentEntry<0> {
              FragmentEntry {
                  entry_point: ENTRY_FS_SINGLE_BUILTIN,
                  targets,
                  constants: Default::default(),
              }
          }
          pub fn fs_empty_entry(
              targets: [Option<wgpu::ColorTargetState>; 0]
          ) -> FragmentEntry<0> {
              FragmentEntry {
                  entry_point: ENTRY_FS_EMPTY,
                  targets,
                  constants: Default::default(),
              }
          }
      },
      actual
    )
  }

  #[test]
  fn write_fragment_states_single_entry() {
    let source = indoc! {r#"
          override test: bool = true;
          @fragment
          fn fs_single() -> @location(0) vec4<f32> {}
      "#
    };

    let module = naga::front::wgsl::parse_str(source).unwrap();
    let actual = fragment_states(&module);

    assert_tokens_eq!(
      quote! {
          #[derive(Debug)]
          pub struct FragmentEntry<const N: usize> {
              pub entry_point: &'static str,
              pub targets: [Option<wgpu::ColorTargetState>; N],
              pub constants: std::collections::HashMap<String, f64>,
          }
          pub fn fragment_state<'a, const N: usize>(
              module: &'a wgpu::ShaderModule,
              entry: &'a FragmentEntry<N>,
          ) -> wgpu::FragmentState<'a> {
              wgpu::FragmentState {
                  module,
                  entry_point: entry.entry_point,
                  targets: &entry.targets,
                  compilation_options: wgpu::PipelineCompilationOptions {
                      constants: &entry.constants,
                      ..Default::default()
                  },
              }
          }
          pub fn fs_single_entry(
              targets: [Option<wgpu::ColorTargetState>; 1],
              overrides: &OverrideConstants
          ) -> FragmentEntry<1> {
              FragmentEntry {
                  entry_point: ENTRY_FS_SINGLE,
                  targets,
                  constants: overrides.constants(),
              }
          }
      },
      actual
    )
  }
}
