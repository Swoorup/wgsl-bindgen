use super::Demo;
use crate::shader_bindings;
use std::time::Instant;
use wgpu::util::DeviceExt;

pub struct TextureArrayDemo {
  pipeline: wgpu::RenderPipeline,
  bind_group1: shader_bindings::simple_array_demo::WgpuBindGroup1,
  bind_group2: shader_bindings::simple_array_demo::WgpuBindGroup2,
  vertex_buffer: wgpu::Buffer,
  textures: Vec<wgpu::TextureView>,
  samplers: Vec<wgpu::Sampler>,
  start_time: Instant,
  surface_format: wgpu::TextureFormat,
}

impl Demo for TextureArrayDemo {
  fn new(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
  ) -> Self {
    // Create shader and pipeline layout
    let shader = shader_bindings::simple_array_demo::create_shader_module_relative_path(
      device,
      crate::SHADER_DIR,
      std::collections::HashMap::new(),
      |path| std::fs::read_to_string(path),
    )
    .expect("Failed to create shader module");
    let render_pipeline_layout =
      shader_bindings::simple_array_demo::create_pipeline_layout(device);

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("TextureArray Render Pipeline"),
      layout: Some(&render_pipeline_layout),
      vertex: shader_bindings::simple_array_demo::vertex_state(
        &shader,
        &shader_bindings::simple_array_demo::vs_main_entry(wgpu::VertexStepMode::Vertex),
      ),
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some(shader_bindings::simple_array_demo::ENTRY_FS_MAIN),
        targets: &[Some(surface_format.into())],
        compilation_options: Default::default(),
      }),
      primitive: wgpu::PrimitiveState::default(),
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview: None,
      cache: Default::default(),
    });

    // Create multiple interesting textures for the array
    let textures = Self::create_texture_array(device, queue);
    let samplers = Self::create_sampler_array(device);

    // Create texture and sampler arrays for the bind group
    let texture_views: Vec<&wgpu::TextureView> = textures.iter().collect();
    let sampler_refs: Vec<&wgpu::Sampler> = samplers.iter().collect();

    let texture_2d_array = device.create_texture_with_data(
      queue,
      &wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
          width: 1024,
          height: 1024,
          depth_or_array_layers: 2,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
      },
      wgpu::wgt::TextureDataOrder::LayerMajor,
      &[0x80; 1024 * 1024 * 2 * 4],
    );

    // Use the generated types with texture arrays
    let bind_group1 = shader_bindings::simple_array_demo::WgpuBindGroup1::from_bindings(
      device,
      shader_bindings::simple_array_demo::WgpuBindGroup1Entries::new(
        shader_bindings::simple_array_demo::WgpuBindGroup1EntriesParams {
          texture_array: &texture_views[..],
          sampler_array: &sampler_refs[..],
          texture_array_no_bind: &texture_2d_array
            .create_view(&wgpu::TextureViewDescriptor::default()),
        },
      ),
    );

    let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("uniforms"),
      contents: bytemuck::cast_slice(&[shader_bindings::simple_array_demo::Uniforms(
        glam::vec4(1.0, 1.0, 1.0, 1.0),
      )]),
      usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group2 = shader_bindings::simple_array_demo::WgpuBindGroup2::from_bindings(
      device,
      shader_bindings::simple_array_demo::WgpuBindGroup2Entries::new(
        shader_bindings::simple_array_demo::WgpuBindGroup2EntriesParams {
          uniforms: uniforms_buffer.as_entire_buffer_binding(),
        },
      ),
    );

    // Initialize the vertex buffer based on the expected input structs.
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("vertex buffer"),
      contents: bytemuck::cast_slice(&[
        shader_bindings::simple_array_demo::VertexInput(glam::vec3(-1.0, -1.0, 0.0)),
        shader_bindings::simple_array_demo::VertexInput(glam::vec3(3.0, -1.0, 0.0)),
        shader_bindings::simple_array_demo::VertexInput(glam::vec3(-1.0, 3.0, 0.0)),
      ]),
      usage: wgpu::BufferUsages::VERTEX,
    });

    Self {
      pipeline,
      bind_group1,
      bind_group2,
      vertex_buffer,
      textures,
      samplers,
      start_time: Instant::now(),
      surface_format,
    }
  }

  fn name(&self) -> &'static str {
    "TextureArray Demo"
  }

  fn description(&self) -> &'static str {
    "Advanced demo showcasing texture arrays with animated blending and effects"
  }

  fn update(
    &mut self,
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    _context: super::DemoContext,
  ) {
    // No updates needed for this static demo
  }

  fn render<'a>(
    &'a mut self,
    _device: &wgpu::Device,
    render_pass: &mut wgpu::RenderPass<'a>,
  ) {
    render_pass.set_pipeline(&self.pipeline);

    // Push constant data also needs to follow alignment rules.
    let push_constant = shader_bindings::simple_array_demo::PushConstants {
      color_matrix: glam::Mat4::IDENTITY,
    };

    render_pass.set_push_constants(
      wgpu::ShaderStages::VERTEX_FRAGMENT,
      0,
      bytemuck::cast_slice(&[push_constant]),
    );

    // Global bind group is already set by main.rs, only set shader-specific bind groups
    self.bind_group1.set(render_pass);
    self.bind_group2.set(render_pass);

    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    render_pass.draw(0..3, 0..1);
  }

  fn get_pipeline(&self) -> &wgpu::RenderPipeline {
    &self.pipeline
  }
}

impl TextureArrayDemo {
  fn create_texture_array(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
  ) -> Vec<wgpu::TextureView> {
    vec![
      // Texture 1: Blue-purple gradient
      Self::create_texture_with_pattern(device, queue, |x, y| {
        let r = (x as f32 / 4.0) * 128.0;
        let g = (y as f32 / 4.0) * 64.0;
        let b = 255.0 - (x as f32 / 4.0) * 128.0;
        [r as u8, g as u8, b as u8, 255]
      }),
      // Texture 2: Checkerboard pattern
      Self::create_texture_with_pattern(device, queue, |x, y| {
        let is_checker = (x + y) % 2 == 0;
        if is_checker {
          [255, 128, 64, 255] // Orange
        } else {
          [64, 128, 255, 255] // Blue
        }
      }),
    ]
  }

  fn create_texture_with_pattern<F>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pattern_fn: F,
  ) -> wgpu::TextureView
  where
    F: Fn(u32, u32) -> [u8; 4] + Clone,
  {
    let size = 8;
    let data: Vec<u8> = (0..size)
      .flat_map(|y| {
        let pattern_fn = pattern_fn.clone();
        (0..size).flat_map(move |x| pattern_fn(x, y))
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

  fn create_sampler_array(device: &wgpu::Device) -> Vec<wgpu::Sampler> {
    vec![
      // Linear sampler
      device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
      }),
      // Nearest sampler
      device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
      }),
    ]
  }
}
