use super::Demo;
use crate::shader_bindings;
use crate::shader_bindings::global_bindings::{
  WgpuBindGroup0Entries, WgpuBindGroup0EntriesParams,
};
use wgpu::util::DeviceExt;

pub struct FullscreenEffectsDemo {
  pipeline: wgpu::RenderPipeline,
  bind_group1: shader_bindings::fullscreen_effects::WgpuBindGroup1,
  bind_group2: shader_bindings::fullscreen_effects::WgpuBindGroup2,
  vertex_buffer: wgpu::Buffer,
  render_bundle: Option<wgpu::RenderBundle>,
  surface_format: wgpu::TextureFormat,
  time_buffer: wgpu::Buffer,
  global_bind_group: shader_bindings::global_bindings::WgpuBindGroup0,
}

impl Demo for FullscreenEffectsDemo {
  fn new(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
  ) -> Self {
    // Create shader and pipeline layout
    let shader = shader_bindings::fullscreen_effects::create_shader_module_relative_path(
      device,
      crate::SHADER_DIR,
      std::collections::HashMap::new(),
      |path| std::fs::read_to_string(path),
    )
    .expect("Failed to create shader module");
    let render_pipeline_layout =
      shader_bindings::fullscreen_effects::create_pipeline_layout(device);

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Fullscreen Effects Render Pipeline"),
      layout: Some(&render_pipeline_layout),
      vertex: shader_bindings::fullscreen_effects::vertex_state(
        &shader,
        &shader_bindings::fullscreen_effects::vs_main_entry(wgpu::VertexStepMode::Vertex),
      ),
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some(shader_bindings::fullscreen_effects::ENTRY_FS_MAIN),
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

    let bind_group1 = shader_bindings::fullscreen_effects::WgpuBindGroup1::from_bindings(
      device,
      shader_bindings::fullscreen_effects::WgpuBindGroup1Entries::new(
        shader_bindings::fullscreen_effects::WgpuBindGroup1EntriesParams {
          main_texture: &texture,
          main_sampler: &sampler,
        },
      ),
    );

    let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("uniforms"),
      contents: bytemuck::cast_slice(&[shader_bindings::fullscreen_effects::Uniforms(
        glam::vec4(0.8, 1.0, 0.6, 1.0),
      )]),
      usage: wgpu::BufferUsages::UNIFORM,
    });

    let bind_group2 = shader_bindings::fullscreen_effects::WgpuBindGroup2::from_bindings(
      device,
      shader_bindings::fullscreen_effects::WgpuBindGroup2Entries::new(
        shader_bindings::fullscreen_effects::WgpuBindGroup2EntriesParams {
          uniforms: uniforms_buffer.as_entire_buffer_binding(),
        },
      ),
    );

    // Create local global uniforms buffer and bind group for this demo
    // This allows us to include the global bind group in the cached render bundle
    let global_uniforms = shader_bindings::global_bindings::GlobalUniforms::new(
      0.0,                           // time
      1.0,                           // scale_factor - will be updated
      glam::Vec2::new(800.0, 600.0), // frame_size - will be updated
      glam::Vec2::new(0.0, 0.0),     // mouse_pos - will be updated
    );
    let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Fullscreen Effects Global Uniforms Buffer"),
      contents: bytemuck::cast_slice(&[global_uniforms]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let global_bind_group =
      shader_bindings::global_bindings::WgpuBindGroup0::from_bindings(
        device,
        WgpuBindGroup0Entries::new(WgpuBindGroup0EntriesParams {
          globals: time_buffer.as_entire_buffer_binding(),
        }),
      );

    // Create vertex buffer for fullscreen triangle
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("vertex buffer"),
      contents: bytemuck::cast_slice(&[
        shader_bindings::fullscreen_effects::VertexInput(glam::vec3(-1.0, -1.0, 0.0)),
        shader_bindings::fullscreen_effects::VertexInput(glam::vec3(3.0, -1.0, 0.0)),
        shader_bindings::fullscreen_effects::VertexInput(glam::vec3(-1.0, 3.0, 0.0)),
      ]),
      usage: wgpu::BufferUsages::VERTEX,
    });

    Self {
      pipeline,
      bind_group1,
      bind_group2,
      vertex_buffer,
      render_bundle: None,
      surface_format,
      time_buffer,
      global_bind_group,
    }
  }

  fn name(&self) -> &'static str {
    "Fullscreen Effects"
  }

  fn description(&self) -> &'static str {
    "Fullscreen shader with ripple effects, color shifting, and vignette"
  }

  fn update(
    &mut self,
    _device: &wgpu::Device,
    queue: &wgpu::Queue,
    context: super::DemoContext,
  ) {
    // Update our local global uniforms buffer with the current time
    // The render bundle contains a static reference to this buffer
    let global_uniforms = shader_bindings::global_bindings::GlobalUniforms::new(
      context.elapsed_time,
      1.0,                // scale_factor - approximation
      context.frame_size, // Use actual frame size from context
      context.mouse_pos,  // mouse_pos - now available through context
    );
    queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[global_uniforms]));
  }

  fn render<'a>(
    &'a mut self,
    device: &wgpu::Device,
    render_pass: &mut wgpu::RenderPass<'a>,
  ) {
    // Use cached render bundle that includes our local global bind group
    // The time buffer is updated in update() but the binding stays the same
    render_pass.execute_bundles(std::iter::once(self.get_render_bundle(device)));
  }

  fn get_pipeline(&self) -> &wgpu::RenderPipeline {
    &self.pipeline
  }
}

impl FullscreenEffectsDemo {
  fn get_render_bundle(&mut self, device: &wgpu::Device) -> &wgpu::RenderBundle {
    self.render_bundle.get_or_insert_with(|| {
      let mut bundle_encoder =
        device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
          label: Some("Fullscreen Effects Render Bundle"),
          color_formats: &[Some(self.surface_format)],
          sample_count: 1,
          depth_stencil: None,
          multiview: None,
        });

      bundle_encoder.set_pipeline(&self.pipeline);

      // Set push constants
      let push_constant = shader_bindings::fullscreen_effects::PushConstants {
        color_matrix: glam::Mat4::IDENTITY,
      };
      bundle_encoder.set_push_constants(
        wgpu::ShaderStages::VERTEX_FRAGMENT,
        0,
        bytemuck::cast_slice(&[push_constant]),
      );

      // Include global bind group in the render bundle for fullscreen effects
      // The bind group reference stays the same, we just update the buffer contents
      // Setting the global bind group outside the render bundle doesn't work
      // because the render bundle needs to capture the state at creation time
      self.global_bind_group.set(&mut bundle_encoder);
      self.bind_group1.set(&mut bundle_encoder);
      self.bind_group2.set(&mut bundle_encoder);
      bundle_encoder.set_vertex_buffer(0, self.vertex_buffer.slice(..));
      bundle_encoder.draw(0..3, 0..1);

      bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
        label: Some("Fullscreen Effects Render Bundle"),
      })
    })
  }

  fn create_checkerboard_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
  ) -> wgpu::TextureView {
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
