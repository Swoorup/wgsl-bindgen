use crate::demos::Demo;
use crate::shader_bindings::compute_demo::particle_physics::{
  self, Job, Params, WgpuBindGroup1Entries, WgpuBindGroup1EntriesParams,
};
use crate::shader_bindings::global_bindings::{
  WgpuBindGroup0Entries, WgpuBindGroup0EntriesParams,
};
use crate::shader_bindings::{self, compute_demo::particle_renderer};
use glam::Vec3;
use wgpu::util::DeviceExt;
use winit::event::KeyEvent;

pub struct ParticleComputeDemo {
  job_buffer: wgpu::Buffer,
  readback_buffer: wgpu::Buffer,
  params_buffer: wgpu::Buffer,
  time_buffer: wgpu::Buffer,
  local_bind_group_0: crate::shader_bindings::global_bindings::WgpuBindGroup0,
  bind_group_1: particle_physics::WgpuBindGroup1,
  compute_pipeline: wgpu::ComputePipeline,
  render_pipeline: wgpu::RenderPipeline,
  vertex_buffer: wgpu::Buffer,
  instance_buffer: wgpu::Buffer,
  num_jobs: u32,
  time: f32,
  jobs_data: Vec<Job>, // Keep a copy for visualization
}

impl Demo for ParticleComputeDemo {
  fn new(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
  ) -> Self {
    // Create initial job data with diverse particle types and interesting patterns
    let num_jobs = 2048u32; // Doubled particle count for much denser visual effect
    let mut jobs = Vec::with_capacity(num_jobs as usize);

    for i in 0..num_jobs {
      let t = i as f32;
      let particle_type = i % 4; // 4 different particle types

      let (pos, vel) = match particle_type {
        0 => {
          // Flock particles - start in clusters spread across screen
          let cluster = (i / 32) as f32;
          let angle = (i % 32) as f32 * 0.2;
          let cluster_center = Vec3::new(
            (cluster * 0.8).cos() * 0.6,
            (cluster * 0.8).sin() * 0.6,
            (cluster * 0.3).sin() * 0.2,
          );
          let offset =
            Vec3::new(angle.cos() * 0.1, angle.sin() * 0.1, (t * 0.05).sin() * 0.05);
          let pos = cluster_center + offset;
          let vel = Vec3::new(
            (t * 0.02).sin() * 0.05,
            (t * 0.03).cos() * 0.05,
            (t * 0.01).sin() * 0.02,
          );
          (pos, vel)
        }
        1 => {
          // Wanderer particles - scattered randomly across screen
          let pos = Vec3::new(
            (t * 0.1234).sin() * 0.8,
            (t * 0.2345).cos() * 0.8,
            (t * 0.0987).sin() * 0.3,
          );
          let vel = Vec3::new(
            (t * 0.1111).cos() * 0.08,
            (t * 0.2222).sin() * 0.08,
            (t * 0.0555).cos() * 0.04,
          );
          (pos, vel)
        }
        2 => {
          // Orbiter particles - start in rings around center
          let ring = ((i / 8) % 8) as f32;
          let angle = (i % 8) as f32 * 0.785; // π/4 spacing for more particles per ring
          let radius = 0.3 + ring * 0.15;
          let pos = Vec3::new(
            angle.cos() * radius,
            angle.sin() * radius,
            (ring * 0.3).sin() * 0.1,
          );
          // Initial tangential velocity for orbital motion
          let vel =
            Vec3::new(-angle.sin() * 0.1, angle.cos() * 0.1, (t * 0.01).cos() * 0.02);
          (pos, vel)
        }
        _ => {
          // Repulser particles - start in central area with outward motion
          let angle = t * 0.618; // Golden angle for nice distribution
          let radius = 0.1 + (t * 0.02).sin() * 0.2;
          let pos = Vec3::new(
            angle.cos() * radius,
            angle.sin() * radius,
            (t * 0.05).sin() * 0.05,
          );
          let vel =
            Vec3::new(angle.cos() * 0.08, angle.sin() * 0.08, (t * 0.01).sin() * 0.02);
          (pos, vel)
        }
      };

      jobs.push(Job::new(
        pos,
        vel,
        Vec3::ZERO,    // Will accumulate forces
        particle_type, // Store particle type in depth field
      ));
    }

    let job_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Particle Physics Demo Job Buffer"),
      contents: bytemuck::cast_slice(&jobs),
      usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    // Create readback buffer to copy data for visualization
    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("Particle Physics Demo Readback Buffer"),
      size: (num_jobs as u64) * std::mem::size_of::<Job>() as u64,
      usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    // Parameters: scale, damping (time is now global)
    let params = Params(1.0, 0.98);
    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Particle Physics Demo Params Buffer"),
      contents: bytemuck::cast_slice(&[params]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Create local global uniforms buffer for compute pass since it can't use global one
    // Note: The render pipeline uses the main global uniforms, this is only for the compute shader
    let global_uniforms = shader_bindings::global_bindings::GlobalUniforms::new(
      0.0,                           // time
      2.0,                           // scale_factor - reasonable default for high-DPI
      glam::Vec2::new(800.0, 600.0), // frame_size - initial default, will be updated
      glam::Vec2::new(0.0, 0.0),     // mouse_pos - initial position
    );
    let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Particle Demo Local Global Uniforms Buffer"),
      contents: bytemuck::cast_slice(&[global_uniforms]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let local_bind_group_0 =
      crate::shader_bindings::global_bindings::WgpuBindGroup0::from_bindings(
        device,
        WgpuBindGroup0Entries::new(WgpuBindGroup0EntriesParams {
          globals: time_buffer.as_entire_buffer_binding(),
        }),
      );

    let bind_group_1 = particle_physics::WgpuBindGroup1::from_bindings(
      device,
      WgpuBindGroup1Entries::new(WgpuBindGroup1EntriesParams {
        jobs: job_buffer.as_entire_buffer_binding(),
        params: params_buffer.as_entire_buffer_binding(),
      }),
    );

    // We'll use the bind group directly instead of the wrapper

    // Create compute pipeline
    let compute_pipeline_layout = particle_physics::create_pipeline_layout(device);
    let compute_shader_module = particle_physics::create_shader_module_relative_path(
      device,
      crate::SHADER_DIR,
      shader_bindings::ShaderEntry::ComputeDemoParticlePhysics,
      std::collections::HashMap::new(),
      |path| std::fs::read_to_string(path),
    )
    .expect("Failed to create compute shader module");

    let compute_pipeline =
      device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Particle Physics Demo Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &compute_shader_module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
      });

    // Create simple render pipeline to visualize the results
    let render_shader = particle_renderer::create_shader_module_relative_path(
      device,
      crate::SHADER_DIR,
      shader_bindings::ShaderEntry::ComputeDemoParticleRenderer,
      std::collections::HashMap::new(),
      |path| std::fs::read_to_string(path),
    )
    .expect("Failed to create shader module");

    let render_pipeline_layout = particle_renderer::create_pipeline_layout(device);

    let render_pipeline =
      device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Particle Visualization Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
          module: &render_shader,
          entry_point: Some(particle_renderer::ENTRY_VS_MAIN),
          compilation_options: wgpu::PipelineCompilationOptions::default(),
          buffers: &[
            // Quad vertices (per vertex) - matches @location(0) quad_pos
            wgpu::VertexBufferLayout {
              array_stride: std::mem::size_of::<glam::Vec2>() as u64,
              step_mode: wgpu::VertexStepMode::Vertex,
              attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0, // @location(0) quad_pos - matches generated VertexInput
              }],
            },
            // Instance data (per instance) - matches @location(1) position_and_size
            wgpu::VertexBufferLayout {
              array_stride: std::mem::size_of::<glam::Vec4>() as u64,
              step_mode: wgpu::VertexStepMode::Instance,
              attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: 1, // @location(1) position_and_size - matches generated VertexInput
              }],
            },
          ],
        },
        fragment: Some(particle_renderer::fragment_state(
          &render_shader,
          &particle_renderer::fs_main_entry([Some(wgpu::ColorTargetState {
            format: surface_format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
          })]),
        )),
        primitive: wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::TriangleList,
          strip_index_format: None,
          front_face: wgpu::FrontFace::Ccw,
          cull_mode: None,
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

    // Create quad vertex buffer for particle rendering (2 triangles = 6 vertices)
    let quad_vertices: &[f32] = &[
      // Triangle 1
      -1.0, -1.0, // Bottom left
      1.0, -1.0, // Bottom right
      -1.0, 1.0, // Top left
      // Triangle 2
      1.0, -1.0, // Bottom right
      1.0, 1.0, // Top right
      -1.0, 1.0, // Top left
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Particle Quad Vertex Buffer"),
      contents: bytemuck::cast_slice(quad_vertices),
      usage: wgpu::BufferUsages::VERTEX,
    });

    // Create instance buffer for particle positions and data
    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("Particle Instance Buffer"),
      size: (num_jobs as u64) * std::mem::size_of::<[f32; 4]>() as u64,
      usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    Self {
      job_buffer,
      readback_buffer,
      params_buffer,
      time_buffer,
      local_bind_group_0,
      bind_group_1,
      compute_pipeline,
      render_pipeline,
      vertex_buffer,
      instance_buffer,
      num_jobs,
      time: 0.0,
      jobs_data: jobs,
    }
  }

  fn name(&self) -> &'static str {
    "Particle Physics Demo"
  }

  fn description(&self) -> &'static str {
    "Running compute shader demo using Vec3A structs.\nDemonstrates that Job struct with multiple vec3<f32> fields\ncompiles and runs correctly without padding overflow issues.\nParticles move with physics simulation on GPU using Vec3A types."
  }

  fn update(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    context: super::DemoContext,
  ) {
    self.time += context.elapsed_time;

    // Update local global uniforms buffer for compute shader
    let global_uniforms = shader_bindings::global_bindings::GlobalUniforms::new(
      self.time,
      2.0,                // scale_factor - reasonable default
      context.frame_size, // Use actual frame size from context
      context.mouse_pos,  // Use actual mouse position
    );
    queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[global_uniforms]));

    // Update parameters for the compute shader
    let params = Params(
      2.0 + (self.time * 0.5).sin() * 1.0, // stronger varying scale
      0.95, // slightly less damping for more dynamic motion
    );

    queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

    // Run compute shader
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Particle Physics Demo Encoder"),
    });

    {
      let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("Particle Physics Demo Pass"),
        timestamp_writes: None,
      });

      compute_pass.set_pipeline(&self.compute_pipeline);
      self.local_bind_group_0.set(&mut compute_pass);
      self.bind_group_1.set(&mut compute_pass);

      let workgroup_size = 64;
      let num_workgroups = self.num_jobs.div_ceil(workgroup_size);
      compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
    }

    // Copy job buffer to readback buffer for visualization
    encoder.copy_buffer_to_buffer(
      &self.job_buffer,
      0,
      &self.readback_buffer,
      0,
      (self.num_jobs as u64) * std::mem::size_of::<Job>() as u64,
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Map the readback buffer to get the actual GPU compute results
    let buffer_slice = self.readback_buffer.slice(..);
    let (tx, rx) = futures::channel::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
      let _ = tx.send(result);
    });

    // Poll the device to complete the mapping
    let _ = device.poll(wgpu::MaintainBase::Wait);

    // Process the mapped data
    if let Ok(Ok(())) = futures::executor::block_on(rx) {
      let data = buffer_slice.get_mapped_range();
      let jobs: &[Job] = bytemuck::cast_slice(&data);

      // Extract instance data from GPU results
      let vertex_data: Vec<[f32; 4]> = jobs
        .iter()
        .enumerate()
        .map(|(i, job)| {
          let particle_type = i % 4; // Regular particle types
          let energy = (i as f32 * 0.37).fract() * 10.0; // Stable energy based on index

          [
            job.position.x, // Use actual GPU computed position
            job.position.y,
            job.position.z,
            particle_type as f32 + energy * 0.01, // Encode type and energy
          ]
        })
        .collect();

      // Unmap the buffer
      drop(data);
      self.readback_buffer.unmap();

      // Update the instance buffer with actual GPU results
      queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&vertex_data));
    } else {
      // Fallback: use the original positions if readback fails
      let vertex_data: Vec<[f32; 4]> = self
        .jobs_data
        .iter()
        .enumerate()
        .map(|(i, job)| {
          let particle_type = i % 4;
          let energy = (i as f32 * 0.37).fract() * 10.0; // Same stable energy calculation

          [
            job.position.x,
            job.position.y,
            job.position.z,
            particle_type as f32 + energy * 0.01,
          ]
        })
        .collect();

      queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&vertex_data));
    }
  }

  fn render<'a>(
    &'a mut self,
    _device: &wgpu::Device,
    render_pass: &mut wgpu::RenderPass<'a>,
  ) {
    render_pass.set_pipeline(&self.render_pipeline);
    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // Quad vertices
    render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..)); // Instance data
    render_pass.draw(0..6, 0..self.num_jobs); // 6 vertices per quad, num_jobs instances
  }

  fn handle_input(&mut self, _event: &KeyEvent) -> bool {
    false
  }

  fn get_pipeline(&self) -> &wgpu::RenderPipeline {
    &self.render_pipeline
  }
}
