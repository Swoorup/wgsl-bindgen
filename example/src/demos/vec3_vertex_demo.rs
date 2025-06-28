use crate::demos::Demo;
use crate::shader_bindings::vec3_vertex_demo::{self, VertexInput};
use glam::Vec3A;
use wgpu::util::DeviceExt;
use winit::event::KeyEvent;

pub struct Vec3VertexDemo {
  vertex_buffer: wgpu::Buffer,
  render_pipeline: wgpu::RenderPipeline,
}

impl Demo for Vec3VertexDemo {
  fn new(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
  ) -> Self {
    // Create vertex data - classic RGB triangle gradient
    let vertices = &[
      VertexInput(Vec3A::new(-0.5, -0.5, 0.0), 1), // Bottom left - Red
      VertexInput(Vec3A::new(0.5, -0.5, 0.0), 2),  // Bottom right - Green
      VertexInput(Vec3A::new(0.0, 0.5, 0.0), 3),   // Top - Blue
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Vec3 Vertex Demo Vertex Buffer"),
      contents: bytemuck::cast_slice(vertices),
      usage: wgpu::BufferUsages::VERTEX,
    });

    let pipeline_layout = vec3_vertex_demo::create_pipeline_layout(device);
    let shader_module = vec3_vertex_demo::create_shader_module_embed_source(device);

    let render_pipeline =
      device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Vec3 Vertex Demo Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: vec3_vertex_demo::vertex_state(
          &shader_module,
          &vec3_vertex_demo::vs_main_entry(wgpu::VertexStepMode::Vertex),
        ),
        fragment: Some(vec3_vertex_demo::fragment_state(
          &shader_module,
          &vec3_vertex_demo::fs_main_entry([Some(wgpu::ColorTargetState {
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
    "Vec3 Vertex Demo"
  }

  fn description(&self) -> &'static str {
    "Demonstrates vec3<f32> vertex input with glam::Vec3A and proper Float32x3 format.\nShows gradient effect from linear interpolation of texture_id values.\nBuiltin vertex_index field correctly skipped in generated struct."
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
