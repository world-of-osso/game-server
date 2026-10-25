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

mod network_payloads;

const REPORT_WINDOW: Duration = Duration::from_secs(10);
const REPORT_TOP: usize = 15;

#[derive(Default)]
struct SystemTotal {
    time: Duration,
    runs: u64,
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
        let mut totals = self.totals.lock().expect("system timings lock");
        let total = totals.entry(name.0.clone()).or_default();
        total.time += elapsed;
        total.runs += 1;
    }
}

/// `LogPlugin::custom_layer` hook: installs the layer and its report system.
pub fn system_timing_layer(app: &mut App) -> Option<BoxedLayer> {
    network_payloads::install(app);
    let totals = Totals::default();
    app.insert_resource(SystemTimings {
        totals: totals.clone(),
        window_start: Instant::now(),
    })
    .add_systems(Last, report_system_timings);
    Some(Box::new(SystemTimingLayer { totals }))
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
            "  {:6.2} ms/s {:5.1}% {:6} runs {:.3} ms/run  {name}",
            ms * 1000.0 / wall_ms,
            ms * 100.0 / wall_ms,
            total.runs,
            ms / total.runs.max(1) as f64,
        );
    }
}
