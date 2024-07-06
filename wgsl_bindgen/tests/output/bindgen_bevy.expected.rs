#![allow(unused, non_snake_case, non_camel_case_types, non_upper_case_globals)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderEntry {
    Pbr,
}
impl ShaderEntry {
    pub fn create_pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
        match self {
            Self::Pbr => pbr::create_pipeline_layout(device),
        }
    }
    pub fn create_shader_module_embed_source(
        &self,
        device: &wgpu::Device,
    ) -> wgpu::ShaderModule {
        match self {
            Self::Pbr => pbr::create_shader_module_embed_source(device),
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
    const BEVY_PBRPBRTYPES_STANDARD_MATERIAL_ASSERTS: () = {
        assert!(
            std::mem::offset_of!(bevy_pbr::pbr::types::StandardMaterial, base_color) == 0
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::pbr::types::StandardMaterial, emissive) == 16
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::pbr::types::StandardMaterial,
            perceptual_roughness) == 32
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::pbr::types::StandardMaterial, metallic) == 36
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::pbr::types::StandardMaterial, reflectance) ==
            40
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::pbr::types::StandardMaterial, flags) == 44
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::pbr::types::StandardMaterial, alpha_cutoff) ==
            48
        );
        assert!(std::mem::size_of:: < bevy_pbr::pbr::types::StandardMaterial > () == 64);
    };
    const BEVY_PBRMESH_VIEW_TYPES_VIEW_ASSERTS: () = {
        assert!(std::mem::offset_of!(bevy_pbr::mesh_view_types::View, view_proj) == 0);
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::View, inverse_view_proj) ==
            64
        );
        assert!(std::mem::offset_of!(bevy_pbr::mesh_view_types::View, view) == 128);
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::View, inverse_view) == 192
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::View, projection) == 256
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::View, inverse_projection) ==
            320
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::View, world_position) == 384
        );
        assert!(std::mem::offset_of!(bevy_pbr::mesh_view_types::View, width) == 396);
        assert!(std::mem::offset_of!(bevy_pbr::mesh_view_types::View, height) == 400);
        assert!(std::mem::size_of:: < bevy_pbr::mesh_view_types::View > () == 416);
    };
    const BEVY_PBRMESH_VIEW_TYPES_DIRECTIONAL_LIGHT_ASSERTS: () = {
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::DirectionalLight,
            view_projection) == 0
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::DirectionalLight, color) ==
            64
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::DirectionalLight,
            direction_to_light) == 80
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::DirectionalLight, flags) ==
            92
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::DirectionalLight,
            shadow_depth_bias) == 96
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::DirectionalLight,
            shadow_normal_bias) == 100
        );
        assert!(
            std::mem::size_of:: < bevy_pbr::mesh_view_types::DirectionalLight > () == 112
        );
    };
    const BEVY_PBRMESH_VIEW_TYPES_LIGHTS_ASSERTS: () = {
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::Lights, directional_lights)
            == 0
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::Lights, ambient_color) == 112
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::Lights, cluster_dimensions)
            == 128
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::Lights, cluster_factors) ==
            144
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::Lights, n_directional_lights)
            == 160
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::Lights,
            spot_light_shadowmap_offset) == 164
        );
        assert!(std::mem::size_of:: < bevy_pbr::mesh_view_types::Lights > () == 176);
    };
    const BEVY_PBRMESH_VIEW_TYPES_POINT_LIGHT_ASSERTS: () = {
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::PointLight,
            light_custom_data) == 0
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::PointLight,
            color_inverse_square_range) == 16
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::PointLight, position_radius)
            == 32
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::PointLight, flags) == 48
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::PointLight,
            shadow_depth_bias) == 52
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::PointLight,
            shadow_normal_bias) == 56
        );
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::PointLight,
            spot_light_tan_angle) == 60
        );
        assert!(std::mem::size_of:: < bevy_pbr::mesh_view_types::PointLight > () == 64);
    };
    const BEVY_PBRMESH_VIEW_TYPES_POINT_LIGHTS_ASSERTS: () = {
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::PointLights < 1 >, data) == 0
        );
        assert!(
            std::mem::size_of:: < bevy_pbr::mesh_view_types::PointLights < 1 > > () == 64
        );
    };
    const BEVY_PBRMESH_VIEW_TYPES_CLUSTER_LIGHT_INDEX_LISTS_ASSERTS: () = {
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::ClusterLightIndexLists < 1 >,
            data) == 0
        );
        assert!(
            std::mem::size_of:: < bevy_pbr::mesh_view_types::ClusterLightIndexLists < 1 >
            > () == 4
        );
    };
    const BEVY_PBRMESH_VIEW_TYPES_CLUSTER_OFFSETS_AND_COUNTS_ASSERTS: () = {
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_view_types::ClusterOffsetsAndCounts < 1
            >, data) == 0
        );
        assert!(
            std::mem::size_of:: < bevy_pbr::mesh_view_types::ClusterOffsetsAndCounts < 1
            > > () == 16
        );
    };
    const BEVY_PBRMESH_TYPES_MESH_ASSERTS: () = {
        assert!(std::mem::offset_of!(bevy_pbr::mesh_types::Mesh, model) == 0);
        assert!(
            std::mem::offset_of!(bevy_pbr::mesh_types::Mesh, inverse_transpose_model) ==
            64
        );
        assert!(std::mem::offset_of!(bevy_pbr::mesh_types::Mesh, flags) == 128);
        assert!(std::mem::size_of:: < bevy_pbr::mesh_types::Mesh > () == 144);
    };
}
pub mod bevy_pbr {
    use super::{_root, _root::*};
    pub mod mesh_vertex_output {
        use super::{_root, _root::*};
        #[repr(C)]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct MeshVertexOutput {
            pub world_position: glam::Vec4,
            pub world_normal: glam::Vec3A,
        }
        impl MeshVertexOutput {
            pub const fn new(
                world_position: glam::Vec4,
                world_normal: glam::Vec3A,
            ) -> Self {
                Self {
                    world_position,
                    world_normal,
                }
            }
        }
    }
    pub mod pbr {
        use super::{_root, _root::*};
        pub mod types {
            use super::{_root, _root::*};
            #[repr(C, align(16))]
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct StandardMaterial {
                /// size: 16, offset: 0x0, type: `vec4<f32>`
                pub base_color: glam::Vec4,
                /// size: 16, offset: 0x10, type: `vec4<f32>`
                pub emissive: glam::Vec4,
                /// size: 4, offset: 0x20, type: `f32`
                pub perceptual_roughness: f32,
                /// size: 4, offset: 0x24, type: `f32`
                pub metallic: f32,
                /// size: 4, offset: 0x28, type: `f32`
                pub reflectance: f32,
                /// size: 4, offset: 0x2C, type: `u32`
                pub flags: u32,
                /// size: 4, offset: 0x30, type: `f32`
                pub alpha_cutoff: f32,
                pub _pad_alpha_cutoff: [u8; 0x10 - core::mem::size_of::<f32>()],
            }
            impl StandardMaterial {
                pub const fn new(
                    base_color: glam::Vec4,
                    emissive: glam::Vec4,
                    perceptual_roughness: f32,
                    metallic: f32,
                    reflectance: f32,
                    flags: u32,
                    alpha_cutoff: f32,
                ) -> Self {
                    Self {
                        base_color,
                        emissive,
                        perceptual_roughness,
                        metallic,
                        reflectance,
                        flags,
                        alpha_cutoff,
                        _pad_alpha_cutoff: [0; 0x10 - core::mem::size_of::<f32>()],
                    }
                }
            }
            #[repr(C)]
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct StandardMaterialInit {
                pub base_color: glam::Vec4,
                pub emissive: glam::Vec4,
                pub perceptual_roughness: f32,
                pub metallic: f32,
                pub reflectance: f32,
                pub flags: u32,
                pub alpha_cutoff: f32,
            }
            impl StandardMaterialInit {
                pub const fn build(&self) -> StandardMaterial {
                    StandardMaterial {
                        base_color: self.base_color,
                        emissive: self.emissive,
                        perceptual_roughness: self.perceptual_roughness,
                        metallic: self.metallic,
                        reflectance: self.reflectance,
                        flags: self.flags,
                        alpha_cutoff: self.alpha_cutoff,
                        _pad_alpha_cutoff: [0; 0x10 - core::mem::size_of::<f32>()],
                    }
                }
            }
            impl From<StandardMaterialInit> for StandardMaterial {
                fn from(data: StandardMaterialInit) -> Self {
                    data.build()
                }
            }
            pub const STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32 = 32u32;
            pub const STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT: u32 = 16u32;
            pub const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32 = 64u32;
            pub const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32 = 128u32;
        }
    }
    pub mod mesh_view_types {
        use super::{_root, _root::*};
        #[repr(C, align(16))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct View {
            /// size: 64, offset: 0x0, type: `mat4x4<f32>`
            pub view_proj: glam::Mat4,
            /// size: 64, offset: 0x40, type: `mat4x4<f32>`
            pub inverse_view_proj: glam::Mat4,
            /// size: 64, offset: 0x80, type: `mat4x4<f32>`
            pub view: glam::Mat4,
            /// size: 64, offset: 0xC0, type: `mat4x4<f32>`
            pub inverse_view: glam::Mat4,
            /// size: 64, offset: 0x100, type: `mat4x4<f32>`
            pub projection: glam::Mat4,
            /// size: 64, offset: 0x140, type: `mat4x4<f32>`
            pub inverse_projection: glam::Mat4,
            /// size: 12, offset: 0x180, type: `vec3<f32>`
            pub world_position: glam::Vec3A,
            pub _pad_world_position: [u8; 0xC - core::mem::size_of::<glam::Vec3A>()],
            /// size: 4, offset: 0x18C, type: `f32`
            pub width: f32,
            /// size: 4, offset: 0x190, type: `f32`
            pub height: f32,
            pub _pad_height: [u8; 0x10 - core::mem::size_of::<f32>()],
        }
        impl View {
            pub const fn new(
                view_proj: glam::Mat4,
                inverse_view_proj: glam::Mat4,
                view: glam::Mat4,
                inverse_view: glam::Mat4,
                projection: glam::Mat4,
                inverse_projection: glam::Mat4,
                world_position: glam::Vec3A,
                width: f32,
                height: f32,
            ) -> Self {
                Self {
                    view_proj,
                    inverse_view_proj,
                    view,
                    inverse_view,
                    projection,
                    inverse_projection,
                    world_position,
                    _pad_world_position: [0; 0xC - core::mem::size_of::<glam::Vec3A>()],
                    width,
                    height,
                    _pad_height: [0; 0x10 - core::mem::size_of::<f32>()],
                }
            }
        }
        #[repr(C)]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct ViewInit {
            pub view_proj: glam::Mat4,
            pub inverse_view_proj: glam::Mat4,
            pub view: glam::Mat4,
            pub inverse_view: glam::Mat4,
            pub projection: glam::Mat4,
            pub inverse_projection: glam::Mat4,
            pub world_position: glam::Vec3A,
            pub width: f32,
            pub height: f32,
        }
        impl ViewInit {
            pub const fn build(&self) -> View {
                View {
                    view_proj: self.view_proj,
                    inverse_view_proj: self.inverse_view_proj,
                    view: self.view,
                    inverse_view: self.inverse_view,
                    projection: self.projection,
                    inverse_projection: self.inverse_projection,
                    world_position: self.world_position,
                    _pad_world_position: [0; 0xC - core::mem::size_of::<glam::Vec3A>()],
                    width: self.width,
                    height: self.height,
                    _pad_height: [0; 0x10 - core::mem::size_of::<f32>()],
                }
            }
        }
        impl From<ViewInit> for View {
            fn from(data: ViewInit) -> Self {
                data.build()
            }
        }
        #[repr(C, align(16))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct DirectionalLight {
            /// size: 64, offset: 0x0, type: `mat4x4<f32>`
            pub view_projection: glam::Mat4,
            /// size: 16, offset: 0x40, type: `vec4<f32>`
            pub color: glam::Vec4,
            /// size: 12, offset: 0x50, type: `vec3<f32>`
            pub direction_to_light: glam::Vec3A,
            pub _pad_direction_to_light: [u8; 0xC - core::mem::size_of::<glam::Vec3A>()],
            /// size: 4, offset: 0x5C, type: `u32`
            pub flags: u32,
            /// size: 4, offset: 0x60, type: `f32`
            pub shadow_depth_bias: f32,
            /// size: 4, offset: 0x64, type: `f32`
            pub shadow_normal_bias: f32,
            pub _pad_shadow_normal_bias: [u8; 0xC - core::mem::size_of::<f32>()],
        }
        impl DirectionalLight {
            pub const fn new(
                view_projection: glam::Mat4,
                color: glam::Vec4,
                direction_to_light: glam::Vec3A,
                flags: u32,
                shadow_depth_bias: f32,
                shadow_normal_bias: f32,
            ) -> Self {
                Self {
                    view_projection,
                    color,
                    direction_to_light,
                    _pad_direction_to_light: [0; 0xC
                        - core::mem::size_of::<glam::Vec3A>()],
                    flags,
                    shadow_depth_bias,
                    shadow_normal_bias,
                    _pad_shadow_normal_bias: [0; 0xC - core::mem::size_of::<f32>()],
                }
            }
        }
        #[repr(C)]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct DirectionalLightInit {
            pub view_projection: glam::Mat4,
            pub color: glam::Vec4,
            pub direction_to_light: glam::Vec3A,
            pub flags: u32,
            pub shadow_depth_bias: f32,
            pub shadow_normal_bias: f32,
        }
        impl DirectionalLightInit {
            pub const fn build(&self) -> DirectionalLight {
                DirectionalLight {
                    view_projection: self.view_projection,
                    color: self.color,
                    direction_to_light: self.direction_to_light,
                    _pad_direction_to_light: [0; 0xC
                        - core::mem::size_of::<glam::Vec3A>()],
                    flags: self.flags,
                    shadow_depth_bias: self.shadow_depth_bias,
                    shadow_normal_bias: self.shadow_normal_bias,
                    _pad_shadow_normal_bias: [0; 0xC - core::mem::size_of::<f32>()],
                }
            }
        }
        impl From<DirectionalLightInit> for DirectionalLight {
            fn from(data: DirectionalLightInit) -> Self {
                data.build()
            }
        }
        #[repr(C, align(16))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct Lights {
            /// size: 112, offset: 0x0, type: `array<bevy_pbr::mesh_view_types::DirectionalLight, 1>`
            pub directional_lights: [_root::bevy_pbr::mesh_view_types::DirectionalLight; 1],
            /// size: 16, offset: 0x70, type: `vec4<f32>`
            pub ambient_color: glam::Vec4,
            /// size: 16, offset: 0x80, type: `vec4<u32>`
            pub cluster_dimensions: [u32; 4],
            /// size: 16, offset: 0x90, type: `vec4<f32>`
            pub cluster_factors: glam::Vec4,
            /// size: 4, offset: 0xA0, type: `u32`
            pub n_directional_lights: u32,
            /// size: 4, offset: 0xA4, type: `i32`
            pub spot_light_shadowmap_offset: i32,
            pub _pad_spot_light_shadowmap_offset: [u8; 0xC
                - core::mem::size_of::<i32>()],
        }
        impl Lights {
            pub const fn new(
                directional_lights: [_root::bevy_pbr::mesh_view_types::DirectionalLight; 1],
                ambient_color: glam::Vec4,
                cluster_dimensions: [u32; 4],
                cluster_factors: glam::Vec4,
                n_directional_lights: u32,
                spot_light_shadowmap_offset: i32,
            ) -> Self {
                Self {
                    directional_lights,
                    ambient_color,
                    cluster_dimensions,
                    cluster_factors,
                    n_directional_lights,
                    spot_light_shadowmap_offset,
                    _pad_spot_light_shadowmap_offset: [0; 0xC
                        - core::mem::size_of::<i32>()],
                }
            }
        }
        #[repr(C)]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct LightsInit {
            pub directional_lights: [_root::bevy_pbr::mesh_view_types::DirectionalLight; 1],
            pub ambient_color: glam::Vec4,
            pub cluster_dimensions: [u32; 4],
            pub cluster_factors: glam::Vec4,
            pub n_directional_lights: u32,
            pub spot_light_shadowmap_offset: i32,
        }
        impl LightsInit {
            pub const fn build(&self) -> Lights {
                Lights {
                    directional_lights: self.directional_lights,
                    ambient_color: self.ambient_color,
                    cluster_dimensions: self.cluster_dimensions,
                    cluster_factors: self.cluster_factors,
                    n_directional_lights: self.n_directional_lights,
                    spot_light_shadowmap_offset: self.spot_light_shadowmap_offset,
                    _pad_spot_light_shadowmap_offset: [0; 0xC
                        - core::mem::size_of::<i32>()],
                }
            }
        }
        impl From<LightsInit> for Lights {
            fn from(data: LightsInit) -> Self {
                data.build()
            }
        }
        #[repr(C, align(16))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct PointLight {
            /// size: 16, offset: 0x0, type: `vec4<f32>`
            pub light_custom_data: glam::Vec4,
            /// size: 16, offset: 0x10, type: `vec4<f32>`
            pub color_inverse_square_range: glam::Vec4,
            /// size: 16, offset: 0x20, type: `vec4<f32>`
            pub position_radius: glam::Vec4,
            /// size: 4, offset: 0x30, type: `u32`
            pub flags: u32,
            /// size: 4, offset: 0x34, type: `f32`
            pub shadow_depth_bias: f32,
            /// size: 4, offset: 0x38, type: `f32`
            pub shadow_normal_bias: f32,
            /// size: 4, offset: 0x3C, type: `f32`
            pub spot_light_tan_angle: f32,
        }
        impl PointLight {
            pub const fn new(
                light_custom_data: glam::Vec4,
                color_inverse_square_range: glam::Vec4,
                position_radius: glam::Vec4,
                flags: u32,
                shadow_depth_bias: f32,
                shadow_normal_bias: f32,
                spot_light_tan_angle: f32,
            ) -> Self {
                Self {
                    light_custom_data,
                    color_inverse_square_range,
                    position_radius,
                    flags,
                    shadow_depth_bias,
                    shadow_normal_bias,
                    spot_light_tan_angle,
                }
            }
        }
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct PointLights<const N: usize> {
            /// size: 64, offset: 0x0, type: `array<bevy_pbr::mesh_view_types::PointLight>`
            pub data: [_root::bevy_pbr::mesh_view_types::PointLight; N],
        }
        impl<const N: usize> PointLights<N> {
            pub const fn new(
                data: [_root::bevy_pbr::mesh_view_types::PointLight; N],
            ) -> Self {
                Self { data }
            }
        }
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct ClusterLightIndexLists<const N: usize> {
            /// size: 4, offset: 0x0, type: `array<u32>`
            pub data: [u32; N],
        }
        impl<const N: usize> ClusterLightIndexLists<N> {
            pub const fn new(data: [u32; N]) -> Self {
                Self { data }
            }
        }
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct ClusterOffsetsAndCounts<const N: usize> {
            /// size: 16, offset: 0x0, type: `array<vec4<u32>>`
            pub data: [[u32; 4]; N],
        }
        impl<const N: usize> ClusterOffsetsAndCounts<N> {
            pub const fn new(data: [[u32; 4]; N]) -> Self {
                Self { data }
            }
        }
        pub const POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE: u32 = 2u32;
        pub const POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32 = 1u32;
        pub const DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32 = 1u32;
    }
    pub mod mesh_types {
        use super::{_root, _root::*};
        #[repr(C, align(16))]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct Mesh {
            /// size: 64, offset: 0x0, type: `mat4x4<f32>`
            pub model: glam::Mat4,
            /// size: 64, offset: 0x40, type: `mat4x4<f32>`
            pub inverse_transpose_model: glam::Mat4,
            /// size: 4, offset: 0x80, type: `u32`
            pub flags: u32,
            pub _pad_flags: [u8; 0x10 - core::mem::size_of::<u32>()],
        }
        impl Mesh {
            pub const fn new(
                model: glam::Mat4,
                inverse_transpose_model: glam::Mat4,
                flags: u32,
            ) -> Self {
                Self {
                    model,
                    inverse_transpose_model,
                    flags,
                    _pad_flags: [0; 0x10 - core::mem::size_of::<u32>()],
                }
            }
        }
        #[repr(C)]
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct MeshInit {
            pub model: glam::Mat4,
            pub inverse_transpose_model: glam::Mat4,
            pub flags: u32,
        }
        impl MeshInit {
            pub const fn build(&self) -> Mesh {
                Mesh {
                    model: self.model,
                    inverse_transpose_model: self.inverse_transpose_model,
                    flags: self.flags,
                    _pad_flags: [0; 0x10 - core::mem::size_of::<u32>()],
                }
            }
        }
        impl From<MeshInit> for Mesh {
            fn from(data: MeshInit) -> Self {
                data.build()
            }
        }
        pub const MESH_FLAGS_SHADOW_RECEIVER_BIT: u32 = 1u32;
    }
    pub mod utils {
        use super::{_root, _root::*};
        pub const PI: f32 = 3.1415927f32;
    }
}
pub mod bytemuck_impls {
    use super::{_root, _root::*};
    unsafe impl bytemuck::Zeroable for bevy_pbr::mesh_vertex_output::MeshVertexOutput {}
    unsafe impl bytemuck::Pod for bevy_pbr::mesh_vertex_output::MeshVertexOutput {}
    unsafe impl bytemuck::Zeroable for bevy_pbr::pbr::types::StandardMaterial {}
    unsafe impl bytemuck::Pod for bevy_pbr::pbr::types::StandardMaterial {}
    unsafe impl bytemuck::Zeroable for bevy_pbr::mesh_view_types::View {}
    unsafe impl bytemuck::Pod for bevy_pbr::mesh_view_types::View {}
    unsafe impl bytemuck::Zeroable for bevy_pbr::mesh_view_types::DirectionalLight {}
    unsafe impl bytemuck::Pod for bevy_pbr::mesh_view_types::DirectionalLight {}
    unsafe impl bytemuck::Zeroable for bevy_pbr::mesh_view_types::Lights {}
    unsafe impl bytemuck::Pod for bevy_pbr::mesh_view_types::Lights {}
    unsafe impl bytemuck::Zeroable for bevy_pbr::mesh_view_types::PointLight {}
    unsafe impl bytemuck::Pod for bevy_pbr::mesh_view_types::PointLight {}
    unsafe impl<const N: usize> bytemuck::Zeroable
    for bevy_pbr::mesh_view_types::PointLights<N> {}
    unsafe impl<const N: usize> bytemuck::Pod
    for bevy_pbr::mesh_view_types::PointLights<N> {}
    unsafe impl<const N: usize> bytemuck::Zeroable
    for bevy_pbr::mesh_view_types::ClusterLightIndexLists<N> {}
    unsafe impl<const N: usize> bytemuck::Pod
    for bevy_pbr::mesh_view_types::ClusterLightIndexLists<N> {}
    unsafe impl<const N: usize> bytemuck::Zeroable
    for bevy_pbr::mesh_view_types::ClusterOffsetsAndCounts<N> {}
    unsafe impl<const N: usize> bytemuck::Pod
    for bevy_pbr::mesh_view_types::ClusterOffsetsAndCounts<N> {}
    unsafe impl bytemuck::Zeroable for bevy_pbr::mesh_types::Mesh {}
    unsafe impl bytemuck::Pod for bevy_pbr::mesh_types::Mesh {}
}
pub mod pbr {
    use super::{_root, _root::*};
    pub mod bind_groups {
        use super::{_root, _root::*};
        #[derive(Debug)]
        pub struct WgpuBindGroup0EntriesParams<'a> {
            pub view: wgpu::BufferBinding<'a>,
            pub lights: wgpu::BufferBinding<'a>,
            pub point_lights: wgpu::BufferBinding<'a>,
            pub cluster_light_index_lists: wgpu::BufferBinding<'a>,
            pub cluster_offsets_and_counts: wgpu::BufferBinding<'a>,
            pub point_shadow_textures: &'a wgpu::TextureView,
            pub point_shadow_textures_sampler: &'a wgpu::Sampler,
            pub directional_shadow_textures: &'a wgpu::TextureView,
            pub directional_shadow_textures_sampler: &'a wgpu::Sampler,
        }
        #[derive(Clone, Debug)]
        pub struct WgpuBindGroup0Entries<'a> {
            pub view: wgpu::BindGroupEntry<'a>,
            pub lights: wgpu::BindGroupEntry<'a>,
            pub point_lights: wgpu::BindGroupEntry<'a>,
            pub cluster_light_index_lists: wgpu::BindGroupEntry<'a>,
            pub cluster_offsets_and_counts: wgpu::BindGroupEntry<'a>,
            pub point_shadow_textures: wgpu::BindGroupEntry<'a>,
            pub point_shadow_textures_sampler: wgpu::BindGroupEntry<'a>,
            pub directional_shadow_textures: wgpu::BindGroupEntry<'a>,
            pub directional_shadow_textures_sampler: wgpu::BindGroupEntry<'a>,
        }
        impl<'a> WgpuBindGroup0Entries<'a> {
            pub fn new(params: WgpuBindGroup0EntriesParams<'a>) -> Self {
                Self {
                    view: wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(params.view),
                    },
                    lights: wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(params.lights),
                    },
                    point_lights: wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::Buffer(params.point_lights),
                    },
                    cluster_light_index_lists: wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::Buffer(
                            params.cluster_light_index_lists,
                        ),
                    },
                    cluster_offsets_and_counts: wgpu::BindGroupEntry {
                        binding: 8,
                        resource: wgpu::BindingResource::Buffer(
                            params.cluster_offsets_and_counts,
                        ),
                    },
                    point_shadow_textures: wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            params.point_shadow_textures,
                        ),
                    },
                    point_shadow_textures_sampler: wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(
                            params.point_shadow_textures_sampler,
                        ),
                    },
                    directional_shadow_textures: wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(
                            params.directional_shadow_textures,
                        ),
                    },
                    directional_shadow_textures_sampler: wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(
                            params.directional_shadow_textures_sampler,
                        ),
                    },
                }
            }
            pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 9] {
                [
                    self.view,
                    self.lights,
                    self.point_lights,
                    self.cluster_light_index_lists,
                    self.cluster_offsets_and_counts,
                    self.point_shadow_textures,
                    self.point_shadow_textures_sampler,
                    self.directional_shadow_textures,
                    self.directional_shadow_textures_sampler,
                ]
            }
            pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
                self.as_array().into_iter().collect()
            }
        }
        #[derive(Debug)]
        pub struct WgpuBindGroup0(wgpu::BindGroup);
        impl WgpuBindGroup0 {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                label: Some("Pbr::BindGroup0::LayoutDescriptor"),
                entries: &[
                    /// @binding(0): "_root::bevy_pbr::mesh_view_bindings::view"
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                                std::mem::size_of::<
                                    _root::bevy_pbr::mesh_view_types::View,
                                >() as _,
                            ),
                        },
                        count: None,
                    },
                    /// @binding(1): "_root::bevy_pbr::mesh_view_bindings::lights"
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                                std::mem::size_of::<
                                    _root::bevy_pbr::mesh_view_types::Lights,
                                >() as _,
                            ),
                        },
                        count: None,
                    },
                    /// @binding(6): "_root::bevy_pbr::mesh_view_bindings::point_lights"
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: true,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    /// @binding(7): "_root::bevy_pbr::mesh_view_bindings::cluster_light_index_lists"
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: true,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    /// @binding(8): "_root::bevy_pbr::mesh_view_bindings::cluster_offsets_and_counts"
                    wgpu::BindGroupLayoutEntry {
                        binding: 8,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: true,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    /// @binding(2): "_root::bevy_pbr::mesh_view_bindings::point_shadow_textures"
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    /// @binding(3): "_root::bevy_pbr::mesh_view_bindings::point_shadow_textures_sampler"
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Comparison,
                        ),
                        count: None,
                    },
                    /// @binding(4): "_root::bevy_pbr::mesh_view_bindings::directional_shadow_textures"
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    /// @binding(5): "_root::bevy_pbr::mesh_view_bindings::directional_shadow_textures_sampler"
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Comparison,
                        ),
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
                bindings: WgpuBindGroup0Entries,
            ) -> Self {
                let bind_group_layout = Self::get_bind_group_layout(&device);
                let entries = bindings.as_array();
                let bind_group = device
                    .create_bind_group(
                        &wgpu::BindGroupDescriptor {
                            label: Some("Pbr::BindGroup0"),
                            layout: &bind_group_layout,
                            entries: &entries,
                        },
                    );
                Self(bind_group)
            }
            pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
                render_pass.set_bind_group(0, &self.0, &[]);
            }
        }
        #[derive(Debug)]
        pub struct WgpuBindGroup1EntriesParams<'a> {
            pub material: wgpu::BufferBinding<'a>,
        }
        #[derive(Clone, Debug)]
        pub struct WgpuBindGroup1Entries<'a> {
            pub material: wgpu::BindGroupEntry<'a>,
        }
        impl<'a> WgpuBindGroup1Entries<'a> {
            pub fn new(params: WgpuBindGroup1EntriesParams<'a>) -> Self {
                Self {
                    material: wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(params.material),
                    },
                }
            }
            pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 1] {
                [self.material]
            }
            pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
                self.as_array().into_iter().collect()
            }
        }
        #[derive(Debug)]
        pub struct WgpuBindGroup1(wgpu::BindGroup);
        impl WgpuBindGroup1 {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                label: Some("Pbr::BindGroup1::LayoutDescriptor"),
                entries: &[
                    /// @binding(0): "_root::bevy_pbr::pbr::bindings::material"
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                                std::mem::size_of::<
                                    _root::bevy_pbr::pbr::types::StandardMaterial,
                                >() as _,
                            ),
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
                bindings: WgpuBindGroup1Entries,
            ) -> Self {
                let bind_group_layout = Self::get_bind_group_layout(&device);
                let entries = bindings.as_array();
                let bind_group = device
                    .create_bind_group(
                        &wgpu::BindGroupDescriptor {
                            label: Some("Pbr::BindGroup1"),
                            layout: &bind_group_layout,
                            entries: &entries,
                        },
                    );
                Self(bind_group)
            }
            pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
                render_pass.set_bind_group(1, &self.0, &[]);
            }
        }
        #[derive(Debug)]
        pub struct WgpuBindGroup2EntriesParams<'a> {
            pub mesh: wgpu::BufferBinding<'a>,
        }
        #[derive(Clone, Debug)]
        pub struct WgpuBindGroup2Entries<'a> {
            pub mesh: wgpu::BindGroupEntry<'a>,
        }
        impl<'a> WgpuBindGroup2Entries<'a> {
            pub fn new(params: WgpuBindGroup2EntriesParams<'a>) -> Self {
                Self {
                    mesh: wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(params.mesh),
                    },
                }
            }
            pub fn as_array(self) -> [wgpu::BindGroupEntry<'a>; 1] {
                [self.mesh]
            }
            pub fn collect<B: FromIterator<wgpu::BindGroupEntry<'a>>>(self) -> B {
                self.as_array().into_iter().collect()
            }
        }
        #[derive(Debug)]
        pub struct WgpuBindGroup2(wgpu::BindGroup);
        impl WgpuBindGroup2 {
            pub const LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
                label: Some("Pbr::BindGroup2::LayoutDescriptor"),
                entries: &[
                    /// @binding(0): "_root::bevy_pbr::mesh_bindings::mesh"
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(
                                std::mem::size_of::<_root::bevy_pbr::mesh_types::Mesh>()
                                    as _,
                            ),
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
                bindings: WgpuBindGroup2Entries,
            ) -> Self {
                let bind_group_layout = Self::get_bind_group_layout(&device);
                let entries = bindings.as_array();
                let bind_group = device
                    .create_bind_group(
                        &wgpu::BindGroupDescriptor {
                            label: Some("Pbr::BindGroup2"),
                            layout: &bind_group_layout,
                            entries: &entries,
                        },
                    );
                Self(bind_group)
            }
            pub fn set<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
                render_pass.set_bind_group(2, &self.0, &[]);
            }
        }
        #[derive(Debug, Copy, Clone)]
        pub struct WgpuBindGroups<'a> {
            pub bind_group0: &'a WgpuBindGroup0,
            pub bind_group1: &'a WgpuBindGroup1,
            pub bind_group2: &'a WgpuBindGroup2,
        }
        impl<'a> WgpuBindGroups<'a> {
            pub fn set(&self, pass: &mut wgpu::RenderPass<'a>) {
                self.bind_group0.set(pass);
                self.bind_group1.set(pass);
                self.bind_group2.set(pass);
            }
        }
    }
    pub use self::bind_groups::*;
    pub fn set_bind_groups<'a>(
        pass: &mut wgpu::RenderPass<'a>,
        bind_group0: &'a bind_groups::WgpuBindGroup0,
        bind_group1: &'a bind_groups::WgpuBindGroup1,
        bind_group2: &'a bind_groups::WgpuBindGroup2,
    ) {
        bind_group0.set(pass);
        bind_group1.set(pass);
        bind_group2.set(pass);
    }
    pub const ENTRY_FRAGMENT: &str = "fragment";
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
    pub fn fragment_entry(
        targets: [Option<wgpu::ColorTargetState>; 1],
    ) -> FragmentEntry<1> {
        FragmentEntry {
            entry_point: ENTRY_FRAGMENT,
            targets,
            constants: Default::default(),
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
                    label: Some("Pbr::PipelineLayout"),
                    bind_group_layouts: &[
                        &bind_groups::WgpuBindGroup0::get_bind_group_layout(device),
                        &bind_groups::WgpuBindGroup1::get_bind_group_layout(device),
                        &bind_groups::WgpuBindGroup2::get_bind_group_layout(device),
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
                label: Some("pbr.wgsl"),
                source: wgpu::ShaderSource::Wgsl(source),
            })
    }
    pub const SHADER_STRING: &'static str = r#"
struct MeshVertexOutputX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZSXE5DFPBPW65LUOB2XIX {
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
}

struct StandardMaterialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    perceptual_roughness: f32,
    metallic: f32,
    reflectance: f32,
    flags: u32,
    alpha_cutoff: f32,
}

struct ViewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX {
    view_proj: mat4x4<f32>,
    inverse_view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    projection: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    world_position: vec3<f32>,
    width: f32,
    height: f32,
}

struct DirectionalLightX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX {
    view_projection: mat4x4<f32>,
    color: vec4<f32>,
    direction_to_light: vec3<f32>,
    flags: u32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
}

struct LightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX {
    directional_lights: array<DirectionalLightX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX, 1>,
    ambient_color: vec4<f32>,
    cluster_dimensions: vec4<u32>,
    cluster_factors: vec4<f32>,
    n_directional_lights: u32,
    spot_light_shadowmap_offset: i32,
}

struct PointLightX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX {
    light_custom_data: vec4<f32>,
    color_inverse_square_range: vec4<f32>,
    position_radius: vec4<f32>,
    flags: u32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
    spot_light_tan_angle: f32,
}

struct PointLightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX {
    data: array<PointLightX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX>,
}

struct ClusterLightIndexListsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX {
    data: array<u32>,
}

struct ClusterOffsetsAndCountsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX {
    data: array<vec4<u32>>,
}

struct MeshX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OR4XAZLTX {
    model: mat4x4<f32>,
    inverse_transpose_model: mat4x4<f32>,
    flags: u32,
}

struct PbrInputX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX {
    material: StandardMaterialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX,
    occlusion: f32,
    frag_coord: vec4<f32>,
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    is_orthographic: bool,
}

const STANDARD_MATERIAL_FLAGS_UNLIT_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX: u32 = 32u;
const STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX: u32 = 16u;
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUEX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX: u32 = 64u;
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASKX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX: u32 = 128u;
const PIX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX: f32 = 3.1415927f;
const POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVEX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX: u32 = 2u;
const MESH_FLAGS_SHADOW_RECEIVER_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OR4XAZLTX: u32 = 1u;
const POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX: u32 = 1u;
const DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX: u32 = 1u;

@group(1) @binding(0) 
var<uniform> materialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MJUW4ZDJNZTXGX: StandardMaterialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX;
@group(0) @binding(0) 
var<uniform> viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX: ViewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX;
@group(0) @binding(1) 
var<uniform> lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX: LightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX;
@group(0) @binding(6) 
var<storage> point_lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX: PointLightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX;
@group(0) @binding(7) 
var<storage> cluster_light_index_listsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX: ClusterLightIndexListsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX;
@group(0) @binding(8) 
var<storage> cluster_offsets_and_countsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX: ClusterOffsetsAndCountsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX;
@group(2) @binding(0) 
var<uniform> meshX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7MJUW4ZDJNZTXGX: MeshX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OR4XAZLTX;
@group(0) @binding(2) 
var point_shadow_texturesX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX: texture_depth_cube_array;
@group(0) @binding(3) 
var point_shadow_textures_samplerX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX: sampler_comparison;
@group(0) @binding(4) 
var directional_shadow_texturesX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX: texture_depth_2d_array;
@group(0) @binding(5) 
var directional_shadow_textures_samplerX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX: sampler_comparison;

fn standard_material_newX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX() -> StandardMaterialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX {
    var material: StandardMaterialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX;

    material.base_color = vec4<f32>(1f, 1f, 1f, 1f);
    material.emissive = vec4<f32>(0f, 0f, 0f, 1f);
    material.perceptual_roughness = 0.089f;
    material.metallic = 0.01f;
    material.reflectance = 0.5f;
    material.flags = STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUEX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX;
    material.alpha_cutoff = 0.5f;
    let _e23 = material;
    return _e23;
}

fn saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(value: f32) -> f32 {
    return clamp(value, 0f, 1f);
}

fn EnvBRDFApproxX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(f0_: vec3<f32>, perceptual_roughness_1: f32, NoV: f32) -> vec3<f32> {
    let c0_ = vec4<f32>(-1f, -0.0275f, -0.572f, 0.022f);
    let c1_ = vec4<f32>(1f, 0.0425f, 1.04f, -0.04f);
    let r = ((perceptual_roughness_1 * c0_) + c1_);
    let a004_ = ((min((r.x * r.x), exp2((-9.28f * NoV))) * r.x) + r.y);
    let AB = ((vec2<f32>(-1.04f, 1.04f) * a004_) + r.zw);
    return ((f0_ * AB.x) + vec3(AB.y));
}

fn perceptualRoughnessToRoughnessX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(perceptualRoughness: f32) -> f32 {
    let clampedPerceptualRoughness = clamp(perceptualRoughness, 0.089f, 1f);
    return (clampedPerceptualRoughness * clampedPerceptualRoughness);
}

fn luminanceX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126f, 0.7152f, 0.0722f));
}

fn change_luminanceX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(c_in: vec3<f32>, l_out: f32) -> vec3<f32> {
    let _e1 = luminanceX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(c_in);
    return (c_in * (l_out / _e1));
}

fn reinhard_luminanceX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(color: vec3<f32>) -> vec3<f32> {
    let _e1 = luminanceX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(color);
    let l_new = (_e1 / (1f + _e1));
    let _e5 = change_luminanceX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(color, l_new);
    return _e5;
}

fn getDistanceAttenuationX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(distanceSquare: f32, inverseRangeSquared: f32) -> f32 {
    let factor = (distanceSquare * inverseRangeSquared);
    let _e6 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX((1f - (factor * factor)));
    let attenuation = (_e6 * _e6);
    return ((attenuation * 1f) / max(distanceSquare, 0.0001f));
}

fn D_GGXX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(roughness: f32, NoH: f32, h: vec3<f32>) -> f32 {
    let oneMinusNoHSquared = (1f - (NoH * NoH));
    let a = (NoH * roughness);
    let k = (roughness / (oneMinusNoHSquared + (a * a)));
    let d = ((k * k) * 0.31830987f);
    return d;
}

fn V_SmithGGXCorrelatedX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(roughness_1: f32, NoV_1: f32, NoL: f32) -> f32 {
    let a2_ = (roughness_1 * roughness_1);
    let lambdaV = (NoL * sqrt((((NoV_1 - (a2_ * NoV_1)) * NoV_1) + a2_)));
    let lambdaL = (NoV_1 * sqrt((((NoL - (a2_ * NoL)) * NoL) + a2_)));
    let v_1 = (0.5f / (lambdaV + lambdaL));
    return v_1;
}

fn F_Schlick_vecX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(f0_1: vec3<f32>, f90_: f32, VoH: f32) -> vec3<f32> {
    return (f0_1 + ((vec3(f90_) - f0_1) * pow((1f - VoH), 5f)));
}

fn fresnelX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(f0_2: vec3<f32>, LoH: f32) -> vec3<f32> {
    let _e4 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(dot(f0_2, vec3(16.5f)));
    let _e6 = F_Schlick_vecX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(f0_2, _e4, LoH);
    return _e6;
}

fn specularX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(f0_3: vec3<f32>, roughness_2: f32, h_1: vec3<f32>, NoV_2: f32, NoL_1: f32, NoH_1: f32, LoH_1: f32, specularIntensity: f32) -> vec3<f32> {
    let _e3 = D_GGXX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(roughness_2, NoH_1, h_1);
    let _e6 = V_SmithGGXCorrelatedX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(roughness_2, NoV_2, NoL_1);
    let _e9 = fresnelX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(f0_3, LoH_1);
    return (((specularIntensity * _e3) * _e6) * _e9);
}

fn F_SchlickX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(f0_4: f32, f90_1: f32, VoH_1: f32) -> f32 {
    return (f0_4 + ((f90_1 - f0_4) * pow((1f - VoH_1), 5f)));
}

fn Fd_BurleyX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(roughness_3: f32, NoV_3: f32, NoL_2: f32, LoH_2: f32) -> f32 {
    let f90_2 = (0.5f + (((2f * roughness_3) * LoH_2) * LoH_2));
    let _e10 = F_SchlickX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(1f, f90_2, NoL_2);
    let _e13 = F_SchlickX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(1f, f90_2, NoV_3);
    return ((_e10 * _e13) * 0.31830987f);
}

fn point_lightX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(world_position: vec3<f32>, light: PointLightX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX, roughness_4: f32, NdotV: f32, N: vec3<f32>, V: vec3<f32>, R: vec3<f32>, F0_: vec3<f32>, diffuseColor: vec3<f32>) -> vec3<f32> {
    var L: vec3<f32>;
    var H: vec3<f32>;
    var NoL_3: f32;
    var NoH_2: f32;
    var LoH_3: f32;

    let light_to_frag = (light.position_radius.xyz - world_position.xyz);
    let distance_square = dot(light_to_frag, light_to_frag);
    let _e9 = getDistanceAttenuationX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(distance_square, light.color_inverse_square_range.w);
    let centerToRay = ((dot(light_to_frag, R) * R) - light_to_frag);
    let _e19 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX((light.position_radius.w * inverseSqrt(dot(centerToRay, centerToRay))));
    let closestPoint = (light_to_frag + (centerToRay * _e19));
    let LspecLengthInverse = inverseSqrt(dot(closestPoint, closestPoint));
    let _e31 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX((roughness_4 + ((light.position_radius.w * 0.5f) * LspecLengthInverse)));
    let normalizationFactor = (roughness_4 / _e31);
    let specularIntensity_1 = (normalizationFactor * normalizationFactor);
    L = (closestPoint * LspecLengthInverse);
    let _e37 = L;
    H = normalize((_e37 + V));
    let _e42 = L;
    let _e44 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(dot(N, _e42));
    NoL_3 = _e44;
    let _e46 = H;
    let _e48 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(dot(N, _e46));
    NoH_2 = _e48;
    let _e50 = L;
    let _e51 = H;
    let _e53 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(dot(_e50, _e51));
    LoH_3 = _e53;
    let _e55 = H;
    let _e56 = NoL_3;
    let _e57 = NoH_2;
    let _e58 = LoH_3;
    let _e61 = specularX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(F0_, roughness_4, _e55, NdotV, _e56, _e57, _e58, specularIntensity_1);
    L = normalize(light_to_frag);
    let _e63 = L;
    H = normalize((_e63 + V));
    let _e66 = L;
    let _e68 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(dot(N, _e66));
    NoL_3 = _e68;
    let _e69 = H;
    let _e71 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(dot(N, _e69));
    NoH_2 = _e71;
    let _e72 = L;
    let _e73 = H;
    let _e75 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(dot(_e72, _e73));
    LoH_3 = _e75;
    let _e76 = NoL_3;
    let _e77 = LoH_3;
    let _e78 = Fd_BurleyX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(roughness_4, NdotV, _e76, _e77);
    let diffuse = (diffuseColor * _e78);
    let _e85 = NoL_3;
    return (((diffuse + _e61) * light.color_inverse_square_range.xyz) * (_e9 * _e85));
}

fn spot_lightX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(world_position_1: vec3<f32>, light_1: PointLightX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX, roughness_5: f32, NdotV_1: f32, N_1: vec3<f32>, V_1: vec3<f32>, R_1: vec3<f32>, F0_1: vec3<f32>, diffuseColor_1: vec3<f32>) -> vec3<f32> {
    var spot_dir: vec3<f32>;

    let _e9 = point_lightX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(world_position_1, light_1, roughness_5, NdotV_1, N_1, V_1, R_1, F0_1, diffuseColor_1);
    spot_dir = vec3<f32>(light_1.light_custom_data.x, 0f, light_1.light_custom_data.y);
    let _e19 = spot_dir.x;
    let _e21 = spot_dir.x;
    let _e26 = spot_dir.z;
    let _e28 = spot_dir.z;
    spot_dir.y = sqrt(((1f - (_e19 * _e21)) - (_e26 * _e28)));
    if ((light_1.flags & POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVEX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX) != 0u) {
        let _e39 = spot_dir.y;
        spot_dir.y = -(_e39);
    }
    let light_to_frag_1 = (light_1.position_radius.xyz - world_position_1.xyz);
    let _e45 = spot_dir;
    let cd = dot(-(_e45), normalize(light_to_frag_1));
    let _e55 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(((cd * light_1.light_custom_data.z) + light_1.light_custom_data.w));
    let spot_attenuation = (_e55 * _e55);
    return (_e9 * spot_attenuation);
}

fn directional_lightX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(light_2: DirectionalLightX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX, roughness_6: f32, NdotV_2: f32, normal: vec3<f32>, view: vec3<f32>, R_2: vec3<f32>, F0_2: vec3<f32>, diffuseColor_2: vec3<f32>) -> vec3<f32> {
    let incident_light = light_2.direction_to_light.xyz;
    let half_vector = normalize((incident_light + view));
    let _e8 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(dot(normal, incident_light));
    let _e10 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(dot(normal, half_vector));
    let _e12 = saturateX_naga_oil_mod_XMJSXM6K7OBRHEOR2OV2GS3DTX(dot(incident_light, half_vector));
    let _e15 = Fd_BurleyX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(roughness_6, NdotV_2, _e8, _e12);
    let diffuse_1 = (diffuseColor_2 * _e15);
    let _e20 = specularX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(F0_2, roughness_6, half_vector, NdotV_2, _e8, _e10, _e12, 1f);
    return (((_e20 + diffuse_1) * light_2.color.xyz) * _e8);
}

fn view_z_to_z_sliceX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(view_z: f32, is_orthographic: bool) -> u32 {
    var z_slice: u32 = 0u;

    if is_orthographic {
        let _e6 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.cluster_factors.z;
        let _e11 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.cluster_factors.w;
        z_slice = u32(floor(((view_z - _e6) * _e11)));
    } else {
        let _e21 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.cluster_factors.z;
        let _e26 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.cluster_factors.w;
        z_slice = u32((((log(-(view_z)) * _e21) - _e26) + 1f));
    }
    let _e31 = z_slice;
    let _e35 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.cluster_dimensions.z;
    return min(_e31, (_e35 - 1u));
}

fn fragment_cluster_indexX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(frag_coord_1: vec2<f32>, view_z_1: f32, is_orthographic_1: bool) -> u32 {
    let _e3 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.cluster_factors;
    let xy = vec2<u32>(floor((frag_coord_1 * _e3.xy)));
    let _e10 = view_z_to_z_sliceX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(view_z_1, is_orthographic_1);
    let _e15 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.cluster_dimensions.x;
    let _e22 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.cluster_dimensions.z;
    let _e28 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.cluster_dimensions.w;
    return min(((((xy.y * _e15) + xy.x) * _e22) + _e10), (_e28 - 1u));
}

fn unpack_offset_and_countsX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(cluster_index: u32) -> vec3<u32> {
    let _e4 = cluster_offsets_and_countsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.data[cluster_index];
    return _e4.xyz;
}

fn get_light_idX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(index: u32) -> u32 {
    let _e4 = cluster_light_index_listsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.data[index];
    return _e4;
}

fn cluster_debug_visualizationX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(output_color_1: vec4<f32>, view_z_2: f32, is_orthographic_2: bool, offset_and_counts: vec3<u32>, cluster_index_1: u32) -> vec4<f32> {
    return output_color_1;
}

fn fetch_point_shadowX_naga_oil_mod_XMJSXM6K7OBRHEOR2ONUGCZDPO5ZQX(light_id: u32, frag_position: vec4<f32>, surface_normal: vec3<f32>) -> f32 {
    let light_3 = point_lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.data[light_id];
    let surface_to_light = (light_3.position_radius.xyz - frag_position.xyz);
    let surface_to_light_abs = abs(surface_to_light);
    let distance_to_light = max(surface_to_light_abs.x, max(surface_to_light_abs.y, surface_to_light_abs.z));
    let normal_offset = ((light_3.shadow_normal_bias * distance_to_light) * surface_normal.xyz);
    let depth_offset = (light_3.shadow_depth_bias * normalize(surface_to_light.xyz));
    let offset_position = ((frag_position.xyz + normal_offset) + depth_offset);
    let frag_ls = (light_3.position_radius.xyz - offset_position.xyz);
    let abs_position_ls = abs(frag_ls);
    let major_axis_magnitude = max(abs_position_ls.x, max(abs_position_ls.y, abs_position_ls.z));
    let zw = ((-(major_axis_magnitude) * light_3.light_custom_data.xy) + light_3.light_custom_data.zw);
    let depth = (zw.x / zw.y);
    let _e51 = textureSampleCompareLevel(point_shadow_texturesX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX, point_shadow_textures_samplerX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX, frag_ls, i32(light_id), depth);
    return _e51;
}

fn fetch_spot_shadowX_naga_oil_mod_XMJSXM6K7OBRHEOR2ONUGCZDPO5ZQX(light_id_1: u32, frag_position_1: vec4<f32>, surface_normal_1: vec3<f32>) -> f32 {
    var spot_dir_1: vec3<f32>;
    var sign: f32 = -1f;

    let light_4 = point_lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.data[light_id_1];
    let surface_to_light_1 = (light_4.position_radius.xyz - frag_position_1.xyz);
    spot_dir_1 = vec3<f32>(light_4.light_custom_data.x, 0f, light_4.light_custom_data.y);
    let _e20 = spot_dir_1.x;
    let _e22 = spot_dir_1.x;
    let _e27 = spot_dir_1.z;
    let _e29 = spot_dir_1.z;
    spot_dir_1.y = sqrt(((1f - (_e20 * _e22)) - (_e27 * _e29)));
    if ((light_4.flags & POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVEX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX) != 0u) {
        let _e40 = spot_dir_1.y;
        spot_dir_1.y = -(_e40);
    }
    let _e42 = spot_dir_1;
    let fwd = -(_e42);
    let distance_to_light_1 = dot(fwd, surface_to_light_1);
    let offset_position_1 = ((-(surface_to_light_1) + (light_4.shadow_depth_bias * normalize(surface_to_light_1))) + ((surface_normal_1.xyz * light_4.shadow_normal_bias) * distance_to_light_1));
    if (fwd.z >= 0f) {
        sign = 1f;
    }
    let _e62 = sign;
    let a_1 = (-1f / (fwd.z + _e62));
    let b = ((fwd.x * fwd.y) * a_1);
    let _e70 = sign;
    let _e78 = sign;
    let _e80 = sign;
    let up_dir = vec3<f32>((1f + (((_e70 * fwd.x) * fwd.x) * a_1)), (_e78 * b), (-(_e80) * fwd.x));
    let _e86 = sign;
    let right_dir = vec3<f32>(-(b), (-(_e86) - ((fwd.y * fwd.y) * a_1)), fwd.y);
    let light_inv_rot = mat3x3<f32>(right_dir, up_dir, fwd);
    let projected_position = (offset_position_1 * light_inv_rot);
    let f_div_minus_z = (1f / (light_4.spot_light_tan_angle * -(projected_position.z)));
    let shadow_xy_ndc = (projected_position.xy * f_div_minus_z);
    let shadow_uv = ((shadow_xy_ndc * vec2<f32>(0.5f, -0.5f)) + vec2<f32>(0.5f, 0.5f));
    let depth_1 = (0.1f / -(projected_position.z));
    let _e122 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.spot_light_shadowmap_offset;
    let _e124 = textureSampleCompareLevel(directional_shadow_texturesX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX, directional_shadow_textures_samplerX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX, shadow_uv, (i32(light_id_1) + _e122), depth_1);
    return _e124;
}

fn fetch_directional_shadowX_naga_oil_mod_XMJSXM6K7OBRHEOR2ONUGCZDPO5ZQX(light_id_2: u32, frag_position_2: vec4<f32>, surface_normal_2: vec3<f32>) -> f32 {
    let light_5 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.directional_lights[light_id_2];
    let normal_offset_1 = (light_5.shadow_normal_bias * surface_normal_2.xyz);
    let depth_offset_1 = (light_5.shadow_depth_bias * light_5.direction_to_light.xyz);
    let offset_position_2 = vec4<f32>(((frag_position_2.xyz + normal_offset_1) + depth_offset_1), frag_position_2.w);
    let offset_position_clip = (light_5.view_projection * offset_position_2);
    if (offset_position_clip.w <= 0f) {
        return 1f;
    }
    let offset_position_ndc = (offset_position_clip.xyz / vec3(offset_position_clip.w));
    if ((any((offset_position_ndc.xy < vec2(-1f))) || (offset_position_ndc.z < 0f)) || any((offset_position_ndc > vec3(1f)))) {
        return 1f;
    }
    let flip_correction = vec2<f32>(0.5f, -0.5f);
    let light_local = ((offset_position_ndc.xy * flip_correction) + vec2<f32>(0.5f, 0.5f));
    let depth_2 = offset_position_ndc.z;
    let _e57 = textureSampleCompareLevel(directional_shadow_texturesX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX, directional_shadow_textures_samplerX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX, light_local, i32(light_id_2), depth_2);
    return _e57;
}

fn prepare_normalX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX(standard_material_flags: u32, world_normal: vec3<f32>, is_front_1: bool) -> vec3<f32> {
    var N_2: vec3<f32>;

    N_2 = normalize(world_normal);
    if ((standard_material_flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX) != 0u) {
        if !(is_front_1) {
            let _e10 = N_2;
            N_2 = -(_e10);
        }
    }
    let _e12 = N_2;
    return _e12;
}

fn calculate_viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX(world_position_2: vec4<f32>, is_orthographic_3: bool) -> vec3<f32> {
    var V_2: vec3<f32>;

    if is_orthographic_3 {
        let _e5 = viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.view_proj[0][2];
        let _e10 = viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.view_proj[1][2];
        let _e15 = viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.view_proj[2][2];
        V_2 = normalize(vec3<f32>(_e5, _e10, _e15));
    } else {
        let _e22 = viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.world_position;
        V_2 = normalize((_e22.xyz - world_position_2.xyz));
    }
    let _e27 = V_2;
    return _e27;
}

fn pbrX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX(in: PbrInputX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX) -> vec4<f32> {
    var output_color_2: vec4<f32>;
    var light_accum: vec3<f32> = vec3(0f);
    var i: u32;
    var shadow: f32;
    var i_1: u32;
    var shadow_1: f32;
    var i_2: u32 = 0u;
    var shadow_2: f32;

    output_color_2 = in.material.base_color;
    let emissive_1 = in.material.emissive;
    let metallic_1 = in.material.metallic;
    let perceptual_roughness_2 = in.material.perceptual_roughness;
    let _e13 = perceptualRoughnessToRoughnessX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(perceptual_roughness_2);
    let occlusion_1 = in.occlusion;
    if ((in.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUEX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX) != 0u) {
        output_color_2.w = 1f;
    } else {
        if ((in.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASKX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX) != 0u) {
            let _e30 = output_color_2.w;
            if (_e30 >= in.material.alpha_cutoff) {
                output_color_2.w = 1f;
            } else {
                discard;
            }
        }
    }
    let NdotV_3 = max(dot(in.N, in.V), 0.0001f);
    let reflectance = in.material.reflectance;
    let _e49 = output_color_2;
    let F0_3 = (vec3((((0.16f * reflectance) * reflectance) * (1f - metallic_1))) + (_e49.xyz * metallic_1));
    let _e54 = output_color_2;
    let diffuse_color = (_e54.xyz * (1f - metallic_1));
    let R_3 = reflect(-(in.V), in.N);
    let _e67 = viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.inverse_view[0][2];
    let _e72 = viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.inverse_view[1][2];
    let _e77 = viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.inverse_view[2][2];
    let _e82 = viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.inverse_view[3][2];
    let view_z_3 = dot(vec4<f32>(_e67, _e72, _e77, _e82), in.world_position);
    let _e89 = fragment_cluster_indexX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(in.frag_coord.xy, view_z_3, in.is_orthographic);
    let _e90 = unpack_offset_and_countsX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(_e89);
    i = _e90.x;
    loop {
        let _e93 = i;
        if (_e93 < (_e90.x + _e90.y)) {
        } else {
            break;
        }
        {
            let _e98 = i;
            let _e99 = get_light_idX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(_e98);
            let light_6 = point_lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.data[_e99];
            shadow = 1f;
            let _e108 = meshX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7MJUW4ZDJNZTXGX.flags;
            if (((_e108 & MESH_FLAGS_SHADOW_RECEIVER_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OR4XAZLTX) != 0u) && ((light_6.flags & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX) != 0u)) {
                let _e121 = fetch_point_shadowX_naga_oil_mod_XMJSXM6K7OBRHEOR2ONUGCZDPO5ZQX(_e99, in.world_position, in.world_normal);
                shadow = _e121;
            }
            let _e126 = point_lightX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(in.world_position.xyz, light_6, _e13, NdotV_3, in.N, in.V, R_3, F0_3, diffuse_color);
            let _e128 = light_accum;
            let _e129 = shadow;
            light_accum = (_e128 + (_e126 * _e129));
        }
        continuing {
            let _e132 = i;
            i = (_e132 + 1u);
        }
    }
    i_1 = (_e90.x + _e90.y);
    loop {
        let _e139 = i_1;
        if (_e139 < ((_e90.x + _e90.y) + _e90.z)) {
        } else {
            break;
        }
        {
            let _e146 = i_1;
            let _e147 = get_light_idX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(_e146);
            let light_7 = point_lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.data[_e147];
            shadow_1 = 1f;
            let _e156 = meshX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7MJUW4ZDJNZTXGX.flags;
            if (((_e156 & MESH_FLAGS_SHADOW_RECEIVER_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OR4XAZLTX) != 0u) && ((light_7.flags & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX) != 0u)) {
                let _e169 = fetch_spot_shadowX_naga_oil_mod_XMJSXM6K7OBRHEOR2ONUGCZDPO5ZQX(_e147, in.world_position, in.world_normal);
                shadow_1 = _e169;
            }
            let _e174 = spot_lightX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(in.world_position.xyz, light_7, _e13, NdotV_3, in.N, in.V, R_3, F0_3, diffuse_color);
            let _e175 = light_accum;
            let _e176 = shadow_1;
            light_accum = (_e175 + (_e174 * _e176));
        }
        continuing {
            let _e179 = i_1;
            i_1 = (_e179 + 1u);
        }
    }
    let n_directional_lights = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.n_directional_lights;
    loop {
        let _e186 = i_2;
        if (_e186 < n_directional_lights) {
        } else {
            break;
        }
        {
            let _e190 = i_2;
            let light_8 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.directional_lights[_e190];
            shadow_2 = 1f;
            let _e197 = meshX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7MJUW4ZDJNZTXGX.flags;
            if (((_e197 & MESH_FLAGS_SHADOW_RECEIVER_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OR4XAZLTX) != 0u) && ((light_8.flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527OR4XAZLTX) != 0u)) {
                let _e208 = i_2;
                let _e211 = fetch_directional_shadowX_naga_oil_mod_XMJSXM6K7OBRHEOR2ONUGCZDPO5ZQX(_e208, in.world_position, in.world_normal);
                shadow_2 = _e211;
            }
            let _e214 = directional_lightX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(light_8, _e13, NdotV_3, in.N, in.V, R_3, F0_3, diffuse_color);
            let _e215 = light_accum;
            let _e216 = shadow_2;
            light_accum = (_e215 + (_e214 * _e216));
        }
        continuing {
            let _e219 = i_2;
            i_2 = (_e219 + 1u);
        }
    }
    let _e223 = EnvBRDFApproxX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(diffuse_color, 1f, NdotV_3);
    let _e224 = EnvBRDFApproxX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(F0_3, perceptual_roughness_2, NdotV_3);
    let _e225 = light_accum;
    let _e229 = lightsX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.ambient_color;
    let _e236 = output_color_2.w;
    let _e240 = output_color_2.w;
    output_color_2 = vec4<f32>(((_e225 + (((_e223 + _e224) * _e229.xyz) * occlusion_1)) + (emissive_1.xyz * _e236)), _e240);
    let _e242 = output_color_2;
    let _e244 = cluster_debug_visualizationX_naga_oil_mod_XMJSXM6K7OBRHEOR2MNWHK43UMVZGKZC7MZXXE53BOJSAX(_e242, view_z_3, in.is_orthographic, _e90, _e89);
    output_color_2 = _e244;
    let _e245 = output_color_2;
    return _e245;
}

fn tone_mappingX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX(in_1: vec4<f32>) -> vec4<f32> {
    let _e2 = reinhard_luminanceX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2NRUWO2DUNFXGOX(in_1.xyz);
    return vec4<f32>(_e2, in_1.w);
}

@fragment 
fn fragment(mesh: MeshVertexOutputX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZSXE5DFPBPW65LUOB2XIX, @builtin(front_facing) is_front: bool, @builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    var output_color: vec4<f32>;
    var pbr_input: PbrInputX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX;
    var emissive: vec4<f32>;
    var metallic: f32;
    var perceptual_roughness: f32;
    var occlusion: f32 = 1f;

    let _e3 = materialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MJUW4ZDJNZTXGX.base_color;
    output_color = _e3;
    let _e7 = materialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MJUW4ZDJNZTXGX.flags;
    if ((_e7 & STANDARD_MATERIAL_FLAGS_UNLIT_BITX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2OR4XAZLTX) == 0u) {
        let _e15 = output_color;
        pbr_input.material.base_color = _e15;
        let _e20 = materialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MJUW4ZDJNZTXGX.reflectance;
        pbr_input.material.reflectance = _e20;
        let _e25 = materialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MJUW4ZDJNZTXGX.flags;
        pbr_input.material.flags = _e25;
        let _e30 = materialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MJUW4ZDJNZTXGX.alpha_cutoff;
        pbr_input.material.alpha_cutoff = _e30;
        let _e33 = materialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MJUW4ZDJNZTXGX.emissive;
        emissive = _e33;
        let _e37 = emissive;
        pbr_input.material.emissive = _e37;
        let _e40 = materialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MJUW4ZDJNZTXGX.metallic;
        metallic = _e40;
        let _e44 = materialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MJUW4ZDJNZTXGX.perceptual_roughness;
        perceptual_roughness = _e44;
        let _e48 = metallic;
        pbr_input.material.metallic = _e48;
        let _e51 = perceptual_roughness;
        pbr_input.material.perceptual_roughness = _e51;
        let _e54 = occlusion;
        pbr_input.occlusion = _e54;
        pbr_input.frag_coord = frag_coord;
        pbr_input.world_position = mesh.world_position;
        pbr_input.world_normal = mesh.world_normal;
        let _e67 = viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2NVSXG2C7OZUWK527MJUW4ZDJNZTXGX.projection[3][3];
        pbr_input.is_orthographic = (_e67 == 1f);
        let _e73 = materialX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MJUW4ZDJNZTXGX.flags;
        let _e76 = prepare_normalX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX(_e73, mesh.world_normal, is_front);
        pbr_input.N = _e76;
        let _e80 = pbr_input.is_orthographic;
        let _e81 = calculate_viewX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX(mesh.world_position, _e80);
        pbr_input.V = _e81;
        let _e82 = pbr_input;
        let _e83 = pbrX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX(_e82);
        let _e84 = tone_mappingX_naga_oil_mod_XMJSXM6K7OBRHEOR2OBRHEOR2MZ2W4Y3UNFXW44YX(_e83);
        output_color = _e84;
    }
    let _e85 = output_color;
    return _e85;
}
"#;
}
