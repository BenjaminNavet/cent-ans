//! Frames-in-flight limit for hidden (`--offscreen`) runs (MEM3, ADR 0786).
//!
//! A windowed client is paced by the swapchain (`desired_maximum_frame_latency`), so the GPU never
//! lags more than two or three frames behind the CPU. A hidden client renders to an `Image`: nothing
//! is presented, the CPU runs as far ahead as Metal allows and every frame in flight holds driver
//! memory (`IOAccelerator (graphics)`, command data and transient allocations), which showed up as
//! footprint swings of 2.4 GB in agent runs only. This plugin counts the submitted frames whose
//! work is not done yet and, past the limit, waits for the oldest. `CENT_ANS_GPU_LATENCY=<n>` sets
//! the limit (default [`DEFAULT_LATENCY`] = 0, unlimited).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::render_resource::PollType;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::{Render, RenderApp, RenderSystems};

/// Frames allowed in flight when nothing is configured: `0` = no limit (as before). The limit is opt-in
/// (`CENT_ANS_GPU_LATENCY=2`): MEM4 measured no footprint gain with it (5 264 Mo unlimited, 5 394 Mo at 2).
pub const DEFAULT_LATENCY: usize = 0;

fn latency() -> usize {
    std::env::var("CENT_ANS_GPU_LATENCY")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(DEFAULT_LATENCY)
}

#[derive(Resource, Clone)]
struct InFlight(Arc<AtomicUsize>);

pub struct GpuPacingPlugin;

impl Plugin for GpuPacingPlugin {
    fn build(&self, app: &mut App) {
        if latency() == 0 {
            return;
        }
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(InFlight(Arc::new(AtomicUsize::new(0))))
            .add_systems(Render, pace.in_set(RenderSystems::Cleanup));
    }
}

fn pace(in_flight: Res<InFlight>, device: Res<RenderDevice>, queue: Res<RenderQueue>) {
    let counter = Arc::clone(&in_flight.0);
    counter.fetch_add(1, Ordering::SeqCst);
    queue.on_submitted_work_done(move || {
        counter.fetch_sub(1, Ordering::SeqCst);
    });
    let limit = latency();
    while in_flight.0.load(Ordering::SeqCst) > limit {
        let _ = device.poll(PollType::Poll);
        std::thread::sleep(Duration::from_micros(200));
    }
}
