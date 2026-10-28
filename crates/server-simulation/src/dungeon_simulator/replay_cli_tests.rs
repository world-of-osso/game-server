use super::healer_intake::Intake;
use std::path::Path;

#[test]
#[ignore = "native replay CLI entry point"]
fn native_replay_cli_driver() {
    let canonical = Path::new("/home/osso/Projects/world-of-osso/game-server/data/world.db")
        .canonicalize()
        .unwrap();
    assert_eq!(
        crate::test_data::world_db_path().canonicalize().unwrap(),
        canonical
    );
    let path = std::env::var("HEALER_TANK_TIMELINE").expect("HEALER_TANK_TIMELINE is required");
    let spec = std::env::var("HEALER_TANK_SPEC").expect("HEALER_TANK_SPEC is required");
    let source = std::env::var("HEALER_TANK_SOURCE").expect("HEALER_TANK_SOURCE is required");
    let report = match (source.as_str(), spec.as_str()) {
        ("simc", "66") => {
            let duration = std::env::var("HEALER_TANK_DURATION")
                .unwrap()
                .parse()
                .unwrap();
            let seed = std::env::var("HEALER_TANK_SEED").unwrap().parse().unwrap();
            super::tank_replay::replay_simc(Path::new(&path), duration, seed)
        }
        ("recorded", "65" | "66") => {
            let intake: Intake = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            if spec == "65" {
                super::healer_replay::replay(&intake, 65)
            } else {
                super::tank_replay::replay(&intake)
            }
        }
        _ => panic!("unsupported replay source/spec: {source}/{spec}"),
    }
    .expect("native replay failed");
    println!("HEALER_TANK_REPORT={report}");
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
#[ignore = "runs the MID2 SimC dummy trace through the CLI"]
fn cli_prints_mid2_simc_tank_pressure() {
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
        text.contains("SimC tank dummy, MID2 level 90; rotation: not_implemented."),
        "{text}"
    );
    let row = text
        .lines()
        .find(|line| line.starts_with("| 66 |"))
        .expect("tank report row");
    let cells: Vec<_> = row.split('|').map(str::trim).collect();
    assert!(cells[2].parse::<f64>().unwrap() > 100_000.0, "{row}");
}
