//! Explicit private catalog selection is a process/environment boundary, not a global env mutation.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn temporary() -> tempfile::TempDir {
    std::fs::create_dir_all("/var/tmp/osso").unwrap();
    tempfile::tempdir_in("/var/tmp/osso").unwrap()
}

fn run_report(directory: &Path, policy: &Path) -> Output {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "dungeon_simulator::healer_report::healer_benchmark_report_recorded_stockade",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .current_dir(&root)
        .env("FOLLOWER_KIT_DB", policy)
        .env("DUNGEON_SIMULATOR_MAP", "34")
        .env("DUNGEON_SIMULATOR_LEVEL", "32")
        .env(
            "HEALER_DUNGEON_INTAKE",
            root.join("content/dungeons/healer-benchmark/stockade-seed1.intake.json"),
        )
        .env("HEALER_DUNGEON_REPORT_DIR", directory)
        .output()
        .expect("native report subprocess")
}

fn read_report(directory: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(directory.join("report.json")).unwrap()).unwrap()
}

fn copy_native_catalog_without_garrick(path: &Path) {
    use rusqlite::{Connection, OpenFlags};
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap();
    let uri = format!(
        "file:{}?mode=ro",
        crate::test_data::world_db_path().display()
    );
    db.execute("ATTACH DATABASE ?1 AS native", [uri]).unwrap();
    for table in [
        "follower_profiles",
        "follower_spec_kit",
        "follower_lfg_dungeons",
        "tdb_world_safe_locs",
        "content_creature",
    ] {
        db.execute_batch(&format!(
            "CREATE TABLE {table} AS SELECT * FROM native.{table};"
        ))
        .unwrap();
    }
    db.execute("DELETE FROM follower_profiles WHERE entry=209057", [])
        .unwrap();
    db.execute_batch("DETACH DATABASE native;").unwrap();
}

#[test]
#[ignore = "RED: explicit private policy input is ignored by the benchmark runtime"]
fn healer_benchmark_private_policy_missing_explicit_path_never_uses_default_catalog() {
    let temp = temporary();
    let policy = temp.path().join("missing-policy.sqlite");
    let directory = temp.path().join("report");
    let output = run_report(&directory, &policy);
    assert!(
        !output.status.success(),
        "missing explicit policy must not produce canonical native metrics: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = read_report(&directory);
    for row in report["specs"].as_array().unwrap() {
        assert!(
            row["metrics"].is_null(),
            "explicit unreadable input cannot fall back: {row}"
        );
        assert!(
            row["gaps"].as_array().unwrap().iter().any(|gap| gap
                .as_str()
                .unwrap()
                .starts_with("PRIVATE_FOLLOWER_CATALOG")),
            "explicit source error required: {row}"
        );
    }
    assert!(
        !policy.exists(),
        "read-only selection must not create an empty policy database"
    );
}

#[test]
#[ignore = "RED: private catalog actor absence is replaced by canonical source data"]
fn healer_benchmark_private_policy_missing_source_actor_never_borrows_canonical_roster() {
    let temp = temporary();
    let policy = temp.path().join("policy.sqlite");
    copy_native_catalog_without_garrick(&policy);
    let directory = temp.path().join("report");
    let output = run_report(&directory, &policy);
    let report = read_report(&directory);
    let paladin = report["specs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["spec"] == 65)
        .unwrap();
    assert!(paladin["metrics"].is_null());
    assert_eq!(
        paladin["gaps"],
        serde_json::json!(["NATIVE_HEALER_SOURCE_PROFILE_ABSENT:209057"]),
        "must observe the selected private catalog, not canonical data: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let native = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let count: u32 = native
        .query_row(
            "SELECT count(*) FROM follower_profiles WHERE entry=209057",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 1,
        "private fixture must not alter the canonical roster"
    );
}
