use std::time::Instant;
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

pub mod texture_array_demo;
pub mod triangle_demo;
pub mod vec3_vertex_demo;

pub use texture_array_demo::TextureArrayDemo;
pub use triangle_demo::TriangleDemo;
pub use vec3_vertex_demo::Vec3VertexDemo;

/// Trait for shader demos that can be rendered and controlled
pub trait Demo {
  /// Initialize the demo with the given device and surface format
  fn new(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
  ) -> Self
  where
    Self: Sized;

  /// Get the name of this demo for display
  fn name(&self) -> &'static str;

  /// Get the description of this demo
  fn description(&self) -> &'static str;

  /// Update the demo state (called every frame)
  fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, elapsed_time: f32);

  /// Render the demo to the given render pass
  fn render<'a>(
    &'a mut self,
    device: &wgpu::Device,
    render_pass: &mut wgpu::RenderPass<'a>,
  );

  /// Handle keyboard input (optional)
  fn handle_input(&mut self, _event: &KeyEvent) -> bool {
    false // Default: no input handled
  }

  /// Get the render pipeline for this demo
  fn get_pipeline(&self) -> &wgpu::RenderPipeline;
}

/// Manager for multiple shader demos
pub struct DemoManager {
  demos: Vec<Box<dyn Demo>>,
  current_demo: usize,
  last_switch_time: Instant,
}

impl DemoManager {
  pub fn new(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
  ) -> Self {
    // Add all available demos
    let demos: Vec<Box<dyn Demo>> = vec![
      Box::new(TriangleDemo::new(device, queue, surface_format)),
      Box::new(TextureArrayDemo::new(device, queue, surface_format)),
      Box::new(Vec3VertexDemo::new(device, queue, surface_format)),
    ];

    Self {
      demos,
      current_demo: 0,
      last_switch_time: Instant::now(),
    }
  }

  pub fn current_demo(&self) -> &dyn Demo {
    &*self.demos[self.current_demo]
  }

  pub fn current_demo_mut(&mut self) -> &mut dyn Demo {
    &mut *self.demos[self.current_demo]
  }

  pub fn current_demo_index(&self) -> usize {
    self.current_demo
  }

  pub fn demo_count(&self) -> usize {
    self.demos.len()
  }

  pub fn handle_input(&mut self, event: &KeyEvent) -> bool {
    // Check for demo switching first
    if let PhysicalKey::Code(key_code) = event.physical_key {
      // Prevent rapid switching
      if self.last_switch_time.elapsed().as_millis() < 200 {
        return true;
      }

      match key_code {
        KeyCode::ArrowLeft => {
          self.previous_demo();
          return true;
        }
        KeyCode::ArrowRight => {
          self.next_demo();
          return true;
        }
        KeyCode::Digit1 => {
          self.switch_to_demo(0);
          return true;
        }
        KeyCode::Digit2 => {
          self.switch_to_demo(1);
          return true;
        }
        KeyCode::Digit3 => {
          self.switch_to_demo(2);
          return true;
        }
        KeyCode::Digit4 => {
          self.switch_to_demo(3);
          return true;
        }
        KeyCode::Digit5 => {
          self.switch_to_demo(4);
          return true;
        }
        _ => {}
      }
    }

    // Pass input to current demo
    self.current_demo_mut().handle_input(event)
  }

  pub fn next_demo(&mut self) {
    self.current_demo = (self.current_demo + 1) % self.demos.len();
    self.last_switch_time = Instant::now();
  }

  pub fn previous_demo(&mut self) {
    self.current_demo = if self.current_demo == 0 {
      self.demos.len() - 1
    } else {
      self.current_demo - 1
    };
    self.last_switch_time = Instant::now();
  }

  pub fn switch_to_demo(&mut self, index: usize) {
    if index < self.demos.len() {
      self.current_demo = index;
      self.last_switch_time = Instant::now();
    }
  }

  pub fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    elapsed_time: f32,
  ) {
    self.current_demo_mut().update(device, queue, elapsed_time);
  }

  pub fn render<'a>(
    &'a mut self,
    device: &wgpu::Device,
    render_pass: &mut wgpu::RenderPass<'a>,
  ) {
    self.current_demo_mut().render(device, render_pass);
  }

  pub fn get_help_text(&self) -> String {
    format!(
            "Demo {}/{}: {}\n{}\n\nControls:\n< > or 1-5: Switch demos\nH: Toggle this help\nESC: Exit",
            self.current_demo + 1,
            self.demos.len(),
            self.current_demo().name(),
            self.current_demo().description()
        )
  }
}
