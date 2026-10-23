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
        "# Dungeon healing metrics\n\nStockade map34 Normal, seed{}, level{}, same raw intake `{}`. No SimC healer HPS reference.\n\n| Spec | Coverage | Deaths | Overheal % | Ending mana | Idle GCD equivalents | Consumed healer absorbs | Effective HPS | Heal+ConsumedPoints/s | Native combat seconds |\n|---|---|---|---|---|---|---|---|---|---|\n",
        report["seed"],
        report["level"],
        report["intake_blake3"].as_str().unwrap()
    );
    for row in report["specs"].as_array().unwrap() {
        text.push_str(&format!(
            "| {} {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            row["spec"],
            row["name"].as_str().unwrap(),
            row["status"].as_str().unwrap(),
            format_metric(row, "party_deaths"),
            format_metric(row, "overheal_pct"),
            format_metric(row, "mana_at_end"),
            format_metric(row, "idle_gcds"),
            format_metric(row, "absorbed_damage"),
            format_metric(row, "effective_hps"),
            format_metric(row, "effective_throughput_hps"),
            format_metric(row, "native_combat_seconds")
        ));
    }
    let scale = &report["recorded_level_scaling"];
    let unit_note = if scale["non_unit_hits"] == 0 {
        "ABSORB_POINTS_VS_PREVENTED_HP_LEVEL_SCALING is moot for this captured intake"
    } else {
        "ABSORB_POINTS_VS_PREVENTED_HP_LEVEL_SCALING retained; points are not normalized prevented HP"
    };
    text.push_str(&format!("\n## Level scaling\n\n{}/{} captured hits have unit scale; {} non-unit. {unit_note}. Factor = native damage multiplier / victim health multiplier, before integer rounding.\n\n| Spec | Replayed non-unit/arrivals | Absorb points on non-unit hits | Share of consumed points |\n|---|---|---|---|\n",scale["unit_hits"],scale["hits"],scale["non_unit_hits"]));
    for row in report["specs"].as_array().unwrap() {
        if row["status"] != "native" {
            text.push_str(&format!("| {} | not run | N/A | N/A |\n", row["spec"]));
            continue;
        }
        let scaling = &row["level_scaling"];
        let share = scaling["absorbed_points_non_unit_share"]
            .as_f64()
            .map_or_else(
                || "N/A (no consumed points)".to_string(),
                |ratio| format!("{:.3}%", 100.0 * ratio),
            );
        text.push_str(&format!(
            "| {} | {}/{} | {} | {share} |\n",
            row["spec"],
            scaling["replayed_non_unit_hits"],
            scaling["replayed_hits"],
            scaling["absorbed_points_on_non_unit_hits"]
        ));
    }
    text.push_str("\n## Coverage gaps\n\n");
    for row in report["specs"].as_array().unwrap() {
        text.push_str(&format!("- Spec{}: {}.\n", row["spec"], row["gaps"]));
    }
    text.push_str("\nMetrics: healer-attributed party Heal receipts only; effective = amount − overheal. Overheal% uses raw healing as denominator. Effective HPS and Heal+Absorb/s use the frozen intake's recorded party-combat window (identical across specs even after an early wipe); native observed combat seconds remain separate. Ending mana is the observed final replay frame, not the next pull's recovered pool. Idle GCD equivalents sum alive combat-frame time with no native cast/channel or active GCD, divided by the profile heal's native hasted GCD; this is a frame-sampled readiness metric, not a counter of hypothetical healing opportunities. Zero healing/combat denominators are null. Absorbs count only healer-attributed SCHOOL_ABSORB points actually consumed on party by native damage handling, never applied/unconsumed shield capacity and never duplicated from combat-log absorbed totals. Combined throughput follows the native damage-meter convention: (effective Heal HP + consumed healer shield points) / recorded combat window. Points are not normalized prevented HP; non-unit scale retains ABSORB_POINTS_VS_PREVENTED_HP_LEVEL_SCALING with hit counts and consumed-point share.\n\nInput freezes source-side raw amounts, hit outcomes/crit flags, party slots/positions, source deaths and native pull order. Recipient mitigation is applied anew. Source brains are disabled during replay; source identity/scaling/spells remain native. Healer and companion decision policy remains unchanged. Future scripted-damage alerts are not reconstructed from a second autonomous route. Native partial spell resistance is an explicit engine gap where magic hits occur; post-mitigation recording is never used.\n");
    text
}

fn write_per_spec_markdown(_report: &Value, _directory: &Path) -> Result<(), String> {
    Ok(())
}

#[test]
#[ignore = "RED: standalone per-spec Markdown artifact missing"]
fn healer_benchmark_writes_standalone_per_spec_markdown() {
    std::fs::create_dir_all("/var/tmp/osso").unwrap();
    let directory = tempfile::tempdir_in("/var/tmp/osso").unwrap();
    let report = json!({"seed":1,"level":32,"intake_blake3":"concrete-fight-input",
    "recorded_level_scaling":{"hits":2,"unit_hits":2,"non_unit_hits":0},
    "specs":[
        {"spec":105,"name":"Restoration Druid","status":"native","gaps":[],
         "level_scaling":{"replayed_hits":2,"replayed_non_unit_hits":0,"absorbed_points_on_non_unit_hits":0.0,"absorbed_points_non_unit_share":0.0},
         "metrics":{"party_deaths":1,"overheal_pct":25.0,"mana_at_end":123.0,"idle_gcds":2.0,
             "absorbed_damage":200.0,"effective_hps":50.0,"effective_throughput_hps":116.6666666667,"native_combat_seconds":3.0}},
        {"spec":65,"name":"Holy Paladin","status":"blocked","metrics":null,"gaps":["NATIVE_HEALER_PROFILE_ABSENT"]}
    ]});
    write_per_spec_markdown(&report, directory.path()).unwrap();
    let druid =
        std::fs::read_to_string(directory.path().join("105.md")).expect("standalone native report");
    assert!(druid.contains("| 105 Restoration Druid | native |"));
    assert!(druid.contains("116.667"));
    assert!(druid.contains("2/2 captured hits have unit scale"));
    assert!(!druid.contains("Holy Paladin"));
    let paladin =
        std::fs::read_to_string(directory.path().join("65.md")).expect("standalone blocked report");
    assert!(paladin.contains("NATIVE_HEALER_PROFILE_ABSENT"));
    assert!(paladin.contains("N/A"));
    assert!(!paladin.contains("Restoration Druid"));
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
    let non_unit = super::healer_intake::non_unit_hits(&intake.hits);
    let report = json!({"schema":1,"map_id":intake.map_id,"difficulty_id":intake.difficulty_id,"seed":intake.seed,
        "recorded_level_scaling":{"hits":intake.hits.len(),"unit_hits":intake.hits.len()-non_unit,"non_unit_hits":non_unit},
        "level":intake.level,"recorded_combat_seconds":super::healer_replay::recorded_combat_seconds(intake),"intake_blake3":blake3::hash(bytes).to_hex().to_string(),
        "intake_hits":intake.hits.len(),"intake_duration_ticks":intake.end_tick,
        "source_status":intake.source_status,"specs":rows});
    write_per_spec_markdown(&report, directory)?;
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
