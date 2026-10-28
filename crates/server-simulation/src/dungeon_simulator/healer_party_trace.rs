//! SimC raid-event source rolls, mapped to five native party slots.
use std::path::Path;

#[derive(Debug, PartialEq)]
pub(super) struct PartyHit {
    pub time_ms: u64,
    pub slot: usize,
    pub amount: f32,
}

pub(super) fn read(path: &Path) -> Result<Vec<PartyHit>, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    parse(&text)
}
fn parse(text: &str) -> Result<Vec<PartyHit>, String> {
    let mut hits = Vec::new();
    let mut pending = None;
    for line in text.lines() {
        let Some((time, rest)) = line.split_once(' ') else {
            continue;
        };
        if let Some(fields) = rest.strip_prefix(
            "Enemy 'Tank_Dummy_Dungeon' direct amount for Action 'raid_damage_shadow' (0): ",
        ) {
            if pending.is_some() {
                return Err("missing SimC party target after source roll".into());
            }
            let time: f64 = time.parse().map_err(|_| "invalid SimC party time")?;
            let amount: f32 = fields
                .strip_prefix("amount=")
                .and_then(|s| s.split_whitespace().next())
                .ok_or("missing SimC party amount")?
                .parse()
                .map_err(|_| "invalid SimC party amount")?;
            if !time.is_finite() || time < 0.0 || !amount.is_finite() || amount < 0.0 {
                return Err("invalid SimC party hit".into());
            }
            pending = Some(((time * 1000.0).round() as u64, amount));
        } else if let Some(fields) = rest.strip_prefix("Tank_Dummy_Dungeon raid_damage_shadow ") {
            if let Some((ms, amount)) = pending.take() {
                let name = fields.split_once(':').ok_or("missing SimC party target")?.0;
                let slot = match name {
                    "MID2_Paladin_Protection" => 0,
                    "party_healer" => 1,
                    "party_damage_1" => 2,
                    "party_damage_2" => 3,
                    "party_damage_3" => 4,
                    _ => return Err(format!("unknown SimC party target {name}")),
                };
                let snapshot_time: f64 =
                    time.parse().map_err(|_| "invalid SimC party target time")?;
                if !snapshot_time.is_finite() || (snapshot_time * 1000.0).round() as u64 != ms {
                    return Err("SimC party roll/target time mismatch".into());
                }
                hits.push(PartyHit {
                    time_ms: ms,
                    slot,
                    amount,
                });
            }
        }
    }
    if pending.is_some() || hits.is_empty() || hits.windows(2).any(|p| p[0].time_ms > p[1].time_ms)
    {
        return Err("SimC party trace missing ordered damage events".into());
    }
    Ok(hits)
}
