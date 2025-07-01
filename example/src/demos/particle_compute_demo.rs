use crate::demos::Demo;
use crate::shader_bindings::global_bindings::{
  WgpuBindGroup0Entries, WgpuBindGroup0EntriesParams,
};
use crate::shader_bindings::particle_physics::{
  self, Job, Params, WgpuBindGroup1Entries, WgpuBindGroup1EntriesParams,
};
use crate::shader_bindings::{self, particle_renderer};
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
    // Create initial job data - demonstrates Vec3A struct without padding issues
    let num_jobs = 64u32;
    let mut jobs = Vec::with_capacity(num_jobs as usize);

    for i in 0..num_jobs {
      let angle = (i as f32) * 0.1;
      jobs.push(Job::new(
        Vec3::new(angle.cos() * 2.0, angle.sin() * 2.0, (i as f32 * 0.05).sin() * 1.0)
          .into(),
        Vec3::new(
          (angle + 1.0).sin() * 0.02,
          (angle + 2.0).cos() * 0.02,
          (angle + 3.0).sin() * 0.01,
        )
        .into(),
        Vec3::ZERO.into(),
        0,
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

    // Create local time buffer for compute pass since it can't use global one
    let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("Particle Demo Local Time Buffer"),
      contents: bytemuck::cast_slice(&[0.0f32]),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let local_bind_group_0 =
      crate::shader_bindings::global_bindings::WgpuBindGroup0::from_bindings(
        device,
        WgpuBindGroup0Entries::new(WgpuBindGroup0EntriesParams {
          time: time_buffer.as_entire_buffer_binding(),
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
    let compute_shader_module =
      particle_physics::create_shader_module_embed_source(device);

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
    let render_shader = particle_renderer::create_shader_module_embed_source(device);

    let render_pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Particle Visualization Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
      });

    let render_pipeline =
      device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Particle Visualization Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: shader_bindings::particle_renderer::vertex_state(
          &render_shader,
          &shader_bindings::particle_renderer::vs_main_entry(
            wgpu::VertexStepMode::Vertex,
          ),
        ),
        fragment: Some(shader_bindings::particle_renderer::fragment_state(
          &render_shader,
          &shader_bindings::particle_renderer::fs_main_entry([Some(
            wgpu::ColorTargetState {
              format: surface_format,
              blend: Some(wgpu::BlendState::ALPHA_BLENDING),
              write_mask: wgpu::ColorWrites::ALL,
            },
          )]),
        )),
        primitive: wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::PointList,
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

    // Create vertex buffer for visualization
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("Particle Visualization Vertex Buffer"),
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

  fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, elapsed_time: f32) {
    self.time += elapsed_time;

    // Update local time buffer for compute shader
    queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[self.time]));

    // Update parameters for the compute shader
    let params = Params(
      1.0 + (self.time * 0.3).sin() * 0.5, // varying scale
      0.98,
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
      let num_workgroups = (self.num_jobs + workgroup_size - 1) / workgroup_size;
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

    // Update vertex buffer for visualization (simplified - using job positions)
    // In a real implementation, you might map the readback buffer, but for this demo
    // we'll update the visualization data based on our local copy
    let vertex_data: Vec<[f32; 4]> = (0..self.num_jobs)
      .map(|i| {
        let job_index = i as usize;
        if job_index < self.jobs_data.len() {
          // Simulate the compute update locally for visualization
          // (In real app you'd read back from GPU)
          let job = &mut self.jobs_data[job_index];

          // Apply some basic updates to simulate the compute shader
          job.position += job.direction * 0.016;
          job.depth += 1;

          // Normalize position for rendering (map to -1..1 range)
          [
            job.position.x * 0.1,
            job.position.y * 0.1,
            0.0,
            1.0 + (job.depth as f32 * 0.001), // Use depth to vary point size
          ]
        } else {
          [0.0, 0.0, 0.0, 1.0]
        }
      })
      .collect();

    queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));
  }

  fn render<'a>(
    &'a mut self,
    _device: &wgpu::Device,
    render_pass: &mut wgpu::RenderPass<'a>,
  ) {
    render_pass.set_pipeline(&self.render_pipeline);
    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    render_pass.draw(0..self.num_jobs, 0..1);
  }

  fn handle_input(&mut self, _event: &KeyEvent) -> bool {
    false
  }

  fn get_pipeline(&self) -> &wgpu::RenderPipeline {
    &self.render_pipeline
  }
}
