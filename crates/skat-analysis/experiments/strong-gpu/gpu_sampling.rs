// SPDX-License-Identifier: EUPL-1.2

//! Experimental RX 6900 XT sampler for single-group hidden distributions.
use crate::sampling::{HiddenCards, Place, World};
use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};
use skat_core::CardSet;
use std::sync::{OnceLock, mpsc};
use wgpu::{self as w, BufferUsages as U};

/// Number of distributions in the largest experiment.
const CAP: usize = 24_000;

struct Backend {
    device: w::Device,
    queue: w::Queue,
    pipeline: w::ComputePipeline,
    jobs: w::Buffer,
    results: w::Buffer,
    read: w::Buffer,
}

impl Backend {
    fn new() -> Option<Self> {
        let instance = w::Instance::default();
        let adapter = pollster::block_on(instance.enumerate_adapters(w::Backends::VULKAN))
            .into_iter()
            .find(|a| a.get_info().name.contains("6900 XT"))?;
        eprintln!("GPU sampler: {}", adapter.get_info().name);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).ok()?;
        let shader = device.create_shader_module(w::ShaderModuleDescriptor {
            label: None,
            source: w::ShaderSource::Wgsl(include_str!("sample.wgsl").into()),
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
        let jobs = buffer(48 * (CAP / 24) as u64, U::COPY_DST | U::STORAGE);
        let results = buffer(16 * CAP as u64, U::COPY_SRC | U::STORAGE);
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

    fn run(&self, jobs: &[[u32; 12]]) -> Option<Vec<[u32; 4]>> {
        let count = jobs.len() * 24;
        let bytes = (count * 16) as u64;
        let input = bytemuck::cast_slice(jobs);
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

// Game workers may submit while the preceding batch executes. Drain only
// requests already waiting, so a lone decision never waits to fill a batch.
type Reply = mpsc::Sender<Option<Vec<[u32; 4]>>>;
type Request = (Vec<[u32; 12]>, Reply);

fn submit(jobs: Vec<[u32; 12]>) -> Option<Vec<[u32; 4]>> {
    static QUEUE: OnceLock<Option<mpsc::Sender<Request>>> = OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let gpu = Backend::new()?;
        let (tx, rx) = mpsc::channel::<Request>();
        std::thread::spawn(move || {
            let mut pending = None;
            let (mut completed, mut submissions, mut largest) = (0, 0, 0);
            while let Some((mut jobs, first)) = pending.take().or_else(|| rx.recv().ok()) {
                let mut replies = vec![(jobs.len() * 24, first)];
                while let Ok((more, reply)) = rx.try_recv() {
                    if jobs.len() + more.len() > CAP / 24 {
                        pending = Some((more, reply));
                        break;
                    }
                    replies.push((more.len() * 24, reply));
                    jobs.extend(more);
                }
                let rows = gpu.run(&jobs);
                if rows.is_some() {
                    let previous = completed;
                    completed += jobs.len();
                    submissions += 1;
                    largest = largest.max(jobs.len());
                    if previous == 0 || previous / 1024 != completed / 1024 {
                        eprintln!("GPU sampler: {completed} decisions in {submissions} submissions, largest {largest}");
                    }
                }
                let mut start = 0;
                for (len, reply) in replies {
                    let _ = reply.send(rows.as_ref().map(|r| r[start..start+len].to_vec()));
                    start += len;
                }
            }
        });
        Some(tx)
    }).as_ref()?;
    let (tx, rx) = mpsc::channel();
    queue.send((jobs, tx)).ok()?;
    rx.recv().ok()?
}

/// One job per independent decision, each with its own original seed and 24 samples.
pub fn batch(inputs: &[(&HiddenCards, u64)]) -> Option<Vec<Vec<World>>> {
    if inputs.is_empty() || inputs.len() > CAP / 24 {
        return None;
    }
    let jobs = inputs
        .iter()
        .map(|&(h, seed)| {
            let slots = h.slots();
            if h.count() <= 24
                || h.count() > u32::MAX.into()
                || slots.iter().any(|s| s.allowed != h.unseen())
            {
                return None;
            }
            let key = ChaCha8Rng::seed_from_u64(seed).get_seed();
            let mut job = [0; 12];
            for (i, k) in key.as_chunks::<4>().0.iter().enumerate() {
                job[i] = u32::from_le_bytes(*k);
            }
            job[8..].copy_from_slice(&[
                h.unseen().bits(),
                slots[0].size.into(),
                slots.get(1).map_or(0, |s| s.size.into()),
                24,
            ]);
            Some(job)
        })
        .collect::<Option<Vec<_>>>()?;
    let rows = submit(jobs)?;
    inputs
        .iter()
        .zip(rows.as_chunks::<24>().0)
        .map(|(&(h, _), rows)| {
            rows.iter()
                .map(|row| {
                    if row[3] != 0 {
                        return None;
                    }
                    let mut world = h.known();
                    for (slot, &bits) in h.slots().iter().zip(row) {
                        let mask = CardSet::from_bits(bits);
                        match slot.place {
                            Place::Seat(s) => world.hands[s.index()] |= mask,
                            Place::Skat => world.skat |= mask,
                        }
                    }
                    Some(world)
                })
                .collect()
        })
        .collect()
}

pub(crate) fn sample(h: &HiddenCards, samples: usize, seed: u64) -> Option<Vec<World>> {
    if samples != 24 {
        return None;
    }
    batch(&[(h, seed)])?.pop()
}
