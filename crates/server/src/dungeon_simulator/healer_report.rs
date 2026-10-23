//! Explicit capture/report commands, invoked as targeted ignored tests.
use super::healer_intake::Intake;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const SPECS: [(u32, &str); 7] = [
    (65, "Holy Paladin"),
    (256, "Discipline Priest"),
    (257, "Holy Priest"),
    (264, "Restoration Shaman"),
    (270, "Mistweaver Monk"),
    (105, "Restoration Druid"),
    (1468, "Preservation Evoker"),
];

fn output_directory() -> PathBuf {
    std::env::var_os("HEALER_DUNGEON_REPORT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| super::runtime::output_directory().join("healer-benchmark"))
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    std::fs::write(path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
}

fn profile_present(spec: u32) -> Result<bool, String> {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|error| error.to_string())?;
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM follower_profiles WHERE spec_id=?1 AND role=1)",
        [spec],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn format_metric(value: &Value, key: &str) -> String {
    value["metrics"][key]
        .as_f64()
        .map_or_else(|| "N/A".into(), |number| format!("{number:.3}"))
}

fn markdown(report: &Value) -> String {
    let mut text = format!(
        "# Dungeon healing metrics\n\nStockade map34 Normal, seed{}, level{}, same raw intake `{}`. No SimC healer HPS reference.\n\n| Spec | Coverage | Deaths | Overheal % | Ending mana | Idle GCD equivalents | Effective HPS |\n|---|---|---|---|---|---|---|\n",
        report["seed"],
        report["level"],
        report["intake_blake3"].as_str().unwrap()
    );
    for row in report["specs"].as_array().unwrap() {
        text.push_str(&format!(
            "| {} {} | {} | {} | {} | {} | {} | {} |\n",
            row["spec"],
            row["name"].as_str().unwrap(),
            row["status"].as_str().unwrap(),
            format_metric(row, "party_deaths"),
            format_metric(row, "overheal_pct"),
            format_metric(row, "mana_at_end"),
            format_metric(row, "idle_gcds"),
            format_metric(row, "effective_hps")
        ));
    }
    text.push_str("\n## Coverage gaps\n\n");
    for row in report["specs"].as_array().unwrap() {
        text.push_str(&format!("- Spec{}: {}.\n", row["spec"], row["gaps"]));
    }
    text.push_str("\nMetrics: healer-attributed party Heal receipts only; effective = amount − overheal. Overheal% uses raw healing as denominator. Effective HPS uses observed party combat seconds. Ending mana is the observed final replay frame, not the next pull's recovered pool. Idle GCD equivalents sum alive combat-frame time with no native cast/channel or active GCD, divided by the profile heal's native hasted GCD; this is a frame-sampled readiness metric, not a counter of hypothetical healing opportunities. Zero healing/combat denominators are null. Absorbed damage is preserved by native mitigation, but is not falsely counted as healing HPS.\n\nInput freezes source-side raw amounts, hit outcomes/crit flags, party slots/positions, source deaths and native pull order. Recipient mitigation is applied anew. Source brains are disabled during replay; source identity/scaling/spells remain native. Healer and companion decision policy remains unchanged. Future scripted-damage alerts are not reconstructed from a second autonomous route. Native partial spell resistance is an explicit engine gap where magic hits occur; post-mitigation recording is never used.\n");
    text
}

pub(super) fn report(intake: &Intake, bytes: &[u8], directory: &Path) -> Result<Value, String> {
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut rows = Vec::new();
    for (spec, name) in SPECS {
        let result = if profile_present(spec)? {
            super::healer_replay::replay(intake, spec)
        } else {
            Err("NATIVE_HEALER_PROFILE_ABSENT".into())
        };
        let mut row = result.unwrap_or_else(
            |gap| json!({"spec":spec,"status":"blocked","metrics":null,"gaps":[gap]}),
        );
        row["name"] = json!(name);
        write_json(&directory.join(format!("{spec}.json")), &row)?;
        rows.push(row);
    }
    let report = json!({"schema":1,"map_id":intake.map_id,"difficulty_id":intake.difficulty_id,"seed":intake.seed,
        "level":intake.level,"intake_blake3":blake3::hash(bytes).to_hex().to_string(),
        "intake_hits":intake.hits.len(),"intake_duration_ticks":intake.end_tick,
        "source_status":intake.source_status,"specs":rows});
    write_json(&directory.join("report.json"), &report)?;
    std::fs::write(directory.join("report.md"), markdown(&report))
        .map_err(|error| error.to_string())?;
    Ok(report)
}

#[test]
#[ignore = "explicit native Stockade capture plus two native healer replays"]
fn healer_benchmark_capture_stockade_and_report() {
    let baseline = super::campaign::run(1).unwrap();
    assert_eq!(
        baseline.status, "cleared",
        "record only actual completed dungeon pulls: {}",
        baseline.detail
    );
    let mut input = baseline.performance["healer_metrics"]["recorded_intake"].clone();
    input["seed"] = json!(baseline.seed);
    input["source_status"] = json!(baseline.status);
    let intake: Intake = serde_json::from_value(input).unwrap();
    assert!(!intake.hits.is_empty(), "native raw receipts required");
    assert!(
        intake.gaps.is_empty(),
        "incomplete raw capture: {:?}",
        intake.gaps
    );
    let directory = output_directory();
    std::fs::create_dir_all(&directory).unwrap();
    let bytes = serde_json::to_vec(&intake).unwrap();
    std::fs::write(directory.join("stockade-seed1.intake.json"), &bytes).unwrap();
    let report = report(&intake, &bytes, &directory).unwrap();
    println!("HEALER_DUNGEON_REPORT {} {}", directory.display(), report);
    for spec in [105, 257] {
        assert!(
            report["specs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["spec"] == spec && row["status"] == "native"),
            "native spec{spec} blocked: {report}"
        );
    }
}

#[test]
#[ignore = "explicit replay of persisted Stockade intake; no second route run"]
fn healer_benchmark_report_recorded_stockade() {
    let path = std::env::var_os("HEALER_DUNGEON_INTAKE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../content/dungeons/healer-benchmark/stockade-seed1.intake.json")
        });
    let bytes = std::fs::read(&path).unwrap();
    let intake: Intake = serde_json::from_slice(&bytes).unwrap();
    let result = report(&intake, &bytes, &output_directory()).unwrap();
    println!("HEALER_DUNGEON_REPORT {result}");
}
