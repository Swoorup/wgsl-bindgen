use crate::demos::Demo;
use crate::shader_bindings::gradient_triangle::{self, VertexInput};
use glam::Vec3;
use wgpu::util::DeviceExt;
use winit::event::KeyEvent;

pub struct GradientTriangleDemo {
  vertex_buffer: wgpu::Buffer,
  render_pipeline: wgpu::RenderPipeline,
}

impl Demo for GradientTriangleDemo {
  fn new(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
  ) -> Self {
    // Create vertex data - classic RGB triangle gradient
    let vertices = &[
      VertexInput(Vec3::new(-0.5, -0.5, 0.0).into(), 1), // Bottom left - Red
      VertexInput(Vec3::new(0.5, -0.5, 0.0).into(), 2),  // Bottom right - Green
      VertexInput(Vec3::new(0.0, 0.5, 0.0).into(), 3),   // Top - Blue
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Gradient Triangle Vertex Buffer"),
      contents: bytemuck::cast_slice(vertices),
      usage: wgpu::BufferUsages::VERTEX,
    });

    let pipeline_layout = gradient_triangle::create_pipeline_layout(device);
    let shader_module = gradient_triangle::create_shader_module_embed_source(device);

    let render_pipeline =
      device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Gradient Triangle Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: gradient_triangle::vertex_state(
          &shader_module,
          &gradient_triangle::vs_main_entry(wgpu::VertexStepMode::Vertex),
        ),
        fragment: Some(gradient_triangle::fragment_state(
          &shader_module,
          &gradient_triangle::fs_main_entry([Some(wgpu::ColorTargetState {
            format: surface_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
          })]),
        )),
        primitive: wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::TriangleList,
          strip_index_format: None,
          front_face: wgpu::FrontFace::Ccw,
          cull_mode: Some(wgpu::Face::Back),
          unclipped_depth: false,
          polygon_mode: wgpu::PolygonMode::Fill,
          conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
          count: 1,
          mask: !0,
          alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
      });

    Self {
      vertex_buffer,
      render_pipeline,
    }
  }

  fn name(&self) -> &'static str {
    "Gradient Triangle"
  }

  fn description(&self) -> &'static str {
    "Classic RGB gradient triangle demonstrating vertex attribute interpolation.\nEach vertex has a texture_id (1=Red, 2=Green, 3=Blue) that gets\ninterpolated across the triangle to create a smooth color gradient."
  }

  fn update(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue, _elapsed_time: f32) {
    // No updates needed for this static demo
  }

  fn render<'a>(
    &'a mut self,
    _device: &wgpu::Device,
    render_pass: &mut wgpu::RenderPass<'a>,
  ) {
    render_pass.set_pipeline(&self.render_pipeline);
    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    render_pass.draw(0..3, 0..1); // Draw single triangle with gradient
  }

  fn handle_input(&mut self, _event: &KeyEvent) -> bool {
    false // No special input handling
  }

  fn get_pipeline(&self) -> &wgpu::RenderPipeline {
    &self.render_pipeline
  }
}
