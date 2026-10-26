//! Independent native samples and explicitly averaged measurement reports.
use super::{inputs, report};
use serde_json::{Value, json};

const DEFAULT_SEEDS: &[u64] = &[70_448, 1, 2];

pub(super) fn read_seeds() -> Result<Vec<u64>, String> {
    let configured = match std::env::var("DPS_PARITY_SEEDS") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(DEFAULT_SEEDS.to_vec()),
        Err(error) => return Err(format!("DPS_PARITY_SEEDS: {error}")),
    };
    configured
        .split(',')
        .map(|value| {
            value
                .trim()
                .parse::<u64>()
                .map_err(|error| format!("DPS_PARITY_SEEDS invalid seed {value:?}: {error}"))
        })
        .collect()
}

pub(super) fn write_seed_report(
    spec: u32,
    count: usize,
    seed: u64,
    report: &Value,
) -> Result<(), String> {
    let path = inputs::directory()?.join(format!("{spec}.native.{count}.seed-{seed}.json"));
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("serialize seed{seed} report: {error}"))?;
    std::fs::write(&path, bytes)
        .map_err(|error| format!("write seed{seed} report {}: {error}", path.display()))
}

pub(super) fn summarize(reports: Vec<Value>) -> Value {
    let mut summary = reports
        .first()
        .expect("at least one native seed report")
        .clone();
    let seeds: Vec<_> = reports
        .iter()
        .map(|report| report["seed"].clone())
        .collect();
    let samples: Vec<_> = reports
        .iter()
        .map(|report| json!({"seed": report["seed"], "dps": report["combat"]["dps"]}))
        .collect();
    let wall_seconds: f64 = reports
        .iter()
        .map(|report| {
            report["wall_seconds"]
                .as_f64()
                .expect("native sample timing")
        })
        .sum();
    summary["detail_seed"] = summary["seed"].clone();
    summary["seed"] = json!(seeds);
    summary["seed_dps"] = json!(samples);
    summary["combat"] = report::mean_combat(&reports);
    summary["wall_seconds"] = json!(wall_seconds);
    summary
}
