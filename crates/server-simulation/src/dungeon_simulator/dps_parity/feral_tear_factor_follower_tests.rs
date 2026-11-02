//! A full follower fight must retain factor traces when Tear ticks.
#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_factor_capture_survives_tear_ticks() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "dungeon_simulator::dps_parity::follower_dps_diagnostic::follower_dps_production_dummy",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("FOLLOWER_DPS_SPEC", "103")
        .env("FOLLOWER_DPS_LEVEL", "90")
        .env("FOLLOWER_DPS_MAP", "2813")
        .env("FOLLOWER_DPS_SEED", "70448")
        .env("FOLLOWER_FACTOR_SPELL", "22568")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let factors: serde_json::Value = stdout
        .lines()
        .find_map(|line| {
            line.split_once("FOLLOWER_FACTORS ")
                .map(|(_, value)| serde_json::from_str(value).unwrap())
        })
        .expect("full factor capture");
    let events = factors["events"]
        .as_array()
        .expect("captured damage events");
    assert!(
        events.iter().any(|event| event["spell_id"] == 391356),
        "Tear damage must be observed"
    );
    assert!(
        events.iter().any(|event| event["spell_id"] == 22568),
        "Bite damage must be observed"
    );
}
