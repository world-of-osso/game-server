//! Pre-recipient amounts from the pinned SimC tank dummy debug trace.
use std::path::Path;

#[derive(Debug, PartialEq)]
enum Kind {
    Melee,
    Bleed,
}

#[derive(Debug, PartialEq)]
struct Hit {
    time_ms: u64,
    amount: f64,
    kind: Kind,
}

fn read_trace(path: &Path, dummy: &str) -> Result<Vec<Hit>, String> {
    let trace = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    parse_trace(&trace, dummy)
}

fn parse_trace(trace: &str, dummy: &str) -> Result<Vec<Hit>, String> {
    let source = format!("Enemy '{dummy}' ");
    let mut hits = Vec::new();
    for line in trace.lines() {
        let Some((time, rest)) = line.split_once(' ') else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(&source) else {
            continue;
        };
        let (kind, fields) = if let Some(fields) = rest.strip_prefix("direct amount for Action '") {
            (Kind::Melee, fields)
        } else if let Some(fields) = rest.strip_prefix("tick amount for Action '") {
            (Kind::Bleed, fields)
        } else {
            continue;
        };
        let Some((action, fields)) = fields.split_once("' (0)") else {
            continue;
        };
        let is_dummy_damage = match kind {
            Kind::Melee => {
                action.starts_with("melee_main_hand_") || action.starts_with("melee_nuke_")
            }
            Kind::Bleed => action.starts_with("spell_dot_"),
        };
        if !is_dummy_damage {
            continue;
        }
        let seconds: f64 = time
            .parse()
            .map_err(|error| format!("invalid SimC time {time}: {error}"))?;
        let amount_text = fields
            .split_once("amount=")
            .ok_or_else(|| format!("missing SimC amount: {line}"))?
            .1
            .split_whitespace()
            .next()
            .ok_or_else(|| format!("missing SimC amount: {line}"))?;
        let amount: f64 = amount_text
            .parse()
            .map_err(|error| format!("invalid SimC amount {amount_text}: {error}"))?;
        if !seconds.is_finite() || seconds < 0.0 || !amount.is_finite() || amount < 0.0 {
            return Err(format!("invalid SimC hit: {line}"));
        }
        hits.push(Hit {
            time_ms: (seconds * 1000.0).round() as u64,
            amount,
            kind,
        });
    }
    if hits.is_empty()
        || hits
            .windows(2)
            .any(|pair| pair[0].time_ms > pair[1].time_ms)
    {
        return Err("SimC tank dummy trace has no ordered damage events".into());
    }
    Ok(hits)
}

#[test]
fn reads_simc_dummy_raw_melee_and_bleed_without_tank_heals() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/dungeon_simulator/fixtures/simc_tank_dummy.log");
    let hits = read_trace(&path, "Tank_Dummy_Dungeon").unwrap();
    assert_eq!(
        hits,
        vec![
            Hit {
                time_ms: 4000,
                amount: 245737.0,
                kind: Kind::Melee
            },
            Hit {
                time_ms: 4000,
                amount: 505757.0,
                kind: Kind::Melee
            },
            Hit {
                time_ms: 7004,
                amount: 25000.0,
                kind: Kind::Bleed
            },
        ]
    );
    assert!(parse_trace("not a damage trace", "Tank_Dummy_Dungeon").is_err());
}
