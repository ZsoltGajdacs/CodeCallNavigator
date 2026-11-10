//! Nonblocking timestamp samples. Completed GPU samples carry their originating frame index.
use crate::app::State;
use rune::{Frame, compose::DeviceKey};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

#[derive(Default)]
pub(super) struct Meter {
    device: Option<DeviceKey>,
    slots: Vec<Slot>,
    invalid_samples: u32,
}

struct Slot {
    queries: wgpu::QuerySet,
    resolved: wgpu::Buffer,
    read: wgpu::Buffer,
    status: Arc<AtomicU8>,
    sample: u64,
}

const IDLE: u8 = 0;
const SUBMITTED: u8 = 1;
const MAPPING: u8 = 2;
const READY: u8 = 3;
const FAILED: u8 = 4;

impl Meter {
    pub(super) fn draw(&mut self, frame: &mut Frame<'_, State>, sample: u64) {
        self.follow_device(frame);
        self.collect(frame);
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.status.load(Ordering::Acquire) == IDLE);
        let clear = frame.rt.clear_color();
        frame.ui.prepare(
            frame.device,
            frame.queue,
            &mut frame.rt.text.atlas,
            &frame.rt.instances,
            frame.size,
        );
        if let Some(slot) = slot {
            frame.ui.draw_timed(
                frame.encoder,
                frame.target,
                clear,
                Some(wgpu::RenderPassTimestampWrites {
                    query_set: &slot.queries,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
            );
            frame
                .encoder
                .resolve_query_set(&slot.queries, 0..2, &slot.resolved, 0);
            frame
                .encoder
                .copy_buffer_to_buffer(&slot.resolved, 0, &slot.read, 0, 16);
            slot.sample = sample;
            slot.status.store(SUBMITTED, Ordering::Release);
        } else {
            frame.ui.draw(frame.encoder, frame.target, clear);
        }
    }

    fn follow_device(&mut self, frame: &mut Frame<'_, State>) {
        if self.device == Some(frame.device_key()) {
            return;
        }
        self.device = Some(frame.device_key());
        self.slots.clear();
        let supported = frame
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY);
        frame.rt.trace_channel(
            "codegraph.gpu_supported",
            "level",
            u8::from(supported) as f32,
        );
        if supported {
            self.slots = (0..4).map(|_| Slot::new(frame.device)).collect();
        }
    }

    fn collect(&mut self, frame: &mut Frame<'_, State>) {
        let _ = frame.device.poll(wgpu::PollType::Poll);
        self.take_completed(frame);
        for slot in &mut self.slots {
            match slot.status.load(Ordering::Acquire) {
                SUBMITTED => slot.map(),
                FAILED => frame
                    .rt
                    .trace_channel("codegraph.gpu_map_failed", "level", 1.0),
                _ => {}
            }
        }
    }

    fn take_completed(&mut self, frame: &mut Frame<'_, State>) {
        // Drain one originating sample per trace row; ready samples must never overwrite
        // one another's scalar channels. Oldest first also prevents a fast slot starving others.
        let ready = self
            .slots
            .iter_mut()
            .filter(|slot| slot.status.load(Ordering::Acquire) == READY)
            .min_by_key(|slot| slot.sample);
        if let Some(slot) = ready {
            let ticks = slot.take_ticks();
            match duration_ms(ticks, frame.queue.get_timestamp_period()) {
                Some(milliseconds) => {
                    frame
                        .rt
                        .trace_channel("codegraph.gpu_sample", "level", slot.sample as f32);
                    frame
                        .rt
                        .trace_channel("codegraph.gpu_ms", "level", milliseconds);
                }
                None => self.invalid_samples += 1,
            }
        }
        frame.rt.trace_channel(
            "codegraph.gpu_invalid_samples",
            "level",
            self.invalid_samples as f32,
        );
    }
}

impl Slot {
    fn new(device: &wgpu::Device) -> Self {
        let buffer = |usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("pan-profile"),
                size: 16,
                usage,
                mapped_at_creation: false,
            })
        };
        Self {
            queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("pan-profile"),
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            }),
            resolved: buffer(wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC),
            read: buffer(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST),
            status: Arc::new(AtomicU8::new(IDLE)),
            sample: 0,
        }
    }

    fn take_ticks(&mut self) -> [u64; 2] {
        let bytes = self
            .read
            .slice(..)
            .get_mapped_range()
            .expect("completed timestamp map");
        let first = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let last = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        drop(bytes);
        self.read.unmap();
        self.status.store(IDLE, Ordering::Release);
        [first, last]
    }

    fn map(&mut self) {
        self.status.store(MAPPING, Ordering::Release);
        let status = self.status.clone();
        self.read
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                status.store(
                    if result.is_ok() { READY } else { FAILED },
                    Ordering::Release,
                );
            });
    }
}

fn duration_ms([first, last]: [u64; 2], period: f32) -> Option<f32> {
    if first == 0 || last <= first {
        return None;
    }
    let milliseconds = (last - first) as f64 * f64::from(period) / 1e6;
    (milliseconds.is_finite() && milliseconds > 0.0).then_some(milliseconds as f32)
}

#[cfg(test)]
mod tests {
    use super::duration_ms;

    #[test]
    fn zero_equal_and_reversed_gpu_timestamps_are_unavailable_instead_of_zero_ms() {
        for pair in [[0, 0], [0, 100], [100, 100], [200, 100]] {
            assert_eq!(duration_ms(pair, 1.0), None);
        }
        assert_eq!(duration_ms([100, 1_000_100], 2.0), Some(2.0));
        assert_eq!(duration_ms([100, 1_000_100], 0.0), None);
    }
}
