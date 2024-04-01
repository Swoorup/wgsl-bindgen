#![allow(unused, non_snake_case, non_camel_case_types, non_upper_case_globals)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderEntry {
    Padding,
}
impl ShaderEntry {
    pub fn create_pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
        match self {
            Self::Padding => padding::create_pipeline_layout(device),
        }
    }
    pub fn create_shader_module_embed_source(
        &self,
        device: &wgpu::Device,
    ) -> wgpu::ShaderModule {
        match self {
            Self::Padding => padding::create_shader_module_embed_source(device),
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
    const PADDING_STYLE_ASSERTS: () = {
        assert!(std::mem::offset_of!(padding::Style, color) == 0);
        assert!(std::mem::offset_of!(padding::Style, width) == 16);
        assert!(std::mem::size_of:: < padding::Style > () == 32);
    };
}
pub mod padding {
    use super::{_root, _root::*};
    #[repr(C, align(16))]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct Style {
        /// size: 16, offset: 0x0, type: `vec4<f32>`
        pub color: glam::Vec4,
        /// size: 4, offset: 0x10, type: `f32`
        pub width: f32,
        pub _pad_width: [u8; 0x8 - core::mem::size_of::<f32>()],
        pub _padding: [u8; 0x8],
    }
    impl Style {
        pub const fn new(color: glam::Vec4, width: f32) -> Self {
            Self {
                color,
                width,
                _pad_width: [0; 0x8 - core::mem::size_of::<f32>()],
                _padding: [0; 0x8],
            }
        }
    }
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct StyleInit {
        pub color: glam::Vec4,
        pub width: f32,
    }
    impl StyleInit {
        pub const fn build(&self) -> Style {
            Style {
                color: self.color,
                width: self.width,
                _pad_width: [0; 0x8 - core::mem::size_of::<f32>()],
                _padding: [0; 0x8],
            }
        }
    }
    impl From<StyleInit> for Style {
        fn from(data: StyleInit) -> Self {
            data.build()
        }
    }
    pub mod bind_groups {
        #[derive(Debug)]
        pub struct WgpuBindGroupLayout0<'a> {
            pub frame: wgpu::BufferBinding<'a>,
        }
        impl<'a> WgpuBindGroupLayout0<'a> {
            pub fn entries(self) -> [wgpu::BindGroupEntry<'a>; 1] {
                [
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(self.frame),
                    },
                ]
            }
        }
        #[derive(Debug)]
        pub struct WgpuBindGroup0(wgpu::BindGroup);
        impl WgpuBindGroup0 {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                label: Some("Padding::BindGroup0::LayoutDescriptor"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: true,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            };
            pub fn get_bind_group_layout(
                device: &wgpu::Device,
            ) -> wgpu::BindGroupLayout {
                device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
            }
            pub fn from_bindings(
                device: &wgpu::Device,
                bindings: WgpuBindGroupLayout0,
            ) -> Self {
                let bind_group_layout = Self::get_bind_group_layout(&device);
                let entries = bindings.entries();
                let bind_group = device
                    .create_bind_group(
                        &wgpu::BindGroupDescriptor {
                            label: Some("Padding::BindGroup0"),
                            layout: &bind_group_layout,
                            entries: &entries,
                        },
                    );
                Self(bind_group)
            }
            pub fn set<'a>(&'a self, render_pass: &mut wgpu::ComputePass<'a>) {
                render_pass.set_bind_group(0, &self.0, &[]);
            }
        }
        #[derive(Debug, Copy, Clone)]
        pub struct WgpuBindGroups<'a> {
            pub bind_group0: &'a WgpuBindGroup0,
        }
        impl<'a> WgpuBindGroups<'a> {
            pub fn set(&self, pass: &mut wgpu::ComputePass<'a>) {
                self.bind_group0.set(pass);
            }
        }
    }
    pub fn set_bind_groups<'a>(
        pass: &mut wgpu::ComputePass<'a>,
        bind_group0: &'a bind_groups::WgpuBindGroup0,
    ) {
        bind_group0.set(pass);
    }
    pub mod compute {
        pub const MAIN_WORKGROUP_SIZE: [u32; 3] = [1, 1, 1];
        pub fn create_main_pipeline_embed_source(
            device: &wgpu::Device,
        ) -> wgpu::ComputePipeline {
            let module = super::create_shader_module_embed_source(device);
            let layout = super::create_pipeline_layout(device);
            device
                .create_compute_pipeline(
                    &wgpu::ComputePipelineDescriptor {
                        label: Some("Compute Pipeline main"),
                        layout: Some(&layout),
                        module: &module,
                        entry_point: "main",
                    },
                )
        }
    }
    pub const ENTRY_MAIN: &str = "main";
    #[derive(Debug)]
    pub struct WgpuPipelineLayout;
    impl WgpuPipelineLayout {
        pub fn bind_group_layout_entries(
            entries: [wgpu::BindGroupLayout; 1],
        ) -> [wgpu::BindGroupLayout; 1] {
            entries
        }
    }
    pub fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Padding::PipelineLayout"),
                    bind_group_layouts: &[
                        &bind_groups::WgpuBindGroup0::get_bind_group_layout(device),
                    ],
                    push_constant_ranges: &[],
                },
            )
    }
    pub fn create_shader_module_embed_source(
        device: &wgpu::Device,
    ) -> wgpu::ShaderModule {
        let source = std::borrow::Cow::Borrowed(SHADER_STRING);
        device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("padding.wgsl"),
                source: wgpu::ShaderSource::Wgsl(source),
            })
    }
    pub const SHADER_STRING: &'static str = r#"
struct Style {
    color: vec4<f32>,
    width: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) 
var<storage> frame: Style;

@compute @workgroup_size(1, 1, 1) 
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    return;
}
"#;
}
pub mod bytemuck_impls {
    use super::{_root, _root::*};
    unsafe impl bytemuck::Zeroable for padding::Style {}
    unsafe impl bytemuck::Pod for padding::Style {}
}
