//! Work counters (ADR 0827): what a frame costs in WORK rather than in wall-clock time, so a
//! measurement does not depend on how busy the machine is.
//!
//! - **Draw calls, triangles, instances** submitted through `TrackedRenderPass` (Bevy's own phases
//!   and the project's instanced / skinned / billboard commands). Bevy exposes no counter and
//!   wgpu none either (Metal has no pipeline statistics), so the cargo feature `perf-counters`
//!   turns on `bevy_render/detailed_trace` and a `tracing` layer counts the `draw*` events. The
//!   layer refuses every other callsite of that module (`Interest::never`, a static check in the
//!   macro), so the cost is the few draw events only. Without the feature the draw counters are
//!   absent from the report (`null`) and the guard fails on them.
//! - **Pipelines**: `PipelineCache` size in the render world, sampled once per frame (created =
//!   growth, so a late specialisation shows up even when it hides inside a frame).
//! - **CPU time**: process CPU (`getrusage`, every thread: render, workers, driver) and main
//!   thread CPU (`CLOCK_THREAD_CPUTIME_ID`) per frame. A thread that waits for a loaded machine
//!   burns no CPU time, which is why these figures hold under load where frame times do not.
//! - **Passes**: `RenderDiagnosticsPlugin` CPU spans per pass (Metal gives CPU spans only), count
//!   and mean per pass name.
//!
//! The bench steps only name the current phase (`WorkRecorder::phase`); sampling happens in `Last`
//! and the report adds a `work` object per phase. The dump section `work` shows the last frames.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;
use bevy::render::render_resource::PipelineCache;
use bevy::render::{Render, RenderApp, RenderSystems};
use serde_json::{Value, json};

use super::bench::percentile;
use super::dump::DumpAppExt;

/// Frames kept for the live `work` dump section.
const LIVE_WINDOW: usize = 240;
/// Passes listed in a phase report.
const TOP_PASSES: usize = 10;

// --- render side ---------------------------------------------------------------------------

static DRAWS: AtomicU64 = AtomicU64::new(0);
static TRIANGLES: AtomicU64 = AtomicU64::new(0);
static INSTANCES: AtomicU64 = AtomicU64::new(0);

/// What the render world submitted during one frame.
#[derive(Clone, Copy, Debug, Default)]
struct RenderSample {
    draws: u64,
    triangles: u64,
    instances: u64,
    pipelines_total: usize,
}

static RENDER_SAMPLES: Mutex<VecDeque<RenderSample>> = Mutex::new(VecDeque::new());

/// Render world, end of frame: closes the frame's counters and queues them for the main world.
fn close_render_frame(pipelines: Res<PipelineCache>) {
    let sample = RenderSample {
        draws: DRAWS.swap(0, Ordering::Relaxed),
        triangles: TRIANGLES.swap(0, Ordering::Relaxed),
        instances: INSTANCES.swap(0, Ordering::Relaxed),
        pipelines_total: pipelines.pipelines().count(),
    };
    if let Ok(mut queue) = RENDER_SAMPLES.lock() {
        queue.push_back(sample);
        // The main world drains every frame; this only bounds a stalled consumer.
        while queue.len() > 64 {
            queue.pop_front();
        }
    }
}

/// Draw-event counting layer (feature `perf-counters`).
#[cfg(feature = "perf-counters")]
pub mod draw_layer {
    use std::fmt::{self, Write};
    use std::sync::atomic::Ordering;

    use tracing::field::{Field, Visit};
    use tracing::subscriber::Interest;
    use tracing::{Event, Metadata, Subscriber};
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::Context;

    use super::{DRAWS, INSTANCES, TRIANGLES};

    /// Target of the `trace!` events of `TrackedRenderPass`.
    pub const TARGET: &str = "bevy_render::render_phase::draw_state";
    /// Lines of the `draw*` events in `bevy_render 0.20.0` `render_phase/draw_state.rs`
    /// (`draw`, `draw_indexed`, `draw_indirect`, `draw_indexed_indirect`, the four `multi_draw_*`).
    /// The crate is pinned (`=0.20.0`); a bump must re-read these lines, the report shows
    /// `draw_calls: null` if they miss.
    const DRAW_LINES: [u32; 8] = [277, 400, 428, 453, 486, 527, 570, 613];

    /// Fixed buffer for the event message: no allocation per draw.
    struct Message {
        bytes: [u8; 96],
        len: usize,
    }

    impl Write for Message {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            let room = self.bytes.len() - self.len;
            let take = text.len().min(room);
            self.bytes[self.len..self.len + take].copy_from_slice(&text.as_bytes()[..take]);
            self.len += take;
            Ok(())
        }
    }

    struct MessageVisitor<'a>(&'a mut Message);

    impl Visit for MessageVisitor<'_> {
        fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
            if field.name() == "message" {
                let _ = write!(self.0, "{value:?}");
            }
        }
    }

    /// `"a..b"` -> `b - a`.
    fn range_len(token: &str) -> u64 {
        token
            .split_once("..")
            .and_then(|(from, to)| Some(to.parse::<u64>().ok()? - from.parse::<u64>().ok()?))
            .unwrap_or(0)
    }

    pub struct DrawCounter;

    impl<S: Subscriber> Layer<S> for DrawCounter {
        fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
            if metadata.target() != TARGET {
                return Interest::always();
            }
            if metadata
                .line()
                .is_some_and(|line| DRAW_LINES.contains(&line))
            {
                Interest::always()
            } else {
                Interest::never()
            }
        }

        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            if event.metadata().target() != TARGET {
                return;
            }
            let mut message = Message {
                bytes: [0; 96],
                len: 0,
            };
            event.record(&mut MessageVisitor(&mut message));
            let text = std::str::from_utf8(&message.bytes[..message.len]).unwrap_or("");
            DRAWS.fetch_add(1, Ordering::Relaxed);
            let mut tokens = text.split_whitespace();
            let (instances, triangles) = match tokens.next() {
                // "draw: 0..3 0..1"
                Some("draw:") => {
                    let vertices = range_len(tokens.next().unwrap_or(""));
                    let instances = range_len(tokens.next().unwrap_or(""));
                    (instances, vertices / 3 * instances)
                }
                // "draw indexed: 0..36 0 5..9"
                Some("draw") if text.starts_with("draw indexed:") => {
                    let _ = tokens.next();
                    let indices = range_len(tokens.next().unwrap_or(""));
                    let _base = tokens.next();
                    let instances = range_len(tokens.next().unwrap_or(""));
                    (instances, indices / 3 * instances)
                }
                // Indirect and multi draws: the counts live in a GPU buffer.
                _ => (0, 0),
            };
            INSTANCES.fetch_add(instances, Ordering::Relaxed);
            TRIANGLES.fetch_add(triangles, Ordering::Relaxed);
        }
    }

    /// `LogPlugin::fmt_layer`: the usual stderr formatter, minus the draw events (they are only
    /// there to be counted, a line per draw would drown the log).
    pub fn fmt_layer(_app: &mut bevy::prelude::App) -> Option<bevy::log::BoxedFmtLayer> {
        use tracing_subscriber::Layer as _;
        use tracing_subscriber::filter::FilterFn;
        Some(Box::new(
            tracing_subscriber::fmt::Layer::default()
                .with_writer(std::io::stderr)
                .with_filter(FilterFn::new(|metadata| metadata.target() != TARGET)),
        ))
    }
}

// --- CPU clocks ----------------------------------------------------------------------------

/// CPU time (user + system) of every thread of the process, in ms.
pub fn process_cpu_ms() -> f64 {
    // SAFETY: `getrusage` only writes into the zeroed struct we hand it.
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0 {
            return 0.0;
        }
        let micros = |t: libc::timeval| t.tv_sec as f64 * 1e6 + t.tv_usec as f64;
        (micros(usage.ru_utime) + micros(usage.ru_stime)) / 1000.0
    }
}

/// CPU time of the calling thread, in ms (`CLOCK_THREAD_CPUTIME_ID`).
pub fn thread_cpu_ms() -> f64 {
    let mut spec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `clock_gettime` only writes into the struct we hand it.
    let status = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut spec) };
    if status != 0 {
        return 0.0;
    }
    spec.tv_sec as f64 * 1e3 + spec.tv_nsec as f64 / 1e6
}

/// Heap of the allocator (mimalloc: committed bytes now and at peak), in MiB. `None` when the
/// platform allocator is used (`system-alloc`).
pub fn heap_commit_mib() -> Option<(f64, f64)> {
    #[cfg(not(feature = "system-alloc"))]
    {
        unsafe extern "C" {
            fn mi_process_info(
                elapsed_msecs: *mut usize,
                user_msecs: *mut usize,
                system_msecs: *mut usize,
                current_rss: *mut usize,
                peak_rss: *mut usize,
                current_commit: *mut usize,
                peak_commit: *mut usize,
                page_faults: *mut usize,
            );
        }
        let (mut a, mut b, mut c, mut d, mut e, mut commit, mut peak, mut g) =
            (0usize, 0, 0, 0, 0, 0, 0, 0);
        // SAFETY: documented mimalloc call; every pointer is a valid local `usize`.
        unsafe {
            mi_process_info(
                &mut a,
                &mut b,
                &mut c,
                &mut d,
                &mut e,
                &mut commit,
                &mut peak,
                &mut g,
            );
        }
        let mib = |bytes: usize| bytes as f64 / (1024.0 * 1024.0);
        Some((mib(commit), mib(peak)))
    }
    #[cfg(feature = "system-alloc")]
    None
}

// --- main side -----------------------------------------------------------------------------

/// Series of one measured phase.
#[derive(Default, Clone)]
struct PhaseWork {
    process_cpu_ms: Vec<f32>,
    main_cpu_ms: Vec<f32>,
    draws: Vec<f32>,
    triangles: Vec<f32>,
    instances: Vec<f32>,
    passes: Vec<f32>,
    pipelines_created: usize,
    pipelines_total: usize,
    /// Pass name -> (sum of CPU ms, samples).
    pass_ms: BTreeMap<String, (f64, u32)>,
}

impl PhaseWork {
    fn trim(&mut self, keep: usize) {
        for series in [
            &mut self.process_cpu_ms,
            &mut self.main_cpu_ms,
            &mut self.draws,
            &mut self.triangles,
            &mut self.instances,
            &mut self.passes,
        ] {
            if series.len() > keep {
                series.drain(..series.len() - keep);
            }
        }
    }

    fn json(&self) -> Value {
        let r = |v: f32| (v * 100.0).round() / 100.0;
        let stat = |series: &[f32], q: f32| {
            let mut sorted = series.to_vec();
            sorted.sort_by(f32::total_cmp);
            r(percentile(&sorted, q))
        };
        let mean = |series: &[f32]| r(series.iter().sum::<f32>() / series.len().max(1) as f32);
        let counted = !self.draws.is_empty() && self.draws.iter().any(|d| *d > 0.0);
        let mut passes: Vec<(&String, f64, u32)> = self
            .pass_ms
            .iter()
            .map(|(name, (sum, n))| (name, sum / f64::from((*n).max(1)), *n))
            .collect();
        passes.sort_by(|a, b| b.1.total_cmp(&a.1));
        let top: Vec<Value> = passes
            .iter()
            .take(TOP_PASSES)
            .map(|(name, mean_ms, n)| json!({"pass": name, "cpu_ms_mean": (mean_ms * 1000.0).round() / 1000.0, "samples": n}))
            .collect();
        json!({
            "frames": self.process_cpu_ms.len(),
            "render_frames": self.draws.len(),
            // null = the build has no `perf-counters` (or the callsite lines moved): guard fails.
            "draw_calls_p50": counted.then(|| stat(&self.draws, 0.5)),
            "draw_calls_p95": counted.then(|| stat(&self.draws, 0.95)),
            "draw_calls_max": counted.then(|| stat(&self.draws, 1.0)),
            "triangles_p50": counted.then(|| stat(&self.triangles, 0.5)),
            "instances_p50": counted.then(|| stat(&self.instances, 0.5)),
            "pipelines_created": self.pipelines_created,
            "pipelines_total": self.pipelines_total,
            "render_passes_p50": stat(&self.passes, 0.5),
            "process_cpu_ms_mean": mean(&self.process_cpu_ms),
            "process_cpu_ms_p50": stat(&self.process_cpu_ms, 0.5),
            "process_cpu_ms_p95": stat(&self.process_cpu_ms, 0.95),
            "main_thread_cpu_ms_mean": mean(&self.main_cpu_ms),
            "main_thread_cpu_ms_p50": stat(&self.main_cpu_ms, 0.5),
            "main_thread_cpu_ms_p95": stat(&self.main_cpu_ms, 0.95),
            "pass_cpu_ms": top,
        })
    }
}

/// Collects the counters; `phase` is set by the benches (`None` = not measured).
#[derive(Resource, Default)]
pub struct WorkRecorder {
    pub phase: Option<&'static str>,
    phases: BTreeMap<&'static str, PhaseWork>,
    live: PhaseWork,
    last_process_ms: f64,
    last_thread_ms: f64,
    pipelines_seen: Option<usize>,
}

impl WorkRecorder {
    /// Report block of a phase (`null` if nothing was recorded).
    pub fn phase_json(&self, phase: &str) -> Value {
        self.phases.get(phase).map_or(Value::Null, PhaseWork::json)
    }
}

/// Adds the `work` block of `phase` to a `Stats::json()` object.
pub fn with_work(mut stats: Value, recorder: Option<&WorkRecorder>, phase: &str) -> Value {
    if let (Some(object), Some(recorder)) = (stats.as_object_mut(), recorder) {
        object.insert("work".into(), recorder.phase_json(phase));
    }
    stats
}

/// Main world, last system of the frame.
/// Exclusive on purpose: an exclusive system runs on the thread that drives the schedule (the main
/// thread), so the thread CPU clock read here is always the same thread's (a plain system may run on
/// any task-pool thread, whose clocks are unrelated).
fn sample_frame(world: &mut World) {
    let thread_ms = thread_cpu_ms();
    world.resource_scope(|world, mut recorder: Mut<WorkRecorder>| {
        record_frame(
            &mut recorder,
            world.get_resource::<DiagnosticsStore>(),
            thread_ms,
        );
    });
}

fn record_frame(
    recorder: &mut WorkRecorder,
    diagnostics: Option<&DiagnosticsStore>,
    thread_ms: f64,
) {
    let process_ms = process_cpu_ms();
    let process_delta = (process_ms - recorder.last_process_ms) as f32;
    let thread_delta = (thread_ms - recorder.last_thread_ms) as f32;
    let first = recorder.last_process_ms == 0.0;
    recorder.last_process_ms = process_ms;
    recorder.last_thread_ms = thread_ms;
    let samples: Vec<RenderSample> = RENDER_SAMPLES
        .lock()
        .map(|mut queue| queue.drain(..).collect())
        .unwrap_or_default();

    // Pass spans of the last frame: name -> CPU ms.
    let mut passes: Vec<(String, f64)> = Vec::new();
    if let Some(store) = diagnostics {
        for diagnostic in store.iter() {
            let path = diagnostic.path().as_str();
            if let Some(name) = path.strip_suffix("/elapsed_cpu")
                && let Some(value) = diagnostic.value()
            {
                passes.push((name.trim_start_matches("render/").to_owned(), value));
            }
        }
    }

    let record = |work: &mut PhaseWork, with_passes: bool| {
        if !first {
            work.process_cpu_ms.push(process_delta);
            work.main_cpu_ms.push(thread_delta);
            work.passes.push(passes.len() as f32);
        }
        for sample in &samples {
            work.draws.push(sample.draws as f32);
            work.triangles.push(sample.triangles as f32);
            work.instances.push(sample.instances as f32);
        }
        if with_passes {
            for (name, ms) in &passes {
                let entry = work.pass_ms.entry(name.clone()).or_default();
                entry.0 += ms;
                entry.1 += 1;
            }
        }
        if let Some(last) = samples.last() {
            work.pipelines_total = last.pipelines_total;
        }
    };

    let mut created = 0usize;
    for sample in &samples {
        if let Some(seen) = recorder.pipelines_seen {
            created += sample.pipelines_total.saturating_sub(seen);
        }
        recorder.pipelines_seen = Some(sample.pipelines_total);
    }
    record(&mut recorder.live, false);
    recorder.live.trim(LIVE_WINDOW);
    recorder.live.pipelines_created += created;
    if let Some(phase) = recorder.phase {
        let work = recorder.phases.entry(phase).or_default();
        record(work, true);
        work.pipelines_created += created;
    }
}

fn dump(world: &World) -> (String, Value) {
    let Some(recorder) = world.get_resource::<WorkRecorder>() else {
        return ("work".into(), Value::Null);
    };
    let mut live = recorder.live.json();
    live["note"] = json!(format!(
        "last {LIVE_WINDOW} frames; draw counters need the cargo feature perf-counters; pipelines_created = since start"
    ));
    ("work".into(), live)
}

/// Registers the recorder, the render-side sampler and the dump section. `diagnostics` also adds
/// Bevy's render diagnostics (CPU span per pass): benches only, it is not free.
pub struct WorkCountersPlugin {
    pub diagnostics: bool,
}

impl Plugin for WorkCountersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorkRecorder>()
            .register_dump(dump)
            .add_systems(Last, sample_frame);
        if self.diagnostics {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
        }
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(Render, close_render_frame.in_set(RenderSystems::Cleanup));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_without_draw_counters_reports_null() {
        let mut work = PhaseWork::default();
        work.process_cpu_ms.extend([10.0, 12.0, 11.0]);
        work.draws.extend([0.0, 0.0]);
        let report = work.json();
        assert!(report["draw_calls_p50"].is_null());
        assert_eq!(report["process_cpu_ms_p50"], json!(11.0));
    }

    #[test]
    fn phase_with_draws_reports_percentiles() {
        let mut work = PhaseWork::default();
        work.draws.extend([100.0, 200.0, 300.0]);
        work.triangles.extend([1.0, 2.0, 3.0]);
        let report = work.json();
        assert_eq!(report["draw_calls_p50"], json!(200.0));
        assert_eq!(report["draw_calls_max"], json!(300.0));
    }

    #[test]
    fn cpu_clocks_advance() {
        let (before_process, before_thread) = (process_cpu_ms(), thread_cpu_ms());
        let mut sum = 0u64;
        for n in 0..20_000_000u64 {
            sum = sum.wrapping_add(std::hint::black_box(n));
        }
        std::hint::black_box(sum);
        assert!(process_cpu_ms() > before_process);
        assert!(thread_cpu_ms() > before_thread);
    }

    #[cfg(feature = "perf-counters")]
    #[test]
    fn range_lengths() {
        assert_eq!(draw_layer::TARGET, "bevy_render::render_phase::draw_state");
    }
}
