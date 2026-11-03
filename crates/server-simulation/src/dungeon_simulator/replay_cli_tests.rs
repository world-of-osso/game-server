use super::healer_intake::Intake;
use std::path::Path;

#[test]
#[ignore = "native replay CLI entry point; healer mode replays a class party over FOLLOWER_DPS_WORLD_DB"]
fn native_replay_cli_driver() {
    let path = std::env::var("HEALER_TANK_TIMELINE").expect("HEALER_TANK_TIMELINE is required");
    let spec = std::env::var("HEALER_TANK_SPEC").expect("HEALER_TANK_SPEC is required");
    let source = std::env::var("HEALER_TANK_SOURCE").expect("HEALER_TANK_SOURCE is required");
    let mode = std::env::var("HEALER_TANK_MODE").unwrap_or_else(|_| "tank".into());
    // Recorded intakes were captured against canonical; the SimC healer
    // replay refuses it and reads FOLLOWER_DPS_WORLD_DB instead.
    if source == "recorded" {
        let canonical = Path::new("/home/osso/Projects/world-of-osso/game-server/data/world.db")
            .canonicalize()
            .unwrap();
        assert_eq!(
            crate::test_data::world_db_path().canonicalize().unwrap(),
            canonical
        );
    }
    let report = match (source.as_str(), mode.as_str(), spec.as_str()) {
        ("simc", "healer", _) => {
            let level = std::env::var("HEALER_TANK_LEVEL")
                .expect("HEALER_TANK_LEVEL: the class party's level, 70 or 90")
                .parse()
                .expect("numeric party level");
            let duration = std::env::var("HEALER_TANK_DURATION")
                .unwrap()
                .parse()
                .unwrap();
            let seed = std::env::var("HEALER_TANK_SEED").unwrap().parse().unwrap();
            if let Some(party_trace) = std::env::var_os("HEALER_TANK_PARTY_TRACE") {
                if let Some(profile) = std::env::var_os("HEALER_ENEMY_PROFILE") {
                    super::healer_simc_replay::replay_simc_party_with_profile(
                        spec.parse().unwrap(),
                        level,
                        Path::new(&path),
                        Path::new(&party_trace),
                        duration,
                        seed,
                        Path::new(&profile),
                    )
                } else {
                    let targets = std::env::var("HEALER_TANK_TARGETS")
                        .unwrap_or_else(|_| "1".into())
                        .parse()
                        .expect("numeric HEALER_TANK_TARGETS");
                    super::healer_simc_replay::replay_simc_party_with_targets(
                        spec.parse().unwrap(),
                        level,
                        Path::new(&path),
                        Path::new(&party_trace),
                        duration,
                        seed,
                        targets,
                    )
                }
            } else {
                super::healer_simc_replay::replay_simc(
                    spec.parse().unwrap(),
                    level,
                    Path::new(&path),
                    duration,
                    seed,
                )
            }
        }
        ("simc", "tank", "66" | "73" | "104" | "250" | "268" | "581") => {
            let duration = std::env::var("HEALER_TANK_DURATION")
                .unwrap()
                .parse()
                .unwrap();
            let seed = std::env::var("HEALER_TANK_SEED").unwrap().parse().unwrap();
            let healing = std::env::var("HEALER_TANK_EXTERNAL_HEALING").unwrap_or_default() == "1";
            let pressure_scale = std::env::var("HEALER_TANK_PRESSURE_SCALE")
                .unwrap_or_else(|_| "1".into())
                .parse()
                .expect("numeric pressure scale");
            super::tank_replay::replay_simc_with_pressure(
                spec.parse().unwrap(),
                Path::new(&path),
                duration,
                seed,
                healing,
                pressure_scale,
            )
        }
        ("recorded", "tank", "65" | "66") => {
            let intake: Intake = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            if spec == "65" {
                super::healer_replay::replay(&intake, 65)
            } else {
                super::tank_replay::replay(&intake)
            }
        }
        _ => panic!("unsupported replay source/mode/spec: {source}/{mode}/{spec}"),
    }
    .expect("native replay failed");
    println!("HEALER_TANK_REPORT={report}");
}

#[test]
#[ignore = "requires the class-party snapshot and SimC tank and party traces"]
fn party_pulse_seed_replays_identical_events_and_outcome() {
    fn replay() -> (serde_json::Value, Vec<serde_json::Value>) {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "dungeon_simulator::replay_cli_tests::native_replay_cli_driver",
                "--exact",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("HEALER_TANK_SOURCE", "simc")
            .env("HEALER_TANK_MODE", "healer")
            .env("HEALER_TANK_LEVEL", "90")
            .env("HEALER_TANK_DURATION", "120")
            .env("HEALER_TANK_SPEC", "105")
            .env("HEALER_TANK_SEED", "8")
            .env("HEALER_TANK_TRACE_EVENTS", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let report = stdout
            .lines()
            .find_map(|line| {
                line.split_once("HEALER_TANK_REPORT=")
                    .map(|(_, report)| serde_json::from_str(report).unwrap())
            })
            .expect("replay report");
        let stderr = String::from_utf8(output.stderr).unwrap();
        let events: Vec<serde_json::Value> = stderr
            .lines()
            .filter_map(|line| line.strip_prefix("HEALER_REPLAY_EVENT "))
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(!events.is_empty(), "replay emitted no events");
        (report, events)
    }
    let first = replay();
    let second = replay();
    for (index, (left, right)) in first.1.iter().zip(&second.1).enumerate() {
        assert_eq!(left, right, "combat event {index} diverged");
    }
    assert_eq!(
        first.1.len(),
        second.1.len(),
        "combat event counts diverged"
    );
    assert_eq!(first.0, second.0, "replay outcomes diverged");
}

/// Train-executed batches use the same replay entry point as single-seed runs.
#[test]
#[ignore = "native tank replay batch entry point"]
fn native_tank_replay_batch() {
    let path = std::env::var("HEALER_TANK_TIMELINE").expect("HEALER_TANK_TIMELINE is required");
    let spec = std::env::var("HEALER_TANK_SPEC").unwrap().parse().unwrap();
    let duration = std::env::var("HEALER_TANK_DURATION")
        .unwrap()
        .parse()
        .unwrap();
    let healing = std::env::var("HEALER_TANK_EXTERNAL_HEALING").unwrap_or_default() == "1";
    for seed in std::env::var("HEALER_TANK_SEEDS").unwrap().split(',') {
        let seed: u64 = seed.parse().expect("numeric replay seed");
        let report = super::tank_replay::replay_simc_with_healing(
            spec,
            Path::new(&path),
            duration,
            seed,
            healing,
        )
        .expect("native replay failed");
        println!("HEALER_TANK_BATCH_SEED={seed} REPORT={report}");
    }
}

#[test]
fn all_replay_cli_summarizes_parallel_results() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = std::process::Command::new("python3")
        .arg(root.join("scripts/tests/test_healer_tank_all.py"))
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
#[ignore = "runs the native tank replay through the CLI"]
fn cli_prints_a_real_protection_tank_report() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("scripts/healer_tank_replay.py"))
        .arg("--timeline")
        .arg(root.join("content/dungeons/healer-benchmark/stockade-seed1.intake.json"))
        .arg("--source")
        .arg("recorded")
        .arg("--spec")
        .arg("66")
        .arg("--executable")
        .arg(std::env::current_exe().unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("| Spec | Raw damage | Damage taken | Mitigation % |"),
        "{text}"
    );
    let row = text
        .lines()
        .find(|line| line.starts_with("| 66 |"))
        .expect("tank report row");
    let cells: Vec<_> = row.split('|').map(str::trim).collect();
    assert!(cells[2].parse::<f64>().unwrap() > 0.0, "{row}");
    assert!(cells[3].parse::<f64>().unwrap() > 0.0, "{row}");
}

#[test]
#[ignore = "runs MID2 Vengeance resource and target-debuff reporting through the CLI"]
fn cli_prints_vengeance_resource_counters_and_target_debuffs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("scripts/healer_tank_replay.py"))
        .arg("--timeline")
        .arg(std::env::var("HEALER_TANK_SIMC_TRACE").unwrap())
        .arg("--profile-dir")
        .arg(std::env::var("DPS_PARITY_OUTPUT").unwrap())
        .arg("--spec")
        .arg("581")
        .arg("--executable")
        .arg(std::env::current_exe().unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("| soul_fragments_generated |"), "{text}");
    assert!(text.contains("| soul_fragments_consumed |"), "{text}");
    assert!(text.contains("| Target debuff ID | Uptime % |"), "{text}");
    assert!(
        text.contains("| 207771 |") && text.contains("| 204598 |"),
        "{text}"
    );
}

#[test]
#[ignore = "runs two real MID2 Protection dummy replays through the CLI"]
fn cli_aggregates_two_real_tank_seeds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let run = |extra: &[&str]| {
        let output = std::process::Command::new("python3")
            .arg(root.join("scripts/healer_tank_replay.py"))
            .args(["--spec", "66", "--simc-tank-heal", "--timeline"])
            .arg(std::env::var("HEALER_TANK_SIMC_TRACE").unwrap())
            .arg("--profile-dir")
            .arg(std::env::var("DPS_PARITY_OUTPUT").unwrap())
            .arg("--executable")
            .arg(std::env::current_exe().unwrap())
            .args(extra)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    let cast_count = |text: &str| -> f64 {
        text.lines()
            .find(|line| line.starts_with("| 53600 | "))
            .expect("Shield of the Righteous row")
            .split('|')
            .nth(2)
            .unwrap()
            .trim()
            .parse()
            .unwrap()
    };
    let first = cast_count(&run(&["--seed", "70448"]));
    let second = cast_count(&run(&["--seed", "70449"]));
    let aggregate = run(&["--seeds", "2"]);
    let expected = format!(
        "| 53600 | {:.2} ({:.2}–{:.2}) |",
        (first + second) / 2.0,
        first.min(second),
        first.max(second)
    );
    assert!(
        aggregate.contains(&expected),
        "expected {expected}: {aggregate}"
    );
    assert!(
        aggregate.contains("2 seeds 70448–70449") && aggregate.contains("/2 |"),
        "{aggregate}"
    );
}

#[test]
#[ignore = "runs the MID2 Protection dummy replay and its native Energize events"]
fn cli_prints_tank_energize_sources() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("scripts/healer_tank_replay.py"))
        .args([
            "--spec",
            "66",
            "--duration",
            "300",
            "--simc-tank-heal",
            "--timeline",
        ])
        .arg(std::env::var("HEALER_TANK_SIMC_TRACE").unwrap())
        .arg("--profile-dir")
        .arg(std::env::var("DPS_PARITY_OUTPUT").unwrap())
        .arg("--executable")
        .arg(std::env::current_exe().unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    let section = text
        .split("| Energize spell ID | Attempted | Effective |")
        .nth(1)
        .expect("Energize report");
    let row = section
        .lines()
        .find(|line| line.starts_with("| 204019 | "))
        .expect("Blessed Hammer energize");
    let cells: Vec<_> = row.split('|').map(str::trim).collect();
    let attempted = cells[2].parse::<u32>().unwrap();
    let effective = cells[3].parse::<u32>().unwrap();
    assert!(
        attempted > 0 && effective > 0 && effective <= attempted,
        "{row}"
    );
}

#[test]
#[ignore = "runs a class-party healer against the SimC dummy trace over FOLLOWER_DPS_WORLD_DB"]
fn cli_prints_class_healer_report() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("scripts/healer_tank_replay.py"))
        .args([
            "--mode",
            "healer",
            "--spec",
            "264",
            "--duration",
            "30",
            "--level",
            "90",
            "--world-db",
        ])
        .arg(std::env::var("FOLLOWER_DPS_WORLD_DB").unwrap())
        .arg("--timeline")
        .arg(std::env::var("HEALER_TANK_SIMC_TRACE").unwrap())
        .arg("--profile-dir")
        .arg(std::env::var("DPS_PARITY_OUTPUT").unwrap())
        .arg("--executable")
        .arg(std::env::current_exe().unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    let row = text
        .lines()
        .find(|line| line.starts_with("| 264 |"))
        .expect("healer report row");
    let cells: Vec<_> = row.split('|').map(str::trim).collect();
    assert!(cells[2].parse::<f64>().unwrap() > 0.0, "{row}");
    assert!(text.contains("| Spell ID | Effective HPS | Raw HPS |"));
    assert!(text.contains("| Time (s) | Mana | Max mana |"));
}

#[test]
#[ignore = "runs the healed MID2 SimC dummy trace through the CLI"]
fn cli_selects_simc_dummy_healing() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let output = std::process::Command::new("python3")
        .arg(root.join("scripts/healer_tank_replay.py"))
        .arg("--timeline")
        .arg(trace)
        .arg("--simc-tank-heal")
        .arg("--executable")
        .arg(std::env::current_exe().unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("External dummy healing: enabled"), "{text}");
    assert!(text.contains("SimC Bloodlust start (s): 0.00"), "{text}");
    assert!(text.contains("| 432607 |"), "{text}");
    let row = text
        .lines()
        .find(|line| line.starts_with("| 66 |"))
        .expect("tank report row");
    assert_eq!(
        row.split('|').map(str::trim).collect::<Vec<_>>()[7],
        "N/A",
        "{row}"
    );
}

#[test]
#[ignore = "runs the MID2 SimC dummy trace through the CLI"]
fn cli_prints_registered_simc_protection_report() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let output = std::process::Command::new("python3")
        .arg(root.join("scripts/healer_tank_replay.py"))
        .arg("--timeline")
        .arg(trace)
        .arg("--executable")
        .arg(std::env::current_exe().unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("SimC tank dummy, MID2 level 90; rotation: registered_literal_simc."),
        "{text}"
    );
    let row = text
        .lines()
        .find(|line| line.starts_with("| 66 |"))
        .expect("tank report row");
    let cells: Vec<_> = row.split('|').map(str::trim).collect();
    assert!(cells[2].parse::<f64>().unwrap() > 100_000.0, "{row}");
    assert!(cells[8].parse::<f64>().unwrap() > 0.0, "{row}");
}
