//! Recorded incoming damage shared by healer and tank replay.
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Timeline {
    hits: Vec<Hit>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Hit {
    time_ms: u64,
    school: u32,
    amount: f64,
    target_role: Role,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Role {
    Tank,
    Healer,
    Damage,
}

fn load(path: &Path) -> Result<Timeline, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let timeline: Timeline = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    if timeline
        .hits
        .windows(2)
        .any(|pair| pair[0].time_ms > pair[1].time_ms)
        || timeline.hits.iter().any(|hit| {
            hit.school == 0
                || hit.school & !0x7f != 0
                || !hit.amount.is_finite()
                || hit.amount < 0.0
        })
    {
        return Err(format!("invalid incoming hit timeline: {}", path.display()));
    }
    Ok(timeline)
}

#[test]
fn loads_concrete_incoming_timeline_and_rejects_out_of_order_hits() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/dungeon_simulator/fixtures/healer_tank_timeline.json");
    let timeline = load(&fixture).unwrap();
    assert_eq!(
        timeline.hits,
        vec![
            Hit {
                time_ms: 250,
                school: 1,
                amount: 1250.0,
                target_role: Role::Tank
            },
            Hit {
                time_ms: 1300,
                school: 4,
                amount: 350.5,
                target_role: Role::Healer
            },
            Hit {
                time_ms: 1300,
                school: 32,
                amount: 700.0,
                target_role: Role::Damage
            },
        ]
    );
    let dir = tempfile::tempdir().unwrap();
    let invalid = dir.path().join("invalid.json");
    std::fs::write(&invalid, r#"{"hits":[{"time_ms":2,"school":1,"amount":1,"target_role":"tank"},{"time_ms":1,"school":1,"amount":1,"target_role":"tank"}]}"#).unwrap();
    assert!(load(&invalid).is_err());
}
