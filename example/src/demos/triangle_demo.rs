use super::Demo;
use crate::shader_bindings;
use wgpu::util::DeviceExt;

pub struct TriangleDemo {
    pipeline: wgpu::RenderPipeline,
    bind_group0: shader_bindings::triangle::WgpuBindGroup0,
    bind_group1: shader_bindings::triangle::WgpuBindGroup1,
    vertex_buffer: wgpu::Buffer,
    time_buffer: wgpu::Buffer,
    render_bundle: Option<wgpu::RenderBundle>,
    surface_format: wgpu::TextureFormat,
}

impl Demo for TriangleDemo {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        // Create shader and pipeline layout
        let shader = shader_bindings::triangle::create_shader_module_embed_source(device);
        let render_pipeline_layout = shader_bindings::triangle::create_pipeline_layout(device);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Triangle Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: shader_bindings::triangle::vertex_state(
                &shader,
                &shader_bindings::triangle::vs_main_entry(wgpu::VertexStepMode::Vertex),
            ),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(shader_bindings::triangle::ENTRY_FS_MAIN),
                targets: &[Some(surface_format.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: Default::default(),
        });

        // Create a simple checkerboard texture
        let texture = Self::create_checkerboard_texture(device, queue);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });

        let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("time buffer"),
            contents: bytemuck::cast_slice(&[0.0f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group0 = shader_bindings::triangle::WgpuBindGroup0::from_bindings(
            device,
            shader_bindings::triangle::WgpuBindGroup0Entries::new(
                shader_bindings::triangle::WgpuBindGroup0EntriesParams {
                    main_texture: &texture,
                    main_sampler: &sampler,
                    time: time_buffer.as_entire_buffer_binding(),
                },
            ),
        );

        let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::cast_slice(&[shader_bindings::triangle::Uniforms(
                glam::vec4(0.8, 1.0, 0.6, 1.0),
            )]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group1 = shader_bindings::triangle::WgpuBindGroup1::from_bindings(
            device,
            shader_bindings::triangle::WgpuBindGroup1Entries::new(
                shader_bindings::triangle::WgpuBindGroup1EntriesParams {
                    uniforms: uniforms_buffer.as_entire_buffer_binding(),
                },
            ),
        );

        // Create vertex buffer for fullscreen triangle
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(&[
                shader_bindings::triangle::VertexInput(glam::vec3a(-1.0, -1.0, 0.0)),
                shader_bindings::triangle::VertexInput(glam::vec3a(3.0, -1.0, 0.0)),
                shader_bindings::triangle::VertexInput(glam::vec3a(-1.0, 3.0, 0.0)),
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            pipeline,
            bind_group0,
            bind_group1,
            vertex_buffer,
            time_buffer,
            render_bundle: None,
            surface_format,
        }
    }

    fn name(&self) -> &'static str {
        "Classic Triangle"
    }

    fn description(&self) -> &'static str {
        "A classic animated triangle with texture mapping and color effects"
    }

    fn update(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue, elapsed_time: f32) {
        // Update time buffer
        queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[elapsed_time]));
    }

    fn render<'a>(&'a mut self, device: &wgpu::Device, render_pass: &mut wgpu::RenderPass<'a>) {
        // Use render bundle for better performance
        render_pass.execute_bundles(std::iter::once(self.get_render_bundle(device)));
    }

    fn get_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }
}

impl TriangleDemo {
    fn get_render_bundle(&mut self, device: &wgpu::Device) -> &wgpu::RenderBundle {
        self.render_bundle.get_or_insert_with(|| {
            let mut bundle_encoder = device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                label: Some("Triangle Render Bundle"),
                color_formats: &[Some(self.surface_format)],
                sample_count: 1,
                depth_stencil: None,
                multiview: None,
            });

            bundle_encoder.set_pipeline(&self.pipeline);
            
            // Set push constants
            let push_constant = shader_bindings::triangle::PushConstants {
                color_matrix: glam::Mat4::IDENTITY,
            };
            bundle_encoder.set_push_constants(
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                0,
                bytemuck::cast_slice(&[push_constant]),
            );

            self.bind_group0.set(&mut bundle_encoder);
            self.bind_group1.set(&mut bundle_encoder);
            bundle_encoder.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            bundle_encoder.draw(0..3, 0..1);
            
            bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
                label: Some("Triangle Render Bundle"),
            })
        })
    }

    fn create_checkerboard_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
        let size = 8;
        let data: Vec<u8> = (0..size)
            .flat_map(|y| {
                (0..size).flat_map(move |x| {
                    let is_checker = (x + y) % 2 == 0;
                    if is_checker {
                        [200, 200, 200, 255] // Light gray
                    } else {
                        [100, 100, 100, 255] // Dark gray
                    }
                })
            })
            .collect();

        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &data,
        );

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }
}