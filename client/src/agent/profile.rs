//! Per-system timing for the bench (feature `profile`).
//!
//! `bevy_ecs/trace` wraps every system run in a `system` tracing span (and every schedule in a
//! `schedule` span). A custom subscriber layer (installed through `LogPlugin::custom_layer`)
//! times each span enter/exit and aggregates by name: count, total, max, plus a log of every span
//! slower than `LONG_SPAN_MS` so that the bench can attribute the spike frames. Without the
//! feature this module only exposes `log_plugin()` (the default `LogPlugin`) and `report()` = null.
//!
//! No Chrome / Tracy crate is needed (they are not in the offline registry).

use bevy::log::LogPlugin;
use serde_json::Value;

/// `LogPlugin` to register in `DefaultPlugins`: with feature `profile`, it carries the timing layer.
pub fn log_plugin() -> LogPlugin {
    #[cfg(feature = "profile")]
    {
        LogPlugin {
            custom_layer: imp::layer,
            ..Default::default()
        }
    }
    #[cfg(not(feature = "profile"))]
    {
        LogPlugin::default()
    }
}

/// Milliseconds since the profiler started (same clock as the spans); 0 without the feature.
pub fn now_ms() -> f64 {
    #[cfg(feature = "profile")]
    {
        imp::now_ms()
    }
    #[cfg(not(feature = "profile"))]
    {
        0.0
    }
}

/// Clears the aggregates (called when the measured phase starts).
pub fn reset() {
    #[cfg(feature = "profile")]
    imp::reset();
}

/// JSON report: top systems by total and by max, and the long spans inside each spike window
/// (`spikes` = `(end_ms, frame_ms)` of the slow frames, on the `now_ms` clock).
pub fn report(spikes: &[(f64, f32)]) -> Value {
    #[cfg(feature = "profile")]
    {
        imp::report(spikes)
    }
    #[cfg(not(feature = "profile"))]
    {
        let _ = spikes;
        Value::Null
    }
}

#[cfg(feature = "profile")]
mod imp {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;

    use bevy::prelude::*;
    use serde_json::{Value, json};
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id};
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::Context;
    use tracing_subscriber::registry::LookupSpan;

    /// Spans slower than this are logged individually.
    const LONG_SPAN_MS: f64 = 1.5;
    const LONG_SPAN_CAP: usize = 20_000;

    fn clock() -> &'static Instant {
        static START: OnceLock<Instant> = OnceLock::new();
        START.get_or_init(Instant::now)
    }

    pub fn now_ms() -> f64 {
        clock().elapsed().as_secs_f64() * 1000.0
    }

    #[derive(Default)]
    struct Agg {
        count: u64,
        total: f64,
        max: f64,
    }

    #[derive(Default)]
    struct Store {
        by_name: HashMap<String, Agg>,
        long: Vec<(f64, f64, String)>,
    }

    fn store() -> &'static Mutex<Store> {
        static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
        STORE.get_or_init(Mutex::default)
    }

    pub fn reset() {
        if let Ok(mut store) = store().lock() {
            *store = Store::default();
        }
    }

    struct SpanInfo {
        label: String,
        entered: Option<Instant>,
    }

    #[derive(Default)]
    struct NameVisitor(Option<String>);

    impl Visit for NameVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "name" {
                self.0 = Some(format!("{value:?}").trim_matches('"').to_string());
            }
        }
        fn record_str(&mut self, field: &Field, value: &str) {
            if field.name() == "name" {
                self.0 = Some(value.to_string());
            }
        }
    }

    struct TimingLayer;

    impl<S> Layer<S> for TimingLayer
    where
        S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
            let mut visitor = NameVisitor::default();
            attrs.record(&mut visitor);
            let kind = attrs.metadata().name();
            let label = match visitor.0 {
                Some(name) => format!("{kind}:{name}"),
                None => kind.to_string(),
            };
            if let Some(span) = ctx.span(id) {
                span.extensions_mut().insert(SpanInfo {
                    label,
                    entered: None,
                });
            }
        }

        fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
            if let Some(span) = ctx.span(id)
                && let Some(info) = span.extensions_mut().get_mut::<SpanInfo>()
            {
                info.entered = Some(Instant::now());
            }
        }

        fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
            let Some(span) = ctx.span(id) else { return };
            let mut extensions = span.extensions_mut();
            let Some(info) = extensions.get_mut::<SpanInfo>() else {
                return;
            };
            let Some(entered) = info.entered.take() else {
                return;
            };
            let ms = entered.elapsed().as_secs_f64() * 1000.0;
            let end = now_ms();
            if let Ok(mut store) = store().lock() {
                let agg = store.by_name.entry(info.label.clone()).or_default();
                agg.count += 1;
                agg.total += ms;
                agg.max = agg.max.max(ms);
                if ms >= LONG_SPAN_MS && store.long.len() < LONG_SPAN_CAP {
                    store.long.push((end, ms, info.label.clone()));
                }
            }
        }
    }

    pub fn layer(_app: &mut App) -> Option<bevy::log::BoxedLayer> {
        let _ = clock();
        Some(Box::new(TimingLayer))
    }

    fn r(v: f64) -> f64 {
        (v * 100.0).round() / 100.0
    }

    pub fn report(spikes: &[(f64, f32)]) -> Value {
        let Ok(store) = store().lock() else {
            return Value::Null;
        };
        let mut rows: Vec<_> = store.by_name.iter().collect();
        rows.sort_by(|a, b| b.1.total.total_cmp(&a.1.total));
        let by_total: Vec<Value> = rows
            .iter()
            .take(30)
            .map(|(name, a)| {
                json!({"span": name, "count": a.count, "total_ms": r(a.total),
                "mean_ms": r(a.total / a.count.max(1) as f64), "max_ms": r(a.max)})
            })
            .collect();
        rows.sort_by(|a, b| b.1.max.total_cmp(&a.1.max));
        let by_max: Vec<Value> = rows
            .iter()
            .take(25)
            .map(|(name, a)| {
                json!({"span": name, "count": a.count, "max_ms": r(a.max),
                "mean_ms": r(a.total / a.count.max(1) as f64)})
            })
            .collect();
        // Spans (systems only) ending inside each spike frame window, biggest first.
        let spike_rows: Vec<Value> = spikes
            .iter()
            .take(40)
            .map(|(end, frame_ms)| {
                let start = end - *frame_ms as f64;
                let mut inside: Vec<_> = store
                    .long
                    .iter()
                    .filter(|(e, _, label)| {
                        *e >= start && *e <= *end + 1.0 && label.starts_with("system")
                    })
                    .collect();
                inside.sort_by(|a, b| b.1.total_cmp(&a.1));
                let top: Vec<Value> = inside
                    .iter()
                    .take(5)
                    .map(|(_, ms, label)| json!([label, r(*ms)]))
                    .collect();
                json!({"at_ms": r(*end), "frame_ms": r(*frame_ms as f64), "top_systems": top})
            })
            .collect();
        json!({"by_total": by_total, "by_max": by_max, "spike_frames": spike_rows})
    }
}
