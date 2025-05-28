#![allow(unused, non_snake_case, non_camel_case_types, non_upper_case_globals)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderEntry {
    Layouts,
}
impl ShaderEntry {
    pub fn create_pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
        match self {
            Self::Layouts => layouts::create_pipeline_layout(device),
        }
    }
    pub fn create_shader_module_embed_source(
        &self,
        device: &wgpu::Device,
    ) -> wgpu::ShaderModule {
        match self {
            Self::Layouts => layouts::create_shader_module_embed_source(device),
        }
    }
}
mod _root {
    pub use super::*;
    pub trait SetBindGroup {
        fn set_bind_group(
            &mut self,
            index: u32,
            bind_group: &wgpu::BindGroup,
            offsets: &[wgpu::DynamicOffset],
        );
    }
    impl SetBindGroup for wgpu::RenderPass<'_> {
        fn set_bind_group(
            &mut self,
            index: u32,
            bind_group: &wgpu::BindGroup,
            offsets: &[wgpu::DynamicOffset],
        ) {
            self.set_bind_group(index, bind_group, offsets);
        }
    }
    impl SetBindGroup for wgpu::RenderBundleEncoder<'_> {
        fn set_bind_group(
            &mut self,
            index: u32,
            bind_group: &wgpu::BindGroup,
            offsets: &[wgpu::DynamicOffset],
        ) {
            self.set_bind_group(index, bind_group, offsets);
        }
    }
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
    const LAYOUTS_SCALARS_ASSERTS: () = {
        assert!(std::mem::offset_of!(layouts::Scalars, a) == 0);
        assert!(std::mem::offset_of!(layouts::Scalars, b) == 4);
        assert!(std::mem::offset_of!(layouts::Scalars, c) == 8);
        assert!(std::mem::size_of:: < layouts::Scalars > () == 16);
    };
    const LAYOUTS_VECTORS_U32_ASSERTS: () = {
        assert!(std::mem::offset_of!(layouts::VectorsU32, a) == 0);
        assert!(std::mem::offset_of!(layouts::VectorsU32, b) == 16);
        assert!(std::mem::offset_of!(layouts::VectorsU32, c) == 32);
        assert!(std::mem::offset_of!(layouts::VectorsU32, _padding) == 48);
        assert!(std::mem::size_of:: < layouts::VectorsU32 > () == 64);
    };
    const LAYOUTS_VECTORS_I32_ASSERTS: () = {
        assert!(std::mem::offset_of!(layouts::VectorsI32, a) == 0);
        assert!(std::mem::offset_of!(layouts::VectorsI32, b) == 16);
        assert!(std::mem::offset_of!(layouts::VectorsI32, c) == 32);
        assert!(std::mem::size_of:: < layouts::VectorsI32 > () == 48);
    };
    const LAYOUTS_VECTORS_F32_ASSERTS: () = {
        assert!(std::mem::offset_of!(layouts::VectorsF32, a) == 0);
        assert!(std::mem::offset_of!(layouts::VectorsF32, b) == 16);
        assert!(std::mem::offset_of!(layouts::VectorsF32, c) == 32);
        assert!(std::mem::size_of:: < layouts::VectorsF32 > () == 48);
    };
    const LAYOUTS_MATRICES_F32_ASSERTS: () = {
        assert!(std::mem::offset_of!(layouts::MatricesF32, a) == 0);
        assert!(std::mem::offset_of!(layouts::MatricesF32, b) == 64);
        assert!(std::mem::offset_of!(layouts::MatricesF32, c) == 128);
        assert!(std::mem::offset_of!(layouts::MatricesF32, d) == 160);
        assert!(std::mem::offset_of!(layouts::MatricesF32, e) == 208);
        assert!(std::mem::offset_of!(layouts::MatricesF32, f) == 256);
        assert!(std::mem::offset_of!(layouts::MatricesF32, g) == 288);
        assert!(std::mem::offset_of!(layouts::MatricesF32, h) == 320);
        assert!(std::mem::offset_of!(layouts::MatricesF32, i) == 352);
        assert!(std::mem::size_of:: < layouts::MatricesF32 > () == 368);
    };
    const LAYOUTS_STATIC_ARRAYS_ASSERTS: () = {
        assert!(std::mem::offset_of!(layouts::StaticArrays, a) == 0);
        assert!(std::mem::offset_of!(layouts::StaticArrays, b) == 20);
        assert!(std::mem::offset_of!(layouts::StaticArrays, c) == 32);
        assert!(std::mem::offset_of!(layouts::StaticArrays, d) == 32800);
        assert!(std::mem::size_of:: < layouts::StaticArrays > () == 32864);
    };
    const LAYOUTS_NESTED_ASSERTS: () = {
        assert!(std::mem::offset_of!(layouts::Nested, a) == 0);
        assert!(std::mem::offset_of!(layouts::Nested, b) == 368);
        assert!(std::mem::size_of:: < layouts::Nested > () == 416);
    };
    const LAYOUTS_UNIFORMS_ASSERTS: () = {
        assert!(std::mem::offset_of!(layouts::Uniforms, color_rgb) == 0);
        assert!(std::mem::offset_of!(layouts::Uniforms, scalars) == 16);
        assert!(std::mem::size_of:: < layouts::Uniforms > () == 32);
    };
}
pub mod layouts {
    use super::{_root, _root::*};
    #[repr(C, align(4))]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct Scalars {
        /// size: 4, offset: 0x0, type: `u32`
        pub a: u32,
        /// size: 4, offset: 0x4, type: `i32`
        pub b: i32,
        /// size: 4, offset: 0x8, type: `f32`
        pub c: f32,
        pub d: [u8; 0x4],
    }
    impl Scalars {
        pub const fn new(a: u32, b: i32, c: f32) -> Self {
            Self { a, b, c, d: [0; 0x4] }
        }
    }
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct ScalarsInit {
        pub a: u32,
        pub b: i32,
        pub c: f32,
    }
    impl ScalarsInit {
        pub const fn build(&self) -> Scalars {
            Scalars {
                a: self.a,
                b: self.b,
                c: self.c,
                d: [0; 0x4],
            }
        }
    }
    impl From<ScalarsInit> for Scalars {
        fn from(data: ScalarsInit) -> Self {
            data.build()
        }
    }
    #[repr(C, align(16))]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct VectorsU32 {
        /// size: 8, offset: 0x0, type: `vec2<u32>`
        pub a: [u32; 2],
        pub _pad_a: [u8; 0x10 - ::core::mem::size_of::<[u32; 2]>()],
        /// size: 12, offset: 0x10, type: `vec3<u32>`
        pub b: [u32; 4],
        /// size: 16, offset: 0x20, type: `vec4<u32>`
        pub c: [u32; 4],
        /// size: 4, offset: 0x30, type: `f32`
        pub _padding: f32,
        pub _pad__padding: [u8; 0x10 - ::core::mem::size_of::<f32>()],
    }
    impl VectorsU32 {
        pub const fn new(a: [u32; 2], b: [u32; 4], c: [u32; 4], _padding: f32) -> Self {
            Self {
                a,
                _pad_a: [0; 0x10 - ::core::mem::size_of::<[u32; 2]>()],
                b,
                c,
                _padding,
                _pad__padding: [0; 0x10 - ::core::mem::size_of::<f32>()],
            }
        }
    }
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct VectorsU32Init {
        pub a: [u32; 2],
        pub b: [u32; 4],
        pub c: [u32; 4],
        pub _padding: f32,
    }
    impl VectorsU32Init {
        pub const fn build(&self) -> VectorsU32 {
            VectorsU32 {
                a: self.a,
                _pad_a: [0; 0x10 - ::core::mem::size_of::<[u32; 2]>()],
                b: self.b,
                c: self.c,
                _padding: self._padding,
                _pad__padding: [0; 0x10 - ::core::mem::size_of::<f32>()],
            }
        }
    }
    impl From<VectorsU32Init> for VectorsU32 {
        fn from(data: VectorsU32Init) -> Self {
            data.build()
        }
    }
    #[repr(C, align(16))]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct VectorsI32 {
        /// size: 8, offset: 0x0, type: `vec2<i32>`
        pub a: [i32; 2],
        pub _pad_a: [u8; 0x10 - ::core::mem::size_of::<[i32; 2]>()],
        /// size: 12, offset: 0x10, type: `vec3<i32>`
        pub b: [i32; 4],
        /// size: 16, offset: 0x20, type: `vec4<i32>`
        pub c: [i32; 4],
    }
    impl VectorsI32 {
        pub const fn new(a: [i32; 2], b: [i32; 4], c: [i32; 4]) -> Self {
            Self {
                a,
                _pad_a: [0; 0x10 - ::core::mem::size_of::<[i32; 2]>()],
                b,
                c,
            }
        }
    }
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct VectorsI32Init {
        pub a: [i32; 2],
        pub b: [i32; 4],
        pub c: [i32; 4],
    }
    impl VectorsI32Init {
        pub const fn build(&self) -> VectorsI32 {
            VectorsI32 {
                a: self.a,
                _pad_a: [0; 0x10 - ::core::mem::size_of::<[i32; 2]>()],
                b: self.b,
                c: self.c,
            }
        }
    }
    impl From<VectorsI32Init> for VectorsI32 {
        fn from(data: VectorsI32Init) -> Self {
            data.build()
        }
    }
    #[repr(C, align(16))]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct VectorsF32 {
        /// size: 8, offset: 0x0, type: `vec2<f32>`
        pub a: [f32; 2],
        pub _pad_a: [u8; 0x10 - ::core::mem::size_of::<[f32; 2]>()],
        /// size: 12, offset: 0x10, type: `vec3<f32>`
        pub b: glam::Vec3A,
        /// size: 16, offset: 0x20, type: `vec4<f32>`
        pub c: glam::Vec4,
    }
    impl VectorsF32 {
        pub const fn new(a: [f32; 2], b: glam::Vec3A, c: glam::Vec4) -> Self {
            Self {
                a,
                _pad_a: [0; 0x10 - ::core::mem::size_of::<[f32; 2]>()],
                b,
                c,
            }
        }
    }
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct VectorsF32Init {
        pub a: [f32; 2],
        pub b: glam::Vec3A,
        pub c: glam::Vec4,
    }
    impl VectorsF32Init {
        pub const fn build(&self) -> VectorsF32 {
            VectorsF32 {
                a: self.a,
                _pad_a: [0; 0x10 - ::core::mem::size_of::<[f32; 2]>()],
                b: self.b,
                c: self.c,
            }
        }
    }
    impl From<VectorsF32Init> for VectorsF32 {
        fn from(data: VectorsF32Init) -> Self {
            data.build()
        }
    }
    #[repr(C, align(16))]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct MatricesF32 {
        /// size: 64, offset: 0x0, type: `mat4x4<f32>`
        pub a: glam::Mat4,
        /// size: 64, offset: 0x40, type: `mat4x3<f32>`
        pub b: [[f32; 4]; 4],
        /// size: 32, offset: 0x80, type: `mat4x2<f32>`
        pub c: [[f32; 2]; 4],
        /// size: 48, offset: 0xA0, type: `mat3x4<f32>`
        pub d: [[f32; 4]; 3],
        /// size: 48, offset: 0xD0, type: `mat3x3<f32>`
        pub e: glam::Mat3A,
        /// size: 24, offset: 0x100, type: `mat3x2<f32>`
        pub f: [[f32; 2]; 3],
        pub _pad_f: [u8; 0x20 - ::core::mem::size_of::<[[f32; 2]; 3]>()],
        /// size: 32, offset: 0x120, type: `mat2x4<f32>`
        pub g: [[f32; 4]; 2],
        /// size: 32, offset: 0x140, type: `mat2x3<f32>`
        pub h: [[f32; 4]; 2],
        /// size: 16, offset: 0x160, type: `mat2x2<f32>`
        pub i: [[f32; 2]; 2],
    }
    impl MatricesF32 {
        pub const fn new(
            a: glam::Mat4,
            b: [[f32; 4]; 4],
            c: [[f32; 2]; 4],
            d: [[f32; 4]; 3],
            e: glam::Mat3A,
            f: [[f32; 2]; 3],
            g: [[f32; 4]; 2],
            h: [[f32; 4]; 2],
            i: [[f32; 2]; 2],
        ) -> Self {
            Self {
                a,
                b,
                c,
                d,
                e,
                f,
                _pad_f: [0; 0x20 - ::core::mem::size_of::<[[f32; 2]; 3]>()],
                g,
                h,
                i,
            }
        }
    }
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct MatricesF32Init {
        pub a: glam::Mat4,
        pub b: [[f32; 4]; 4],
        pub c: [[f32; 2]; 4],
        pub d: [[f32; 4]; 3],
        pub e: glam::Mat3A,
        pub f: [[f32; 2]; 3],
        pub g: [[f32; 4]; 2],
        pub h: [[f32; 4]; 2],
        pub i: [[f32; 2]; 2],
    }
    impl MatricesF32Init {
        pub const fn build(&self) -> MatricesF32 {
            MatricesF32 {
                a: self.a,
                b: self.b,
                c: self.c,
                d: self.d,
                e: self.e,
                f: self.f,
                _pad_f: [0; 0x20 - ::core::mem::size_of::<[[f32; 2]; 3]>()],
                g: self.g,
                h: self.h,
                i: self.i,
            }
        }
    }
    impl From<MatricesF32Init> for MatricesF32 {
        fn from(data: MatricesF32Init) -> Self {
            data.build()
        }
    }
    #[repr(C, align(16))]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct StaticArrays {
        /// size: 20, offset: 0x0, type: `array<u32, 5>`
        pub a: [u32; 5],
        pub _pad_a: [u8; 0x14 - ::core::mem::size_of::<[u32; 5]>()],
        /// size: 12, offset: 0x14, type: `array<f32, 3>`
        pub b: [f32; 3],
        pub _pad_b: [u8; 0xC - ::core::mem::size_of::<[f32; 3]>()],
        /// size: 32768, offset: 0x20, type: `array<mat4x4<f32>, 512>`
        pub c: [glam::Mat4; 512],
        pub _pad_c: [u8; 0x8000 - ::core::mem::size_of::<[glam::Mat4; 512]>()],
        /// size: 64, offset: 0x8020, type: `array<vec3<f32>, 4>`
        pub d: [glam::Vec3A; 4],
        pub _pad_d: [u8; 0x40 - ::core::mem::size_of::<[glam::Vec3A; 4]>()],
    }
    impl StaticArrays {
        pub const fn new(
            a: [u32; 5],
            b: [f32; 3],
            c: [glam::Mat4; 512],
            d: [glam::Vec3A; 4],
        ) -> Self {
            Self {
                a,
                _pad_a: [0; 0x14 - ::core::mem::size_of::<[u32; 5]>()],
                b,
                _pad_b: [0; 0xC - ::core::mem::size_of::<[f32; 3]>()],
                c,
                _pad_c: [0; 0x8000 - ::core::mem::size_of::<[glam::Mat4; 512]>()],
                d,
                _pad_d: [0; 0x40 - ::core::mem::size_of::<[glam::Vec3A; 4]>()],
            }
        }
    }
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct StaticArraysInit {
        pub a: [u32; 5],
        pub b: [f32; 3],
        pub c: [glam::Mat4; 512],
        pub d: [glam::Vec3A; 4],
    }
    impl StaticArraysInit {
        pub const fn build(&self) -> StaticArrays {
            StaticArrays {
                a: self.a,
                _pad_a: [0; 0x14 - ::core::mem::size_of::<[u32; 5]>()],
                b: self.b,
                _pad_b: [0; 0xC - ::core::mem::size_of::<[f32; 3]>()],
                c: self.c,
                _pad_c: [0; 0x8000 - ::core::mem::size_of::<[glam::Mat4; 512]>()],
                d: self.d,
                _pad_d: [0; 0x40 - ::core::mem::size_of::<[glam::Vec3A; 4]>()],
            }
        }
    }
    impl From<StaticArraysInit> for StaticArrays {
        fn from(data: StaticArraysInit) -> Self {
            data.build()
        }
    }
    #[repr(C, align(16))]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct Nested {
        /// size: 368, offset: 0x0, type: `struct`
        pub a: MatricesF32,
        /// size: 48, offset: 0x170, type: `struct`
        pub b: VectorsF32,
    }
    impl Nested {
        pub const fn new(a: MatricesF32, b: VectorsF32) -> Self {
            Self { a, b }
        }
    }
    #[repr(C, align(16))]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct Uniforms {
        /// size: 16, offset: 0x0, type: `vec4<f32>`
        pub color_rgb: glam::Vec4,
        /// size: 16, offset: 0x10, type: `struct`
        pub scalars: Scalars,
    }
    impl Uniforms {
        pub const fn new(color_rgb: glam::Vec4, scalars: Scalars) -> Self {
            Self { color_rgb, scalars }
        }
    }
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct VertexIn {
        pub position: glam::Vec4,
    }
    impl VertexIn {
        pub const fn new(position: glam::Vec4) -> Self {
            Self { position }
        }
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
            entry_point: Some(entry.entry_point),
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
            buffers: [VertexIn::vertex_buffer_layout(vertex_in)],
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
            entry_point: Some(entry.entry_point),
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
    pub struct WgpuBindGroup1EntriesParams<'a> {
        pub uniforms: wgpu::BufferBinding<'a>,
    }
    #[derive(Clone, Debug)]
    pub struct WgpuBindGroup1Entries<'a> {
        pub uniforms: wgpu::BindGroupEntry<'a>,
    }
    impl<'a> WgpuBindGroup1Entries<'a> {
        pub fn new(params: WgpuBindGroup1EntriesParams<'a>) -> Self {
            Self {
                uniforms: wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(params.uniforms),
                },
            }
        }
        pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 1] {
            [self.uniforms]
        }
        pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
            self.as_array().into_iter().collect()
        }
    }
    #[derive(Debug)]
    pub struct WgpuBindGroup1(wgpu::BindGroup);
    impl WgpuBindGroup1 {
        pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
            label: Some("Layouts::BindGroup1::LayoutDescriptor"),
            entries: &[
                /// @binding(0): "uniforms"
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<_root::layouts::Uniforms>() as _,
                        ),
                    },
                    count: None,
                },
            ],
        };
        pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
            device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
        }
        pub fn from_bindings(
            device: &wgpu::Device,
            bindings: WgpuBindGroup1Entries,
        ) -> Self {
            let bind_group_layout = Self::get_bind_group_layout(&device);
            let entries = bindings.as_array();
            let bind_group = device
                .create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        label: Some("Layouts::BindGroup1"),
                        layout: &bind_group_layout,
                        entries: &entries,
                    },
                );
            Self(bind_group)
        }
        pub fn set(&self, pass: &mut impl SetBindGroup) {
            pass.set_bind_group(1, &self.0, &[]);
        }
    }
    #[derive(Debug)]
    pub struct WgpuBindGroup2EntriesParams<'a> {
        pub a: wgpu::BufferBinding<'a>,
        pub b: wgpu::BufferBinding<'a>,
        pub c: wgpu::BufferBinding<'a>,
        pub d: wgpu::BufferBinding<'a>,
        pub f: wgpu::BufferBinding<'a>,
        pub h: wgpu::BufferBinding<'a>,
        pub i: wgpu::BufferBinding<'a>,
    }
    #[derive(Clone, Debug)]
    pub struct WgpuBindGroup2Entries<'a> {
        pub a: wgpu::BindGroupEntry<'a>,
        pub b: wgpu::BindGroupEntry<'a>,
        pub c: wgpu::BindGroupEntry<'a>,
        pub d: wgpu::BindGroupEntry<'a>,
        pub f: wgpu::BindGroupEntry<'a>,
        pub h: wgpu::BindGroupEntry<'a>,
        pub i: wgpu::BindGroupEntry<'a>,
    }
    impl<'a> WgpuBindGroup2Entries<'a> {
        pub fn new(params: WgpuBindGroup2EntriesParams<'a>) -> Self {
            Self {
                a: wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(params.a),
                },
                b: wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(params.b),
                },
                c: wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(params.c),
                },
                d: wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Buffer(params.d),
                },
                f: wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Buffer(params.f),
                },
                h: wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Buffer(params.h),
                },
                i: wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::Buffer(params.i),
                },
            }
        }
        pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 7] {
            [self.a, self.b, self.c, self.d, self.f, self.h, self.i]
        }
        pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
            self.as_array().into_iter().collect()
        }
    }
    #[derive(Debug)]
    pub struct WgpuBindGroup2(wgpu::BindGroup);
    impl WgpuBindGroup2 {
        pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
            label: Some("Layouts::BindGroup2::LayoutDescriptor"),
            entries: &[
                /// @binding(2): "a"
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<_root::layouts::Scalars>() as _,
                        ),
                    },
                    count: None,
                },
                /// @binding(3): "b"
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<_root::layouts::VectorsU32>() as _,
                        ),
                    },
                    count: None,
                },
                /// @binding(4): "c"
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<_root::layouts::VectorsI32>() as _,
                        ),
                    },
                    count: None,
                },
                /// @binding(5): "d"
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<_root::layouts::VectorsF32>() as _,
                        ),
                    },
                    count: None,
                },
                /// @binding(6): "f"
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<_root::layouts::MatricesF32>() as _,
                        ),
                    },
                    count: None,
                },
                /// @binding(8): "h"
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<_root::layouts::StaticArrays>() as _,
                        ),
                    },
                    count: None,
                },
                /// @binding(9): "i"
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<_root::layouts::Nested>() as _,
                        ),
                    },
                    count: None,
                },
            ],
        };
        pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
            device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
        }
        pub fn from_bindings(
            device: &wgpu::Device,
            bindings: WgpuBindGroup2Entries,
        ) -> Self {
            let bind_group_layout = Self::get_bind_group_layout(&device);
            let entries = bindings.as_array();
            let bind_group = device
                .create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        label: Some("Layouts::BindGroup2"),
                        layout: &bind_group_layout,
                        entries: &entries,
                    },
                );
            Self(bind_group)
        }
        pub fn set(&self, pass: &mut impl SetBindGroup) {
            pass.set_bind_group(2, &self.0, &[]);
        }
    }
    /// Bind groups can be set individually using their set(render_pass) method, or all at once using `WgpuBindGroups::set`.
    /// For optimal performance with many draw calls, it's recommended to organize bindings into bind groups based on update frequency:
    ///   - Bind group 0: Least frequent updates (e.g. per frame resources)
    ///   - Bind group 1: More frequent updates
    ///   - Bind group 2: More frequent updates
    ///   - Bind group 3: Most frequent updates (e.g. per draw resources)
    #[derive(Debug, Copy, Clone)]
    pub struct WgpuBindGroups<'a> {
        pub bind_group0: &'a bindings::WgpuBindGroup0,
        pub bind_group1: &'a WgpuBindGroup1,
        pub bind_group2: &'a WgpuBindGroup2,
    }
    impl<'a> WgpuBindGroups<'a> {
        pub fn set(&self, pass: &mut impl SetBindGroup) {
            self.bind_group0.set(pass);
            self.bind_group1.set(pass);
            self.bind_group2.set(pass);
        }
    }
    #[derive(Debug)]
    pub struct WgpuPipelineLayout;
    impl WgpuPipelineLayout {
        pub fn bind_group_layout_entries(
            entries: [wgpu::BindGroupLayout; 3],
        ) -> [wgpu::BindGroupLayout; 3] {
            entries
        }
    }
    pub fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Layouts::PipelineLayout"),
                    bind_group_layouts: &[
                        &bindings::WgpuBindGroup0::get_bind_group_layout(device),
                        &WgpuBindGroup1::get_bind_group_layout(device),
                        &WgpuBindGroup2::get_bind_group_layout(device),
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
                label: Some("layouts.wgsl"),
                source: wgpu::ShaderSource::Wgsl(source),
            })
    }
    pub const SHADER_STRING: &'static str = r#"
struct Scalars {
    a: u32,
    b: i32,
    c: f32,
    @builtin(vertex_index) d: u32,
}

struct VectorsU32_ {
    a: vec2<u32>,
    b: vec3<u32>,
    c: vec4<u32>,
    _padding: f32,
}

struct VectorsI32_ {
    a: vec2<i32>,
    b: vec3<i32>,
    c: vec4<i32>,
}

struct VectorsF32_ {
    a: vec2<f32>,
    b: vec3<f32>,
    c: vec4<f32>,
}

struct MatricesF32_ {
    a: mat4x4<f32>,
    b: mat4x3<f32>,
    c: mat4x2<f32>,
    d: mat3x4<f32>,
    e: mat3x3<f32>,
    f: mat3x2<f32>,
    g: mat2x4<f32>,
    h: mat2x3<f32>,
    i: mat2x2<f32>,
}

struct StaticArrays {
    a: array<u32, 5>,
    b: array<f32, 3>,
    c: array<mat4x4<f32>, 512>,
    d: array<vec3<f32>, 4>,
}

struct Nested {
    a: MatricesF32_,
    b: VectorsF32_,
}

struct Uniforms {
    color_rgb: vec4<f32>,
    scalars: Scalars,
}

struct VertexIn {
    @location(0) position: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@group(0) @binding(0) 
var color_texture: texture_2d<f32>;
@group(0) @binding(1) 
var color_sampler: sampler;
@group(1) @binding(0) 
var<uniform> uniforms: Uniforms;
@group(2) @binding(2) 
var<storage> a: Scalars;
@group(2) @binding(3) 
var<storage> b: VectorsU32_;
@group(2) @binding(4) 
var<storage> c: VectorsI32_;
@group(2) @binding(5) 
var<storage> d: VectorsF32_;
@group(2) @binding(6) 
var<storage> f: MatricesF32_;
@group(2) @binding(8) 
var<storage> h: StaticArrays;
@group(2) @binding(9) 
var<storage> i: Nested;

@vertex 
fn vertex_main(input: VertexIn) -> VertexOutput {
    var output: VertexOutput;

    output.position = input.position;
    let _e4 = output;
    return _e4;
}

@fragment 
fn fragment_main(input_1: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1f, 1f, 1f, 1f);
}
"#;
}
pub mod bytemuck_impls {
    use super::{_root, _root::*};
    unsafe impl bytemuck::Zeroable for layouts::Scalars {}
    unsafe impl bytemuck::Pod for layouts::Scalars {}
    unsafe impl bytemuck::Zeroable for layouts::VectorsU32 {}
    unsafe impl bytemuck::Pod for layouts::VectorsU32 {}
    unsafe impl bytemuck::Zeroable for layouts::VectorsI32 {}
    unsafe impl bytemuck::Pod for layouts::VectorsI32 {}
    unsafe impl bytemuck::Zeroable for layouts::VectorsF32 {}
    unsafe impl bytemuck::Pod for layouts::VectorsF32 {}
    unsafe impl bytemuck::Zeroable for layouts::MatricesF32 {}
    unsafe impl bytemuck::Pod for layouts::MatricesF32 {}
    unsafe impl bytemuck::Zeroable for layouts::StaticArrays {}
    unsafe impl bytemuck::Pod for layouts::StaticArrays {}
    unsafe impl bytemuck::Zeroable for layouts::Nested {}
    unsafe impl bytemuck::Pod for layouts::Nested {}
    unsafe impl bytemuck::Zeroable for layouts::Uniforms {}
    unsafe impl bytemuck::Pod for layouts::Uniforms {}
    unsafe impl bytemuck::Zeroable for layouts::VertexIn {}
    unsafe impl bytemuck::Pod for layouts::VertexIn {}
}
pub mod bindings {
    use super::{_root, _root::*};
    #[derive(Debug)]
    pub struct WgpuBindGroup0EntriesParams<'a> {
        pub color_texture: &'a wgpu::TextureView,
        pub color_sampler: &'a wgpu::Sampler,
    }
    #[derive(Clone, Debug)]
    pub struct WgpuBindGroup0Entries<'a> {
        pub color_texture: wgpu::BindGroupEntry<'a>,
        pub color_sampler: wgpu::BindGroupEntry<'a>,
    }
    impl<'a> WgpuBindGroup0Entries<'a> {
        pub fn new(params: WgpuBindGroup0EntriesParams<'a>) -> Self {
            Self {
                color_texture: wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(params.color_texture),
                },
                color_sampler: wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(params.color_sampler),
                },
            }
        }
        pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 2] {
            [self.color_texture, self.color_sampler]
        }
        pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
            self.as_array().into_iter().collect()
        }
    }
    #[derive(Debug)]
    pub struct WgpuBindGroup0(wgpu::BindGroup);
    impl WgpuBindGroup0 {
        pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
            label: Some("Bindings::BindGroup0::LayoutDescriptor"),
            entries: &[
                /// @binding(0): "color_texture"
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: true,
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                /// @binding(1): "color_sampler"
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        };
        pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
            device.create_bind_group_layout(&Self::LAYOUT_DESCRIPTOR)
        }
        pub fn from_bindings(
            device: &wgpu::Device,
            bindings: WgpuBindGroup0Entries,
        ) -> Self {
            let bind_group_layout = Self::get_bind_group_layout(&device);
            let entries = bindings.as_array();
            let bind_group = device
                .create_bind_group(
                    &wgpu::BindGroupDescriptor {
                        label: Some("Bindings::BindGroup0"),
                        layout: &bind_group_layout,
                        entries: &entries,
                    },
                );
            Self(bind_group)
        }
        pub fn set(&self, pass: &mut impl SetBindGroup) {
            pass.set_bind_group(0, &self.0, &[]);
        }
    }
}
