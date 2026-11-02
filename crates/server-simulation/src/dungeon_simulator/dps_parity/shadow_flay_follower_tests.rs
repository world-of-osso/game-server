use serde_json::Value;
use std::process::Command;

#[test]
#[ignore = "requires the follower DPS snapshot"]
fn shadow_follower_recasts_mind_flay_after_two_ticks() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "dungeon_simulator::dps_parity::follower_dps_diagnostic::follower_dps_production_dummy",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env("FOLLOWER_DPS_SPEC", "258")
        .env("FOLLOWER_DPS_LEVEL", "70")
        .env("FOLLOWER_DPS_MAP", "2515")
        .env("FOLLOWER_DPS_SEED", "1")
        .env("FOLLOWER_TRACE_EVENTS", "1")
        .output()
        .expect("run production follower diagnostic");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = format!(
        "{}\n{}",
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap()
    );
    assert!(text.contains("running 1 test") && text.contains("test result: ok."));
    let receipt: Value = serde_json::from_str(
        text.split("FOLLOWER_DPS ")
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    let actor = receipt["actor_id"].as_u64().unwrap();
    let events: Vec<Value> = text
        .lines()
        .filter_map(|line| line.split_once("FOLLOWER_REPLAY_EVENT "))
        .map(|(_, json)| serde_json::from_str(json).unwrap())
        .filter(|event: &Value| event["event"]["source"].as_u64() == Some(actor))
        .collect();
    let casts: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| event["event"]["kind"] == "CastSuccess")
        .collect();
    let clipped = casts.windows(2).any(|pair| {
        let ((start_index, start), (end_index, end)) = (pair[0], pair[1]);
        if start["event"]["spell_id"] != 15407 || end["event"]["spell_id"] != 15407 {
            return false;
        }
        let elapsed = end["tick"].as_u64().unwrap() - start["tick"].as_u64().unwrap();
        let ticks = events[start_index + 1..end_index]
            .iter()
            .filter(|event| {
                event["event"]["spell_id"] == 15407 && event["event"]["kind"] == "Damage"
            })
            .count();
        elapsed < 40 && ticks == 2
    });
    assert!(
        clipped,
        "expected a Mind Flay recast after two damage ticks"
    );
}
