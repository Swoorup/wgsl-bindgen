use crate::shader_bindings::overlay;
use crate::simple_text;
use wgpu::util::DeviceExt;

/// Simple overlay system for displaying demo information
pub struct OverlayRenderer {
  pipeline: wgpu::RenderPipeline,
  vertex_buffer: wgpu::Buffer,
  uniform_buffer: wgpu::Buffer,
  bind_group: overlay::WgpuBindGroup0,
  text_texture: wgpu::Texture,
  text_texture_view: wgpu::TextureView,
  sampler: wgpu::Sampler,
  help_text: String,
}

impl OverlayRenderer {
  pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
    // Use generated shader module and pipeline layout
    let shader = overlay::create_shader_module_relative_path(
      device,
      crate::SHADER_DIR,
      std::collections::HashMap::new(),
      |path| std::fs::read_to_string(path),
    )
    .expect("Failed to create shader module");
    let pipeline_layout = overlay::create_pipeline_layout(device);

    let vertex_entry = overlay::vs_main_entry();
    let fragment_entry = overlay::fs_main_entry([Some(wgpu::ColorTargetState {
      format: surface_format,
      blend: Some(wgpu::BlendState::ALPHA_BLENDING),
      write_mask: wgpu::ColorWrites::ALL,
    })]);

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("overlay_pipeline"),
      layout: Some(&pipeline_layout),
      vertex: overlay::vertex_state(&shader, &vertex_entry),
      fragment: Some(overlay::fragment_state(&shader, &fragment_entry)),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleStrip,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview: None,
      cache: Default::default(),
    });

    // Create uniform buffer for overlay info
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("overlay_uniform_buffer"),
      size: 32, // 8 floats (expanded for window dimensions)
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    // Create text texture
    let text_texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("text_texture"),
      size: wgpu::Extent3d {
        width: simple_text::TEXTURE_WIDTH,
        height: simple_text::TEXTURE_HEIGHT,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8Unorm,
      usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
      view_formats: &[],
    });

    let text_texture_view =
      text_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      label: Some("text_sampler"),
      mag_filter: wgpu::FilterMode::Nearest, // Use nearest for crisp text pixels
      min_filter: wgpu::FilterMode::Nearest, // Avoid blur on high DPI
      ..Default::default()
    });

    // Create vertex buffer for a simple quad
    let vertices: [f32; 8] = [
      0.0, 0.0, // top-left
      1.0, 0.0, // top-right
      0.0, 1.0, // bottom-left
      1.0, 1.0, // bottom-right
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("overlay_vertex_buffer"),
      contents: bytemuck::cast_slice(&vertices),
      usage: wgpu::BufferUsages::VERTEX,
    });

    let bind_group_entries =
      overlay::WgpuBindGroup0Entries::new(overlay::WgpuBindGroup0EntriesParams {
        info: wgpu::BufferBinding {
          buffer: &uniform_buffer,
          offset: 0,
          size: None,
        },
        text_texture: &text_texture_view,
        text_sampler: &sampler,
      });

    let bind_group = overlay::WgpuBindGroup0::from_bindings(device, bind_group_entries);

    Self {
      pipeline,
      vertex_buffer,
      uniform_buffer,
      bind_group,
      text_texture,
      text_texture_view,
      sampler,
      help_text: String::new(),
    }
  }

  pub fn update_help_text(
    &mut self,
    _device: &wgpu::Device,
    queue: &wgpu::Queue,
    text: String,
  ) {
    // Generate new texture data
    let texture_data =
      simple_text::create_text_texture(&text, simple_text::TEXTURE_WIDTH - 20);

    self.help_text = text;

    // Update the texture
    queue.write_texture(
      wgpu::TexelCopyTextureInfo {
        texture: &self.text_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      &texture_data,
      wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(simple_text::TEXTURE_WIDTH * 4),
        rows_per_image: Some(simple_text::TEXTURE_HEIGHT),
      },
      wgpu::Extent3d {
        width: simple_text::TEXTURE_WIDTH,
        height: simple_text::TEXTURE_HEIGHT,
        depth_or_array_layers: 1,
      },
    );
  }

  pub fn update_info(
    &self,
    queue: &wgpu::Queue,
    demo_index: u32,
    total_demos: u32,
    elapsed_time: f32,
    scale_factor: f32,
    window_width: f32,
    window_height: f32,
  ) {
    let info_data = overlay::InfoData::new(
      demo_index as f32,
      total_demos as f32,
      elapsed_time,
      scale_factor,
      window_width,
      window_height,
      0.0, // padding1
      0.0, // padding2
    );
    queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[info_data]));
  }

  pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, show: bool) {
    if !show {
      return;
    }

    render_pass.set_pipeline(&self.pipeline);
    self.bind_group.set(render_pass);
    render_pass.draw(0..4, 0..1);
  }
}
