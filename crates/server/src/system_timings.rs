//! Per-system wall time, built with `--features system-timings`.
//!
//! Bevy's `trace` feature wraps every system run in an `info_span!("system", name)`.
//! This layer sums the time spent inside those spans and the server logs the
//! most expensive systems once per wall-clock window. Diagnostic only: span
//! bookkeeping adds overhead to every system run.

use bevy::log::BoxedLayer;
use bevy::log::tracing::field::{Field, Visit};
use bevy::log::tracing::span::{Attributes, Id};
use bevy::log::tracing::{Subscriber, info};
use bevy::log::tracing_subscriber::Layer;
use bevy::log::tracing_subscriber::layer::Context;
use bevy::log::tracing_subscriber::registry::LookupSpan;
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const REPORT_WINDOW: Duration = Duration::from_secs(10);
const REPORT_TOP: usize = 15;
const SYSTEM_STALL_BUDGET: Duration = Duration::from_millis(50);

#[derive(Default)]
struct SystemTotal {
    time: Duration,
    runs: u64,
    max: Duration,
}

type Totals = Arc<Mutex<HashMap<String, SystemTotal>>>;

#[derive(Resource)]
struct SystemTimings {
    totals: Totals,
    window_start: Instant,
}

struct SystemName(String);
struct Entered(Instant);

struct NameVisitor(Option<String>);

impl Visit for NameVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "name" {
            self.0 = Some(value.to_owned());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "name" {
            self.0 = Some(format!("{value:?}"));
        }
    }
}

struct SystemTimingLayer {
    totals: Totals,
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for SystemTimingLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if attrs.metadata().name() != "system" {
            return;
        }
        let mut visitor = NameVisitor(None);
        attrs.record(&mut visitor);
        if let (Some(name), Some(span)) = (visitor.0, ctx.span(id)) {
            span.extensions_mut().insert(SystemName(name));
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let mut ext = span.extensions_mut();
        if ext.get_mut::<SystemName>().is_some() {
            ext.replace(Entered(Instant::now()));
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let ext = span.extensions();
        let (Some(name), Some(entered)) = (ext.get::<SystemName>(), ext.get::<Entered>()) else {
            return;
        };
        let elapsed = entered.0.elapsed();
        let name = name.0.clone();
        drop(ext);
        self.record_system_work(&name, elapsed);
        if elapsed > SYSTEM_STALL_BUDGET {
            bevy::log::tracing::warn!(
                system = %name,
                system_work_ms = elapsed.as_secs_f64() * 1000.0,
                "System stall"
            );
        }
    }
}

impl SystemTimingLayer {
    fn record_system_work(&self, name: &str, elapsed: Duration) {
        let mut totals = self.totals.lock().expect("system timings lock");
        let total = totals.entry(name.to_string()).or_default();
        total.time += elapsed;
        total.runs += 1;
        total.max = total.max.max(elapsed);
    }
}

/// `LogPlugin::custom_layer` hook: installs the layer and its report system.
pub fn system_timing_layer(app: &mut App) -> Option<BoxedLayer> {
    let totals = Totals::default();
    app.insert_resource(SystemTimings {
        totals: totals.clone(),
        window_start: Instant::now(),
    })
    .add_systems(Last, report_system_timings);
    Some(Box::new(SystemTimingLayer { totals }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::log::tracing_subscriber::prelude::*;
    use std::io::Write;

    #[derive(Clone)]
    struct CapturedLog(Arc<Mutex<Vec<u8>>>);

    impl Write for CapturedLog {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn slow_system_logs_its_name_and_elapsed_work_before_window_end() {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = CapturedLog(Arc::clone(&bytes));
        let subscriber = bevy::log::tracing_subscriber::registry()
            .with(SystemTimingLayer {
                totals: Totals::default(),
            })
            .with(
                bevy::log::tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .without_time()
                    .with_writer(move || writer.clone()),
            );
        bevy::log::tracing::subscriber::with_default(subscriber, || {
            let span = bevy::log::tracing::info_span!("system", name = "deliberate_fixture_stall");
            let _entered = span.enter();
            std::thread::sleep(Duration::from_millis(60));
        });
        let output = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
        assert!(
            output.contains("System stall"),
            "no immediate stall attribution: {output}"
        );
        assert!(
            output.contains("deliberate_fixture_stall"),
            "missing slow system name: {output}"
        );
        assert!(
            output.contains("system_work_ms="),
            "missing measured work: {output}"
        );
    }
}

fn report_system_timings(mut timings: ResMut<SystemTimings>) {
    let wall = timings.window_start.elapsed();
    if wall < REPORT_WINDOW {
        return;
    }
    timings.window_start = Instant::now();
    let mut rows: Vec<(String, SystemTotal)> = timings
        .totals
        .lock()
        .expect("system timings lock")
        .drain()
        .collect();
    rows.sort_by_key(|(_, total)| std::cmp::Reverse(total.time));
    let all: Duration = rows.iter().map(|(_, total)| total.time).sum();
    let wall_ms = wall.as_secs_f64() * 1000.0;
    info!(
        "System timings: {} systems, {:.1} ms busy per wall second",
        rows.len(),
        all.as_secs_f64() * 1000.0 / wall.as_secs_f64()
    );
    for (name, total) in rows.iter().take(REPORT_TOP) {
        let ms = total.time.as_secs_f64() * 1000.0;
        info!(
            "  {:6.2} ms/s {:5.1}% {:6} runs {:.3} ms/run max {:.3} ms  {name}",
            ms * 1000.0 / wall_ms,
            ms * 100.0 / wall_ms,
            total.runs,
            ms / total.runs.max(1) as f64,
            total.max.as_secs_f64() * 1000.0,
        );
    }
}
