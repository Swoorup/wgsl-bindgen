use naga::StructMember;
use proc_macro2::TokenStream;
use quote::quote;

use crate::quote_gen::RustItemPath;

pub fn shader_stages(module: &naga::Module) -> wgpu::ShaderStages {
  module
    .entry_points
    .iter()
    .map(|entry| match entry.stage {
      naga::ShaderStage::Vertex => wgpu::ShaderStages::VERTEX,
      naga::ShaderStage::Fragment => wgpu::ShaderStages::FRAGMENT,
      naga::ShaderStage::Compute => wgpu::ShaderStages::COMPUTE,
    })
    .collect()
}

pub fn buffer_binding_type(storage: naga::AddressSpace) -> TokenStream {
  match storage {
    naga::AddressSpace::Uniform => quote!(wgpu::BufferBindingType::Uniform),
    naga::AddressSpace::Storage { access } => {
      let _is_read = access.contains(naga::StorageAccess::LOAD);
      let is_write = access.contains(naga::StorageAccess::STORE);

      // TODO: Is this correct?
      if is_write {
        quote!(wgpu::BufferBindingType::Storage { read_only: false })
      } else {
        quote!(wgpu::BufferBindingType::Storage { read_only: true })
      }
    }
    _ => todo!(),
  }
}

pub fn vertex_format(ty: &naga::Type) -> wgpu::VertexFormat {
  // Not all wgsl types work as vertex attributes in wgpu.
  match &ty.inner {
    naga::TypeInner::Scalar(scalar) => match (scalar.kind, scalar.width) {
      (naga::ScalarKind::Sint, 4) => wgpu::VertexFormat::Sint32,
      (naga::ScalarKind::Uint, 4) => wgpu::VertexFormat::Uint32,
      (naga::ScalarKind::Float, 4) => wgpu::VertexFormat::Float32,
      (naga::ScalarKind::Float, 8) => wgpu::VertexFormat::Float64,
      _ => todo!(),
    },
    naga::TypeInner::Vector { size, scalar } => match size {
      naga::VectorSize::Bi => match (scalar.kind, scalar.width) {
        (naga::ScalarKind::Sint, 1) => wgpu::VertexFormat::Sint8x2,
        (naga::ScalarKind::Uint, 1) => wgpu::VertexFormat::Uint8x2,
        (naga::ScalarKind::Sint, 2) => wgpu::VertexFormat::Sint16x2,
        (naga::ScalarKind::Uint, 2) => wgpu::VertexFormat::Uint16x2,
        (naga::ScalarKind::Uint, 4) => wgpu::VertexFormat::Uint32x2,
        (naga::ScalarKind::Sint, 4) => wgpu::VertexFormat::Sint32x2,
        (naga::ScalarKind::Float, 4) => wgpu::VertexFormat::Float32x2,
        (naga::ScalarKind::Float, 8) => wgpu::VertexFormat::Float64x2,
        _ => todo!(),
      },
      naga::VectorSize::Tri => match (scalar.kind, scalar.width) {
        (naga::ScalarKind::Uint, 4) => wgpu::VertexFormat::Uint32x3,
        (naga::ScalarKind::Sint, 4) => wgpu::VertexFormat::Sint32x3,
        (naga::ScalarKind::Float, 4) => wgpu::VertexFormat::Float32x3,
        (naga::ScalarKind::Float, 8) => wgpu::VertexFormat::Float64x3,
        _ => todo!(),
      },
      naga::VectorSize::Quad => match (scalar.kind, scalar.width) {
        (naga::ScalarKind::Sint, 1) => wgpu::VertexFormat::Sint8x4,
        (naga::ScalarKind::Uint, 1) => wgpu::VertexFormat::Uint8x4,
        (naga::ScalarKind::Sint, 2) => wgpu::VertexFormat::Sint16x4,
        (naga::ScalarKind::Uint, 2) => wgpu::VertexFormat::Uint16x4,
        (naga::ScalarKind::Uint, 4) => wgpu::VertexFormat::Uint32x4,
        (naga::ScalarKind::Sint, 4) => wgpu::VertexFormat::Sint32x4,
        (naga::ScalarKind::Float, 4) => wgpu::VertexFormat::Float32x4,
        (naga::ScalarKind::Float, 8) => wgpu::VertexFormat::Float64x4,
        _ => todo!(),
      },
    },
    _ => todo!(), // are these types even valid as attributes?
  }
}

pub struct VertexInput {
  pub item_path: RustItemPath,
  pub fields: Vec<(u32, StructMember)>,
}

// TODO: Handle errors.
// Collect the necessary data to generate an equivalent Rust struct.
pub fn get_vertex_input_structs(
  invoking_entry_module: &str,
  module: &naga::Module,
) -> Vec<VertexInput> {
  // TODO: Handle multiple entries?
  module
    .entry_points
    .iter()
    .find(|e| e.stage == naga::ShaderStage::Vertex)
    .map(|vertex_entry| {
      vertex_entry
        .function
        .arguments
        .iter()
        .filter(|a| a.binding.is_none())
        .filter_map(|argument| {
          let arg_type = &module.types[argument.ty];
          match &arg_type.inner {
            naga::TypeInner::Struct { members, span: _ } => {
              let item_path = RustItemPath::from_mangled(
                arg_type.name.as_ref().unwrap(),
                invoking_entry_module,
              );

              let input = VertexInput {
                item_path,
                fields: members
                  .iter()
                  .filter_map(|member| {
                    // Skip builtins since they have no location binding.
                    let location = match member.binding.as_ref().unwrap() {
                      naga::Binding::BuiltIn(_) => None,
                      naga::Binding::Location { location, .. } => Some(*location),
                    }?;

                    Some((location, member.clone()))
                  })
                  .collect(),
              };

              Some(input)
            }
            // An argument has to have a binding unless it is a structure.
            _ => None,
          }
        })
        .collect()
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
  use indoc::indoc;
  use pretty_assertions::assert_eq;

  use super::*;

  #[test]
  fn shader_stages_none() {
    let source = indoc! {r#"

        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert_eq!(wgpu::ShaderStages::NONE, shader_stages(&module));
  }

  #[test]
  fn shader_stages_vertex() {
    let source = indoc! {r#"
            @vertex
            fn main()  {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert_eq!(wgpu::ShaderStages::VERTEX, shader_stages(&module));
  }

  #[test]
  fn shader_stages_fragment() {
    let source = indoc! {r#"
            @fragment
            fn main()  {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert_eq!(wgpu::ShaderStages::FRAGMENT, shader_stages(&module));
  }

  #[test]
  fn shader_stages_vertex_fragment() {
    let source = indoc! {r#"
            @vertex
            fn vs_main()  {}

            @fragment
            fn fs_main()  {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert_eq!(wgpu::ShaderStages::VERTEX_FRAGMENT, shader_stages(&module));
  }

  #[test]
  fn shader_stages_compute() {
    let source = indoc! {r#"
            @compute
            @workgroup_size(64)
            fn main()  {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert_eq!(wgpu::ShaderStages::COMPUTE, shader_stages(&module));
  }

  #[test]
  fn shader_stages_all() {
    let source = indoc! {r#"
            @vertex
            fn vs_main()  {}

            @fragment
            fn fs_main()  {}

            @compute
            @workgroup_size(64)
            fn cs_main()  {}
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();
    assert_eq!(wgpu::ShaderStages::all(), shader_stages(&module));
  }

  #[test]
  fn vertex_input_structs_two_structs() {
    let source = indoc! {r#"
            struct VertexInput0 {
                @location(0) in0: vec4<f32>,
                @location(1) in1: vec4<f32>,
                @location(2) in2: vec4<f32>,
            };
            
            struct VertexInput1 {
                @location(3) in3: vec4<f32>,
                @location(4) in4: vec4<f32>,
                @builtin(vertex_index) index: u32,
                @location(5) in5: vec4<f32>,
                @location(6) in6: vec4<u32>,
            };

            @vertex
            fn main(
                in0: VertexInput0,
                in1: VertexInput1,
                @builtin(front_facing) in2: bool,
                @location(7) in3: vec4<f32>,
            ) -> @builtin(position) vec4<f32> {
                return vec4<f32>(0.0);
            }
        "#};

    let module = naga::front::wgsl::parse_str(source).unwrap();

    let vertex_inputs = get_vertex_input_structs("", &module);
    // Only structures should be included.
    assert_eq!(2, vertex_inputs.len());

    assert_eq!("VertexInput0", vertex_inputs[0].item_path.name);
    assert_eq!(3, vertex_inputs[0].fields.len());
    assert_eq!("in1", vertex_inputs[0].fields[1].1.name.as_ref().unwrap());
    assert_eq!(1, vertex_inputs[0].fields[1].0);

    assert_eq!("VertexInput1", vertex_inputs[1].item_path.name);
    assert_eq!(4, vertex_inputs[1].fields.len());
    assert_eq!("in5", vertex_inputs[1].fields[2].1.name.as_ref().unwrap());
    assert_eq!(5, vertex_inputs[1].fields[2].0);
  }
}
