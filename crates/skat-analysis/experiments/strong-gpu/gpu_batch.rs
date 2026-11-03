// SPDX-License-Identifier: EUPL-1.2

use wgpu::{self as w, BufferUsages as U};

pub(super) struct Backend {
    device: w::Device,
    queue: w::Queue,
    pipeline: w::ComputePipeline,
    jobs: w::Buffer,
    results: w::Buffer,
    read: w::Buffer,
}

impl Backend {
    pub(super) fn new(source: &str, input_bytes: u64, output_bytes: u64) -> Option<Self> {
        let instance = w::Instance::default();
        let adapter = pollster::block_on(instance.enumerate_adapters(w::Backends::VULKAN))
            .into_iter()
            .filter(|a| {
                matches!(
                    a.get_info().device_type,
                    w::DeviceType::DiscreteGpu | w::DeviceType::IntegratedGpu
                )
            })
            .min_by_key(|a| a.get_info().device_type != w::DeviceType::DiscreteGpu)?;
        eprintln!("GPU compute: {}", adapter.get_info().name);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).ok()?;
        let shader = device.create_shader_module(w::ShaderModuleDescriptor {
            label: None,
            source: w::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_compute_pipeline(&w::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });
        let buffer = |size, usage| {
            device.create_buffer(&w::BufferDescriptor {
                label: None,
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let jobs = buffer(input_bytes, U::COPY_DST | U::STORAGE);
        let results = buffer(output_bytes, U::COPY_SRC | U::STORAGE);
        let read = buffer(results.size(), U::MAP_READ | U::COPY_DST);
        Some(Self {
            device,
            queue,
            pipeline,
            jobs,
            results,
            read,
        })
    }

    pub(super) fn run<T: bytemuck::Pod>(&self, input: &[u8], count: usize) -> Option<Vec<T>> {
        let bytes = (count * std::mem::size_of::<T>()) as u64;
        self.queue.write_buffer(&self.jobs, 0, input);
        let group = self.device.create_bind_group(&w::BindGroupDescriptor {
            layout: &self.pipeline.get_bind_group_layout(0),
            label: None,
            entries: &[
                w::BindGroupEntry {
                    binding: 0,
                    resource: w::BindingResource::Buffer(w::BufferBinding {
                        buffer: &self.jobs,
                        offset: 0,
                        size: w::BufferSize::new(input.len() as u64),
                    }),
                },
                w::BindGroupEntry {
                    binding: 1,
                    resource: self.results.as_entire_binding(),
                },
            ],
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(count.div_ceil(64) as u32, 1, 1);
        }
        enc.copy_buffer_to_buffer(&self.results, 0, &self.read, 0, bytes);
        self.queue.submit([enc.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        let slice = self.read.slice(..bytes);
        slice.map_async(w::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device.poll(w::PollType::wait_indefinitely()).ok()?;
        rx.recv().ok()?.ok()?;
        let out = bytemuck::cast_slice(&slice.get_mapped_range().ok()?).to_vec();
        self.read.unmap();
        Some(out)
    }
}
