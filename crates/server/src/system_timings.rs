//! System and inclusive schedule wall time, built with `--features system-timings`.
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    System,
    Schedule,
    Preparation,
}

struct TimedScope {
    kind: ScopeKind,
    name: String,
}

struct Entered(Instant);

#[derive(Default)]
struct TimingFields {
    name: Option<String>,
}

impl Visit for TimingFields {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "name" {
            self.name = Some(value.to_owned());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "name" {
            self.name = Some(format!("{value:?}"));
        }
    }
}

#[derive(serde::Serialize)]
struct PhaseMeasurement {
    phase: String,
    preparation: bool,
    work_ms: f64,
}

// Parent/child schedule rows overlap; they must not be summed.
#[derive(Default)]
struct MainPhases(Vec<PhaseMeasurement>);

#[derive(Default)]
struct SystemTimingLayer {
    totals: Totals,
    // Bevy system spans explicitly have no parent; ancestry loses enclosing schedules.
    active_schedules: Mutex<HashMap<std::thread::ThreadId, Vec<Id>>>,
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for SystemTimingLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let kind = match attrs.metadata().name() {
            "system" => ScopeKind::System,
            "schedule" => ScopeKind::Schedule,
            "schedule_preparation" => ScopeKind::Preparation,
            _ => return,
        };
        let mut fields = TimingFields::default();
        attrs.record(&mut fields);
        if let (Some(name), Some(span)) = (fields.name, ctx.span(id)) {
            let mut ext = span.extensions_mut();
            if kind == ScopeKind::Schedule && name == "Main" {
                ext.insert(MainPhases::default());
            }
            ext.insert(TimedScope { kind, name });
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let mut ext = span.extensions_mut();
        let Some(scope) = ext.get_mut::<TimedScope>() else {
            return;
        };
        if scope.kind == ScopeKind::Schedule {
            self.active_schedules
                .lock()
                .expect("active schedules lock")
                .entry(std::thread::current().id())
                .or_default()
                .push(id.clone());
        }
        ext.replace(Entered(Instant::now()));
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let ext = span.extensions();
        let (Some(scope), Some(entered)) = (ext.get::<TimedScope>(), ext.get::<Entered>()) else {
            return;
        };
        let (kind, name, elapsed) = (scope.kind, scope.name.clone(), entered.0.elapsed());
        drop(ext);
        if kind == ScopeKind::Preparation {
            self.record_preparation(&ctx, &name);
            return;
        }
        if kind == ScopeKind::System {
            self.record_system_work(&name, elapsed);
        } else if name == "Main" {
            report_main_phases(&ctx, id, elapsed);
        } else {
            self.record_main_phase(
                &ctx,
                PhaseMeasurement {
                    phase: name.clone(),
                    preparation: false,
                    work_ms: elapsed.as_secs_f64() * 1000.0,
                },
            );
        }
        if kind == ScopeKind::Schedule {
            self.pop_schedule(id);
        }
        if elapsed > SYSTEM_STALL_BUDGET {
            log_scope_stall(kind, &name, elapsed);
        }
    }
}

fn log_scope_stall(kind: ScopeKind, name: &str, elapsed: Duration) {
    match kind {
        ScopeKind::System => bevy::log::tracing::warn!(system = %name,
            system_work_ms = elapsed.as_secs_f64() * 1000.0, "System stall"),
        ScopeKind::Schedule => bevy::log::tracing::warn!(schedule = %name,
            schedule_work_ms = elapsed.as_secs_f64() * 1000.0, "Schedule stall"),
        ScopeKind::Preparation => bevy::log::tracing::warn!(schedule = %name,
            schedule_prepare_ms = elapsed.as_secs_f64() * 1000.0, "Schedule preparation stall"),
    }
}

impl SystemTimingLayer {
    fn pop_schedule(&self, id: &Id) {
        let mut active = self.active_schedules.lock().expect("active schedules lock");
        let thread = std::thread::current().id();
        let stack = active.get_mut(&thread).expect("entered schedule stack");
        assert_eq!(stack.pop().as_ref(), Some(id), "schedule exit order");
        if stack.is_empty() {
            active.remove(&thread);
        }
    }

    fn active_schedule_ids(&self) -> Vec<Id> {
        self.active_schedules
            .lock()
            .expect("active schedules lock")
            .get(&std::thread::current().id())
            .cloned()
            .unwrap_or_default()
    }

    fn measure_preparation<S: Subscriber + for<'a> LookupSpan<'a>>(
        &self,
        ctx: &Context<'_, S>,
        name: &str,
    ) -> Option<Duration> {
        self.active_schedule_ids().iter().rev().find_map(|id| {
            let span = ctx.span(id)?;
            let ext = span.extensions();
            let scope = ext.get::<TimedScope>()?;
            if scope.name != name {
                return None;
            }
            Some(ext.get::<Entered>()?.0.elapsed())
        })
    }

    fn record_preparation<S: Subscriber + for<'a> LookupSpan<'a>>(
        &self,
        ctx: &Context<'_, S>,
        name: &str,
    ) {
        let Some(elapsed) = self.measure_preparation(ctx, name) else {
            bevy::log::warn!(
                schedule = name,
                "Schedule preparation lacks an active schedule timer"
            );
            return;
        };
        self.record_main_phase(
            ctx,
            PhaseMeasurement {
                phase: name.to_owned(),
                preparation: true,
                work_ms: elapsed.as_secs_f64() * 1000.0,
            },
        );
        if elapsed > SYSTEM_STALL_BUDGET {
            log_scope_stall(ScopeKind::Preparation, name, elapsed);
        }
    }

    fn record_main_phase<S: Subscriber + for<'a> LookupSpan<'a>>(
        &self,
        ctx: &Context<'_, S>,
        measurement: PhaseMeasurement,
    ) {
        for id in self.active_schedule_ids() {
            let Some(span) = ctx.span(&id) else { continue };
            let mut ext = span.extensions_mut();
            if let Some(phases) = ext.get_mut::<MainPhases>() {
                phases.0.push(measurement);
                return;
            }
        }
    }
}

fn report_main_phases<S: Subscriber + for<'a> LookupSpan<'a>>(
    ctx: &Context<'_, S>,
    id: &Id,
    elapsed: Duration,
) {
    let Some(span) = ctx.span(id) else { return };
    let Some(phases) = span.extensions_mut().remove::<MainPhases>() else {
        return;
    };
    if elapsed <= SYSTEM_STALL_BUDGET {
        return;
    }
    let json = serde_json::to_string(&phases.0).expect("serialize measured schedule phases");
    info!(outer_schedule_ms = elapsed.as_secs_f64() * 1000.0, phases_json = %json, "Main schedule breakdown");
}

// ScheduleBuilt fires after graph/system initialization, before executor initialization/run.
fn report_schedule_prepared(event: On<bevy::ecs::schedule::ScheduleBuilt>) {
    let _notice =
        bevy::log::tracing::info_span!("schedule_preparation", name = ?event.label).entered();
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
    .add_systems(Last, report_system_timings)
    .add_observer(report_schedule_prepared);
    Some(Box::new(SystemTimingLayer {
        totals,
        ..Default::default()
    }))
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

    fn captured_phases(output: &str) -> Vec<serde_json::Value> {
        let line = output
            .lines()
            .find(|line| line.contains("Main schedule breakdown"))
            .unwrap_or_else(|| panic!("missing outer schedule breakdown: {output}"));
        let (_, json) = line
            .split_once("phases_json=")
            .expect("measured phase payload");
        serde_json::from_str(json).expect("valid measured phase JSON")
    }

    #[test]
    fn slow_system_logs_its_name_and_elapsed_work_before_window_end() {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = CapturedLog(Arc::clone(&bytes));
        let subscriber = bevy::log::tracing_subscriber::registry()
            .with(SystemTimingLayer::default())
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
    #[test]
    fn slow_schedule_preparation_is_measured_before_system_execution() {
        struct SlowInitialState;
        impl FromWorld for SlowInitialState {
            fn from_world(_world: &mut World) -> Self {
                std::thread::sleep(Duration::from_millis(60));
                Self
            }
        }
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = CapturedLog(Arc::clone(&bytes));
        let mut app = App::new();
        app.add_systems(Update, |_: Local<SlowInitialState>| {});
        let layer = system_timing_layer(&mut app).unwrap();
        let subscriber = bevy::log::tracing_subscriber::registry().with(layer).with(
            bevy::log::tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .without_time()
                .with_writer(move || writer.clone()),
        );
        bevy::log::tracing::subscriber::with_default(subscriber, || {
            app.update();
        });
        let output = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
        assert!(
            output.contains("Schedule preparation stall"),
            "missing pre-execution attribution: {output}"
        );
        assert!(
            output.contains("schedule=Update"),
            "missing initialized schedule: {output}"
        );
        assert!(
            output.contains("schedule_prepare_ms="),
            "missing measured preparation: {output}"
        );
        let phases = captured_phases(&output);
        let prepared = phases
            .iter()
            .find(|phase| phase["phase"] == "Update" && phase["preparation"] == true)
            .unwrap_or_else(|| panic!("missing measured Update preparation: {output}"));
        assert!(prepared["work_ms"].as_f64().unwrap() >= 50.0);
    }

    #[test]
    fn slow_deferred_command_is_attributed_to_its_enclosing_schedule() {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = CapturedLog(Arc::clone(&bytes));
        let subscriber = bevy::log::tracing_subscriber::registry()
            .with(SystemTimingLayer::default())
            .with(
                bevy::log::tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .without_time()
                    .with_writer(move || writer.clone()),
            );
        bevy::log::tracing::subscriber::with_default(subscriber, || {
            let mut app = App::new();
            app.add_systems(Update, |mut commands: Commands| {
                commands.queue(|_world: &mut World| {
                    std::thread::sleep(Duration::from_millis(60));
                });
            });
            app.update();
            app.update();
        });
        let output = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
        assert!(
            output.contains("Schedule stall"),
            "deferred work has no enclosing schedule attribution: {output}"
        );
        assert!(
            output.contains("schedule=Update"),
            "missing actual Update phase: {output}"
        );
        assert!(
            output.contains("schedule_work_ms="),
            "missing measured inclusive phase work: {output}"
        );
        let phases = captured_phases(&output);
        let update = phases
            .iter()
            .find(|phase| phase["phase"] == "Update" && phase["preparation"] == false)
            .unwrap_or_else(|| panic!("missing measured Update execution/deferred work: {output}"));
        assert!(update["work_ms"].as_f64().unwrap() >= 50.0);
        let deferred = output
            .lines()
            .find(|line| line.contains("Deferred command stall"))
            .unwrap_or_else(|| panic!("deferred producer is not attributed: {output}"));
        assert!(
            deferred.contains("slow_deferred_command_is_attributed_to_its_enclosing_schedule"),
            "deferred work lost its producing system: {deferred}"
        );
        assert_eq!(
            output.matches("Main schedule breakdown").count(),
            2,
            "each update must retain its own phase measurements: {output}"
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
