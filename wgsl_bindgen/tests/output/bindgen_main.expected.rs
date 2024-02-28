#![allow(unused, non_snake_case, non_camel_case_types)]
mod _root {
    pub use super::*;
    const _: () = {
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
pub mod main {
    use super::{_root, _root::*};
    #[repr(C, align(16))]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct Style {
        /// size: 16, offset: 0x0, type: `vec4<f32>`
        pub color: glam::Vec4,
        /// size: 4, offset: 0x10, type: `f32`
        pub width: f32,
        pub _pad_width: [u8; 0x10 - core::mem::size_of::<f32>()],
    }
    impl Style {
        pub const fn new(color: glam::Vec4, width: f32) -> Self {
            Self {
                color,
                width,
                _pad_width: [0; 0x10 - core::mem::size_of::<f32>()],
            }
        }
    }
    unsafe impl bytemuck::Zeroable for Style {}
    unsafe impl bytemuck::Pod for Style {}
    const _: () = {
        assert!(std::mem::offset_of!(Style, color) == 0);
        assert!(std::mem::offset_of!(Style, width) == 16);
        assert!(std::mem::size_of:: < Style > () == 32);
    };
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct StyleInit {
        pub color: glam::Vec4,
        pub width: f32,
    }
    impl StyleInit {
        pub const fn const_into(&self) -> Style {
            Style {
                color: self.color,
                width: self.width,
                _pad_width: [0; 0x10 - core::mem::size_of::<f32>()],
            }
        }
    }
    impl From<StyleInit> for Style {
        fn from(data: StyleInit) -> Self {
            data.const_into()
        }
    }
    pub mod bind_groups {
        #[derive(Debug)]
        pub struct BindGroup0(wgpu::BindGroup);
        #[derive(Debug)]
        pub struct BindGroupLayout0<'a> {
            pub buffer: wgpu::BufferBinding<'a>,
        }
        const LAYOUT_DESCRIPTOR0: wgpu::BindGroupLayoutDescriptor = wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: false,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        };
        impl BindGroup0 {
            pub fn get_bind_group_layout(
                device: &wgpu::Device,
            ) -> wgpu::BindGroupLayout {
                device.create_bind_group_layout(&LAYOUT_DESCRIPTOR0)
            }
            pub fn from_bindings(
                device: &wgpu::Device,
                bindings: BindGroupLayout0,
            ) -> Self {
                let bind_group_layout = device
                    .create_bind_group_layout(&LAYOUT_DESCRIPTOR0);
                let bind_group = device
                    .create_bind_group(
                        &wgpu::BindGroupDescriptor {
                            layout: &bind_group_layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::Buffer(bindings.buffer),
                                },
                            ],
                            label: None,
                        },
                    );
                Self(bind_group)
            }
            pub fn set<'a>(&'a self, render_pass: &mut wgpu::ComputePass<'a>) {
                render_pass.set_bind_group(0, &self.0, &[]);
            }
        }
        #[derive(Debug)]
        pub struct BindGroup1(wgpu::BindGroup);
        #[derive(Debug)]
        pub struct BindGroupLayout1<'a> {
            pub ONE: wgpu::BufferBinding<'a>,
        }
        const LAYOUT_DESCRIPTOR1: wgpu::BindGroupLayoutDescriptor = wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 11,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        };
        impl BindGroup1 {
            pub fn get_bind_group_layout(
                device: &wgpu::Device,
            ) -> wgpu::BindGroupLayout {
                device.create_bind_group_layout(&LAYOUT_DESCRIPTOR1)
            }
            pub fn from_bindings(
                device: &wgpu::Device,
                bindings: BindGroupLayout1,
            ) -> Self {
                let bind_group_layout = device
                    .create_bind_group_layout(&LAYOUT_DESCRIPTOR1);
                let bind_group = device
                    .create_bind_group(
                        &wgpu::BindGroupDescriptor {
                            layout: &bind_group_layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 11,
                                    resource: wgpu::BindingResource::Buffer(bindings.ONE),
                                },
                            ],
                            label: None,
                        },
                    );
                Self(bind_group)
            }
            pub fn set<'a>(&'a self, render_pass: &mut wgpu::ComputePass<'a>) {
                render_pass.set_bind_group(1, &self.0, &[]);
            }
        }
        #[derive(Debug, Copy, Clone)]
        pub struct BindGroups<'a> {
            pub bind_group0: &'a BindGroup0,
            pub bind_group1: &'a BindGroup1,
        }
        impl<'a> BindGroups<'a> {
            pub fn set(&self, pass: &mut wgpu::ComputePass<'a>) {
                self.bind_group0.set(pass);
                self.bind_group1.set(pass);
            }
        }
    }
    pub fn set_bind_groups<'a>(
        pass: &mut wgpu::ComputePass<'a>,
        bind_group0: &'a bind_groups::BindGroup0,
        bind_group1: &'a bind_groups::BindGroup1,
    ) {
        bind_group0.set(pass);
        bind_group1.set(pass);
    }
    pub mod compute {
        pub const MAIN_WORKGROUP_SIZE: [u32; 3] = [1, 1, 1];
        pub fn create_main_pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
            let module = super::create_shader_module(device);
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
    pub fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        &bind_groups::BindGroup0::get_bind_group_layout(device),
                        &bind_groups::BindGroup1::get_bind_group_layout(device),
                    ],
                    push_constant_ranges: &[],
                },
            )
    }
    pub fn init_composer(
        entry_dir_path: &std::path::Path,
    ) -> Result<naga_oil::compose::Composer, std::io::Error> {
        let mut composer = naga_oil::compose::Composer::default();
        let mut source_path = entry_dir_path.to_path_buf();
        source_path.push("bindings.wgsl");
        let source = std::fs::read_to_string(&source_path)?;
        composer
            .add_composable_module(naga_oil::compose::ComposableModuleDescriptor {
                source: &source,
                file_path: source_path.to_str().unwrap(),
                language: naga_oil::compose::ShaderLanguage::Wgsl,
                as_name: Some("bindings".into()),
                ..Default::default()
            })
            .expect("failed to add composer module");
        let mut source_path = entry_dir_path.to_path_buf();
        source_path.push("../additional/types.wgsl");
        let source = std::fs::read_to_string(&source_path)?;
        composer
            .add_composable_module(naga_oil::compose::ComposableModuleDescriptor {
                source: &source,
                file_path: source_path.to_str().unwrap(),
                language: naga_oil::compose::ShaderLanguage::Wgsl,
                as_name: Some("types".into()),
                ..Default::default()
            })
            .expect("failed to add composer module");
        Ok(composer)
    }
    pub fn make_naga_module(
        entry_dir_path: &std::path::Path,
        composer: &mut naga_oil::compose::Composer,
    ) -> Result<wgpu::naga::Module, std::io::Error> {
        let mut source_path = entry_dir_path.to_path_buf();
        source_path.push("main.wgsl");
        let source = std::fs::read_to_string(&source_path)?;
        let module = composer
            .make_naga_module(naga_oil::compose::NagaModuleDescriptor {
                source: &source,
                file_path: source_path.to_str().unwrap(),
                ..Default::default()
            })
            .expect("failed to build naga module");
        Ok(module)
    }
    pub fn naga_module_to_string(module: &wgpu::naga::Module) -> String {
        let info = wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::empty(),
                wgpu::naga::valid::Capabilities::all(),
            )
            .validate(&module);
        let info = info.unwrap();
        wgpu::naga::back::wgsl::write_string(
                &module,
                &info,
                wgpu::naga::back::wgsl::WriterFlags::empty(),
            )
            .expect("failed to convert naga module to source")
    }
    pub fn create_shader_module(
        entry_dir_path: &std::path::Path,
        device: &wgpu::Device,
    ) -> Result<wgpu::ShaderModule, std::io::Error> {
        let mut composer = init_composer(entry_dir_path)?;
        let module = make_naga_module(entry_dir_path, &mut composer)?;
        let source = naga_module_to_string(&module);
        let source = std::borrow::Cow::Owned(source);
        let module = device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(source),
            });
        Ok(module)
    }
}
