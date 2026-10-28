//! Pre-outcome source rolls from the pinned SimC tank dummy debug trace.
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    Swing,
    Strike,
    Bleed,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Hit {
    pub(crate) time_ms: u64,
    pub(crate) amount: f64,
    pub(crate) kind: Kind,
}

pub(crate) fn read_trace(path: &Path, dummy: &str) -> Result<Vec<Hit>, String> {
    let trace = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    parse_trace(&trace, dummy)
}

pub(crate) fn read_bloodlust_time(path: &Path, player: &str) -> Result<Option<u64>, String> {
    let trace = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    parse_bloodlust_time(&trace, player)
}

fn parse_bloodlust_time(trace: &str, player: &str) -> Result<Option<u64>, String> {
    let event = format!("Player '{player}' gains Buff 'bloodlust' (2825)");
    let mut time = None;
    for line in trace.lines() {
        let Some((seconds, rest)) = line.split_once(' ') else {
            continue;
        };
        if !rest.starts_with(&event) {
            continue;
        }
        let seconds: f64 = seconds
            .parse()
            .map_err(|error| format!("invalid Bloodlust time: {error}"))?;
        if !seconds.is_finite() || seconds < 0.0 || time.is_some() {
            return Err("invalid or repeated SimC Bloodlust event".into());
        }
        time = Some((seconds * 1000.0).round() as u64);
    }
    Ok(time)
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
        let (direct, fields) = if let Some(fields) = rest.strip_prefix("direct amount for Action '")
        {
            (true, fields)
        } else if let Some(fields) = rest.strip_prefix("tick amount for Action '") {
            (false, fields)
        } else {
            continue;
        };
        let Some((action, fields)) = fields.split_once("' (0)") else {
            continue;
        };
        let kind = if direct && action.starts_with("melee_main_hand_") {
            Kind::Swing
        } else if direct && action.starts_with("melee_nuke_") {
            Kind::Strike
        } else if !direct && action.starts_with("spell_dot_") {
            Kind::Bleed
        } else {
            continue;
        };
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
fn reads_bloodlust_only_for_the_named_player() {
    let trace = "0.000 Player 'tank' gains Buff 'bloodlust' (2825) (stacks=1)\n3.000 Player 'other' gains Buff 'bloodlust' (2825)";
    assert_eq!(parse_bloodlust_time(trace, "tank").unwrap(), Some(0));
    assert_eq!(parse_bloodlust_time(trace, "other").unwrap(), Some(3000));
    assert_eq!(parse_bloodlust_time(trace, "missing").unwrap(), None);
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
                kind: Kind::Swing
            },
            Hit {
                time_ms: 4000,
                amount: 505757.0,
                kind: Kind::Strike
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
