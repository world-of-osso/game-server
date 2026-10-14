//! Outer-frame and fixed-step accounting for the tick log.
//!
//! The server targets one 50 ms fixed step per outer update. When an update
//! takes longer, Bevy runs catch-up steps and discards time beyond its 250 ms
//! clamp, so the simulation falls behind wall time. This module reports, per
//! wall-clock window, how many fixed steps ran and how long outer updates
//! took, which makes that lag observable in the log.

use bevy::prelude::*;
use std::time::{Duration, Instant};

/// Wall-clock length of one reporting window.
const REPORT_WINDOW: Duration = Duration::from_secs(10);

#[derive(Resource)]
pub struct FrameStats {
    window_start: Instant,
    frame_start: Option<Instant>,
    frames: u32,
    fixed_steps: u32,
    total_work: Duration,
    max_work: Duration,
    work_samples: Vec<Duration>,
    over_budget: u32,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self::starting_at(Instant::now())
    }
}

/// One finished reporting window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameReport {
    pub wall: Duration,
    pub frames: u32,
    pub fixed_steps: u32,
    pub mean_work: Duration,
    pub max_work: Duration,
    /// Nearest-rank p99 over outer frames in this reporting window.
    pub p99_work: Duration,
    pub over_budget: u32,
}

impl FrameReport {
    /// Fixed steps per wall second; 20.0 means the simulation keeps real time.
    pub fn ticks_per_second(&self) -> f64 {
        f64::from(self.fixed_steps) / self.wall.as_secs_f64()
    }
}

impl FrameStats {
    pub fn starting_at(now: Instant) -> Self {
        Self {
            window_start: now,
            frame_start: None,
            frames: 0,
            fixed_steps: 0,
            total_work: Duration::ZERO,
            max_work: Duration::ZERO,
            work_samples: Vec::new(),
            over_budget: 0,
        }
    }

    pub fn begin_frame(&mut self, now: Instant) {
        self.frame_start = Some(now);
    }

    pub fn record_fixed_step(&mut self) {
        self.fixed_steps += 1;
    }

    /// Closes the current frame; returns a report when the window has elapsed.
    pub fn end_frame(&mut self, now: Instant, budget: Duration) -> Option<FrameReport> {
        if let Some(start) = self.frame_start.take() {
            let work = now.saturating_duration_since(start);
            self.frames += 1;
            self.total_work += work;
            self.max_work = self.max_work.max(work);
            self.work_samples.push(work);
            self.over_budget += u32::from(work > budget);
        }
        let wall = now.saturating_duration_since(self.window_start);
        if wall < REPORT_WINDOW {
            return None;
        }
        let report = FrameReport {
            wall,
            frames: self.frames,
            fixed_steps: self.fixed_steps,
            mean_work: self.total_work / self.frames.max(1),
            max_work: self.max_work,
            p99_work: nearest_rank_p99(&mut self.work_samples),
            over_budget: self.over_budget,
        };
        *self = Self::starting_at(now);
        Some(report)
    }
}

fn nearest_rank_p99(samples: &mut [Duration]) -> Duration {
    if samples.is_empty() {
        return Duration::ZERO;
    }
    let rank = (samples.len() * 99).div_ceil(100) - 1;
    let (_, percentile, _) = samples.select_nth_unstable(rank);
    *percentile
}

pub struct FrameStatsPlugin {
    pub budget: Duration,
}

#[derive(Resource)]
struct FrameBudget(Duration);

impl Plugin for FrameStatsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameStats>()
            .insert_resource(FrameBudget(self.budget))
            .add_systems(First, begin_frame)
            .add_systems(FixedFirst, record_fixed_step)
            .add_systems(Last, end_frame);
    }
}

fn begin_frame(mut stats: ResMut<FrameStats>) {
    stats.begin_frame(Instant::now());
}

fn record_fixed_step(mut stats: ResMut<FrameStats>) {
    stats.record_fixed_step();
}

fn end_frame(mut stats: ResMut<FrameStats>, budget: Res<FrameBudget>) {
    let Some(report) = stats.end_frame(Instant::now(), budget.0) else {
        return;
    };
    info!(
        "Frames: {:.1} ticks/s ({} steps, {} frames in {:.1}s), work mean {:.1} ms max {:.1} ms, {} over budget, p99 {:.1} ms",
        report.ticks_per_second(),
        report.fixed_steps,
        report.frames,
        report.wall.as_secs_f64(),
        report.mean_work.as_secs_f64() * 1000.0,
        report.max_work.as_secs_f64() * 1000.0,
        report.over_budget,
        report.p99_work.as_secs_f64() * 1000.0,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUDGET: Duration = Duration::from_millis(50);

    fn run_frame(
        stats: &mut FrameStats,
        start: Instant,
        work_ms: u64,
        steps: u32,
    ) -> Option<FrameReport> {
        stats.begin_frame(start);
        for _ in 0..steps {
            stats.record_fixed_step();
        }
        stats.end_frame(start + Duration::from_millis(work_ms), BUDGET)
    }

    #[test]
    fn p99_uses_nearest_rank_and_does_not_retain_an_older_window() {
        let t0 = Instant::now();
        let mut stats = FrameStats::starting_at(t0);
        let mut now = t0;
        for index in 0..100 {
            let work_ms = if index == 0 { 400 } else { 10 };
            assert_eq!(run_frame(&mut stats, now, work_ms, 1), None);
            now += Duration::from_millis(work_ms + 1);
        }
        let report = stats.end_frame(t0 + REPORT_WINDOW, BUDGET).unwrap();
        assert_eq!(report.frames, 100);
        assert_eq!(report.p99_work, Duration::from_millis(10));
        assert_eq!(report.max_work, Duration::from_millis(400));
        assert_eq!(report.over_budget, 1);
        assert_eq!(run_frame(&mut stats, t0 + REPORT_WINDOW, 25, 1), None);
        let next = stats.end_frame(t0 + REPORT_WINDOW * 2, BUDGET).unwrap();
        assert_eq!(next.p99_work, Duration::from_millis(25));
    }

    #[test]
    fn report_waits_for_the_window_then_resets() {
        let t0 = Instant::now();
        let mut stats = FrameStats::starting_at(t0);
        assert_eq!(run_frame(&mut stats, t0, 10, 1), None);
        let report = run_frame(&mut stats, t0 + REPORT_WINDOW, 30, 1).expect("window elapsed");
        assert_eq!(report.frames, 2);
        assert_eq!(report.fixed_steps, 2);
        assert_eq!(report.mean_work, Duration::from_millis(20));
        assert_eq!(report.max_work, Duration::from_millis(30));
        assert_eq!(report.over_budget, 0);
        assert_eq!(stats.frames, 0);
        assert_eq!(stats.fixed_steps, 0);
    }

    #[test]
    fn slow_frames_count_over_budget_and_lower_tick_rate() {
        let t0 = Instant::now();
        let mut stats = FrameStats::starting_at(t0);
        // 400 ms frames run at most 5 steps (Bevy's 250 ms clamp):
        // 25 frames = 10 s wall but only 125 steps.
        let mut report = None;
        for i in 0..25u64 {
            report = run_frame(&mut stats, t0 + Duration::from_millis(400 * i), 400, 5);
        }
        let report = report.expect("window elapsed");
        assert_eq!(report.over_budget, 25);
        assert_eq!(report.fixed_steps, 125);
        assert!((report.ticks_per_second() - 12.5).abs() < 0.01);
    }
}
