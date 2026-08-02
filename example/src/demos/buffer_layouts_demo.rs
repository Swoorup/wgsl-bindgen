use super::{Demo, DemoContext};
use crate::buffer_layout_bindings::buffer_layouts::{
  self, FixedLayout, RuntimeLayout, WgpuBindGroup0Entries, WgpuBindGroup0EntriesParams,
};
use wgpu::util::DeviceExt;
use winit::{event::KeyEvent, keyboard::KeyCode, keyboard::PhysicalKey};

const MIN_RUNTIME_COLORS: usize = 1;
const MAX_RUNTIME_COLORS: usize = 8;

const RUNTIME_COLORS: [[f32; 4]; MAX_RUNTIME_COLORS] = [
  [0.10, 0.78, 0.90, 1.0],
  [0.92, 0.20, 0.72, 1.0],
  [0.96, 0.82, 0.18, 1.0],
  [0.30, 0.86, 0.38, 1.0],
  [0.98, 0.48, 0.18, 1.0],
  [0.38, 0.42, 1.00, 1.0],
  [0.84, 0.28, 0.98, 1.0],
  [0.12, 0.68, 0.58, 1.0],
];

pub struct BufferLayoutsDemo {
  pipeline: wgpu::RenderPipeline,
  bind_group: buffer_layouts::WgpuBindGroup0,
  uniform_buffer: wgpu::Buffer,
  direct_buffer: wgpu::Buffer,
  array_buffer: wgpu::Buffer,
  runtime_buffer: wgpu::Buffer,
  runtime_header: FixedLayout,
  runtime_colors: Vec<[f32; 4]>,
  runtime_array_dirty: bool,
}

impl Demo for BufferLayoutsDemo {
  fn new(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
  ) -> Self {
    let uniform =
      fixed_layout(rgb(0.95, 0.18, 0.22), [rgb(1.0, 0.68, 0.2), rgb(1.0, 0.9, 0.55)], 0);
    let direct = fixed_layout(
      rgb(0.12, 0.76, 0.46),
      [rgb(0.2, 1.0, 0.72), rgb(0.05, 0.45, 0.30)],
      1,
    );
    let array = [
      fixed_layout(
        rgb(0.18, 0.42, 0.95),
        [rgb(0.38, 0.72, 1.0), rgb(0.12, 0.22, 0.65)],
        2,
      ),
      fixed_layout(
        rgb(1.0, 0.44, 0.12),
        [rgb(1.0, 0.72, 0.25), rgb(0.75, 0.18, 0.08)],
        3,
      ),
    ];
    let runtime_header = fixed_layout(
      rgb(0.55, 0.24, 0.9),
      [rgb(0.78, 0.52, 1.0), rgb(0.30, 0.12, 0.58)],
      3,
    );
    let runtime_colors = RUNTIME_COLORS[..3].to_vec();
    let runtime = RuntimeLayout::new(runtime_header, &runtime_colors);

    let uniform_buffer = buffer(
      device,
      "Layout uniform",
      bytemuck::bytes_of(&uniform),
      wgpu::BufferUsages::UNIFORM,
    );
    let direct_buffer = buffer(
      device,
      "Direct layout",
      bytemuck::bytes_of(&direct),
      wgpu::BufferUsages::STORAGE,
    );
    let array_buffer = buffer(
      device,
      "Layout array",
      bytemuck::cast_slice(&array),
      wgpu::BufferUsages::STORAGE,
    );
    let runtime_buffer =
      buffer(device, "Runtime layout", runtime.as_bytes(), wgpu::BufferUsages::STORAGE);
    let bind_group =
      bind_group(device, &uniform_buffer, &direct_buffer, &array_buffer, &runtime_buffer);

    let shader = buffer_layouts::create_shader_module_embed_source(device);
    let pipeline_layout = buffer_layouts::create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Buffer layouts pipeline"),
      layout: Some(&pipeline_layout),
      vertex: buffer_layouts::vertex_state(&shader, &buffer_layouts::vs_main_entry()),
      fragment: Some(buffer_layouts::fragment_state(
        &shader,
        &buffer_layouts::fs_main_entry([Some(wgpu::ColorTargetState {
          format: surface_format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::ALL,
        })]),
      )),
      primitive: wgpu::PrimitiveState::default(),
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview_mask: None,
      cache: None,
    });

    Self {
      pipeline,
      bind_group,
      uniform_buffer,
      direct_buffer,
      array_buffer,
      runtime_buffer,
      runtime_header,
      runtime_colors,
      runtime_array_dirty: false,
    }
  }

  fn name(&self) -> &'static str {
    "Buffer Layouts"
  }

  fn description(&self) -> &'static str {
    "Uniform and direct storage on top; fixed and runtime arrays below."
  }

  fn update(
    &mut self,
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    _context: DemoContext,
  ) {
    if !self.runtime_array_dirty {
      return;
    }

    self.runtime_header.tag = self.runtime_colors.len() as u32;
    let runtime = RuntimeLayout::new(self.runtime_header, &self.runtime_colors);
    self.runtime_buffer =
      buffer(device, "Runtime layout", runtime.as_bytes(), wgpu::BufferUsages::STORAGE);
    self.bind_group = bind_group(
      device,
      &self.uniform_buffer,
      &self.direct_buffer,
      &self.array_buffer,
      &self.runtime_buffer,
    );
    self.runtime_array_dirty = false;
  }

  fn render<'a>(
    &'a mut self,
    _device: &wgpu::Device,
    render_pass: &mut wgpu::RenderPass<'a>,
  ) {
    render_pass.set_pipeline(&self.pipeline);
    self.bind_group.set(render_pass);
    render_pass.draw(0..3, 0..1);
  }

  fn handle_input(&mut self, event: &KeyEvent) -> bool {
    let PhysicalKey::Code(key_code) = event.physical_key else {
      return false;
    };

    let resized = resize_runtime_colors(&mut self.runtime_colors, key_code);
    self.runtime_array_dirty |= resized;
    resized
  }

  fn controls(&self) -> Option<&'static str> {
    Some("+ / -: Resize the lower-right runtime color array")
  }

  fn get_pipeline(&self) -> &wgpu::RenderPipeline {
    &self.pipeline
  }
}

fn rgb(red: f32, green: f32, blue: f32) -> [f32; 4] {
  [red, green, blue, 0.0]
}

fn fixed_layout(primary: [f32; 4], accents: [[f32; 4]; 2], tag: u32) -> FixedLayout {
  FixedLayout::new(primary, accents, tag)
}

fn buffer(
  device: &wgpu::Device,
  label: &str,
  contents: &[u8],
  usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
  device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some(label),
    contents,
    usage,
  })
}

fn bind_group(
  device: &wgpu::Device,
  uniform_buffer: &wgpu::Buffer,
  direct_buffer: &wgpu::Buffer,
  array_buffer: &wgpu::Buffer,
  runtime_buffer: &wgpu::Buffer,
) -> buffer_layouts::WgpuBindGroup0 {
  buffer_layouts::WgpuBindGroup0::from_bindings(
    device,
    WgpuBindGroup0Entries::new(WgpuBindGroup0EntriesParams {
      uniform_layout: uniform_buffer.as_entire_buffer_binding(),
      direct_layout: direct_buffer.as_entire_buffer_binding(),
      layout_array: array_buffer.as_entire_buffer_binding(),
      runtime_layout: runtime_buffer.as_entire_buffer_binding(),
    }),
  )
}

fn resize_runtime_colors(colors: &mut Vec<[f32; 4]>, key_code: KeyCode) -> bool {
  match key_code {
    KeyCode::Equal | KeyCode::NumpadAdd if colors.len() < MAX_RUNTIME_COLORS => {
      colors.push(RUNTIME_COLORS[colors.len()]);
      true
    }
    KeyCode::Minus | KeyCode::NumpadSubtract if colors.len() > MIN_RUNTIME_COLORS => {
      colors.pop();
      true
    }
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn plus_and_minus_resize_the_runtime_tail_within_bounds() {
    let mut colors = RUNTIME_COLORS[..3].to_vec();

    assert!(resize_runtime_colors(&mut colors, KeyCode::Equal));
    assert_eq!(colors, RUNTIME_COLORS[..4]);

    assert!(resize_runtime_colors(&mut colors, KeyCode::Minus));
    assert_eq!(colors, RUNTIME_COLORS[..3]);

    colors.truncate(MIN_RUNTIME_COLORS);
    assert!(!resize_runtime_colors(&mut colors, KeyCode::Minus));

    colors = RUNTIME_COLORS.to_vec();
    assert!(!resize_runtime_colors(&mut colors, KeyCode::Equal));
  }
}
