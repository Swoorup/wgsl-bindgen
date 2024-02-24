#[allow(unused)]
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
    #[allow(unused_imports)]
    use super::{_root, _root::*};
    pub mod bind_groups {
        #[derive(Debug)]
        pub struct BindGroup0(wgpu::BindGroup);
        #[allow(non_snake_case)]
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
        #[allow(non_snake_case)]
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
    pub fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        let source = std::borrow::Cow::Borrowed(SHADER_STRING);
        device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(source),
            })
    }
    const SHADER_STRING: &'static str = r#"
@group(1) @binding(11) 
var<uniform> ONEX_naga_oil_mod_XMJUW4ZDJNZTXGX: f32;
@group(0) @binding(0) 
var<storage, read_write> buffer: array<f32>;

@compute @workgroup_size(1, 1, 1) 
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let _e5 = ONEX_naga_oil_mod_XMJUW4ZDJNZTXGX;
    let _e8 = buffer[id.x];
    buffer[id.x] = (_e8 * (2f * _e5));
    return;
}
"#;
}
