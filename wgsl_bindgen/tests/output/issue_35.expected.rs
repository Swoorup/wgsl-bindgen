#![allow(unused, non_snake_case, non_camel_case_types, non_upper_case_globals)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderEntry {
    Clear,
}
impl ShaderEntry {
    pub fn create_pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
        match self {
            Self::Clear => clear::create_pipeline_layout(device),
        }
    }
    pub fn create_shader_module_embedded(
        &self,
        device: &wgpu::Device,
        shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>,
    ) -> wgpu::ShaderModule {
        match self {
            Self::Clear => clear::create_shader_module_embedded(device, shader_defs),
        }
    }
}
mod _root {
    pub use super::*;
}
pub mod layout_asserts {
    use super::{_root, _root::*};
    const WGSL_BASE_TYPE_ASSERTS: () = {
        assert!(std::mem::size_of:: < glam::Vec3A > () == 16);
        assert!(std::mem::align_of:: < glam::Vec3A > () == 16);
        assert!(std::mem::size_of:: < glam::Vec4 > () == 16);
        assert!(std::mem::align_of:: < glam::Vec4 > () == 16);
        assert!(std::mem::size_of:: < glam::Mat3A > () == 48);
        assert!(std::mem::align_of:: < glam::Mat3A > () == 16);
        assert!(std::mem::size_of:: < glam::Mat4 > () == 64);
        assert!(std::mem::align_of:: < glam::Mat4 > () == 16);
    };
}
pub mod vertices {
    use super::{_root, _root::*};
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct VertexIn {
        pub position: glam::Vec4,
    }
    pub const fn VertexIn(position: glam::Vec4) -> VertexIn {
        VertexIn { position }
    }
    impl VertexIn {
        pub const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: std::mem::offset_of!(Self, position) as u64,
                shader_location: 0,
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
}
pub mod bytemuck_impls {
    use super::{_root, _root::*};
    unsafe impl bytemuck::Zeroable for vertices::VertexIn {}
    unsafe impl bytemuck::Pod for vertices::VertexIn {}
}
pub mod clear {
    use super::{_root, _root::*};
    pub const ENTRY_VERTEX_MAIN: &str = "vertex_main";
    pub const ENTRY_FRAGMENT_MAIN: &str = "fragment_main";
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
    pub fn vertex_main_entry(vertex_in: wgpu::VertexStepMode) -> VertexEntry<1> {
        VertexEntry {
            entry_point: ENTRY_VERTEX_MAIN,
            buffers: [vertices::VertexIn::vertex_buffer_layout(vertex_in)],
            constants: Default::default(),
        }
    }
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
    pub fn fragment_main_entry(
        targets: [Option<wgpu::ColorTargetState>; 1],
    ) -> FragmentEntry<1> {
        FragmentEntry {
            entry_point: ENTRY_FRAGMENT_MAIN,
            targets,
            constants: Default::default(),
        }
    }
    #[derive(Debug)]
    pub struct WgpuPipelineLayout;
    impl WgpuPipelineLayout {
        pub fn bind_group_layout_entries(
            entries: [wgpu::BindGroupLayout; 0],
        ) -> [wgpu::BindGroupLayout; 0] {
            entries
        }
    }
    pub fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Clear::PipelineLayout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                },
            )
    }
    pub fn load_shader_modules_embedded(
        composer: &mut naga_oil::compose::Composer,
        shader_defs: &std::collections::HashMap<
            String,
            naga_oil::compose::ShaderDefValue,
        >,
    ) -> () {
        composer
            .add_composable_module(naga_oil::compose::ComposableModuleDescriptor {
                source: include_str!("../shaders/issue_35/vertices.wgsl"),
                file_path: "../shaders/issue_35/vertices.wgsl",
                language: naga_oil::compose::ShaderLanguage::Wgsl,
                shader_defs: shader_defs.clone(),
                as_name: Some("vertices".into()),
                ..Default::default()
            })
            .expect("failed to add composer module");
        ()
    }
    pub fn load_naga_module_embedded(
        composer: &mut naga_oil::compose::Composer,
        shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>,
    ) -> wgpu::naga::Module {
        composer
            .make_naga_module(naga_oil::compose::NagaModuleDescriptor {
                source: include_str!("../shaders/issue_35/clear.wgsl"),
                file_path: "../shaders/issue_35/clear.wgsl",
                shader_defs,
                ..Default::default()
            })
            .expect("failed to build naga module")
    }
    pub fn create_shader_module_embedded(
        device: &wgpu::Device,
        shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue>,
    ) -> wgpu::ShaderModule {
        let mut composer = naga_oil::compose::Composer::default();
        load_shader_modules_embedded(&mut composer, &shader_defs);
        let module = load_naga_module_embedded(&mut composer, shader_defs);
        let info = wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::empty(),
                wgpu::naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap();
        let shader_string = wgpu::naga::back::wgsl::write_string(
                &module,
                &info,
                wgpu::naga::back::wgsl::WriterFlags::empty(),
            )
            .expect("failed to convert naga module to source");
        let source = std::borrow::Cow::Owned(shader_string);
        device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("clear.wgsl"),
                source: wgpu::ShaderSource::Wgsl(source),
            })
    }
}
