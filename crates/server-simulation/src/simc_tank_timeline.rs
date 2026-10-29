//! Pre-outcome source rolls from the pinned SimC tank dummy debug trace.
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    Swing,
    Strike,
    BleedApplication,
    Bleed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MeleeTable {
    pub(crate) miss: f32,
    pub(crate) crit: f32,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Hit {
    pub(crate) time_ms: u64,
    pub(crate) amount: f64,
    pub(crate) kind: Kind,
    pub(crate) melee_table: Option<MeleeTable>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Scenario {
    pub(crate) dummy: String,
    pub(crate) player: String,
    pub(crate) target_level: Option<u8>,
    pub(crate) armor_constant: f32,
    pub(crate) heals: Vec<(u64, f32)>,
}

pub(crate) fn read_scenario(path: &Path) -> Result<Scenario, String> {
    let trace = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    parse_scenario(&trace)
}

fn parse_scenario(trace: &str) -> Result<Scenario, String> {
    let dummy = trace
        .lines()
        .find_map(|line| {
            let (_, rest) = line.split_once(" Enemy '")?;
            let (name, rest) = rest.split_once("' actions.default=")?;
            rest.starts_with("/auto_attack").then_some(name.to_owned())
        })
        .ok_or("SimC tank dummy actions absent")?;
    let target_level = trace
        .lines()
        .find_map(|line| line.strip_prefix("target_level="))
        .ok_or("SimC target level absent")?
        .parse::<i16>()
        .map_err(|error| format!("invalid SimC target level: {error}"))?;
    let target_level = if target_level == -1 {
        None
    } else {
        Some(
            u8::try_from(target_level)
                .map_err(|error| format!("invalid SimC target level: {error}"))?,
        )
    };
    let armor_marker = format!("'{dummy}' base armor coefficient set to ");
    let armor_constant = trace
        .lines()
        .filter_map(|line| line.split_once(&armor_marker).map(|(_, value)| value))
        .last()
        .ok_or("SimC dummy armor constant absent")?
        .trim_end_matches('.')
        .parse::<f32>()
        .map_err(|error| format!("invalid SimC armor constant: {error}"))?;
    if !armor_constant.is_finite() || armor_constant <= 0.0 {
        return Err("invalid SimC dummy armor constant".into());
    }
    let heal_marker = format!("Enemy '{dummy}' Action 'tank_heal' (0) heals Player '");
    let mut heals = Vec::new();
    let mut player = None;
    for line in trace.lines() {
        let Some((time, event)) = line.split_once(' ') else {
            continue;
        };
        let Some(rest) = event.strip_prefix(&heal_marker) else {
            continue;
        };
        let (name, amount) = rest
            .split_once(" for ")
            .ok_or("SimC tank heal amount absent")?;
        let name = name
            .strip_suffix('\'')
            .ok_or("SimC tank heal target absent")?;
        if player.as_deref().is_some_and(|previous| previous != name) {
            return Err("SimC tank heals have multiple targets".into());
        }
        player = Some(name.to_owned());
        let seconds = time
            .parse::<f64>()
            .map_err(|error| format!("invalid SimC heal time: {error}"))?;
        let amount = amount
            .split_whitespace()
            .next()
            .ok_or("SimC tank heal amount absent")?
            .parse::<f32>()
            .map_err(|error| format!("invalid SimC tank heal: {error}"))?;
        if !seconds.is_finite() || seconds < 0.0 || !amount.is_finite() || amount <= 0.0 {
            return Err("invalid SimC tank heal".into());
        }
        heals.push(((seconds * 1000.0).round() as u64, amount));
    }
    if heals.is_empty() || heals.windows(2).any(|pair| pair[0].0 > pair[1].0) {
        return Err("SimC tank healing events absent or unordered".into());
    }
    Ok(Scenario {
        dummy,
        player: player.ok_or("SimC tank heal target absent")?,
        target_level,
        armor_constant,
        heals,
    })
}

pub(crate) fn read_trace(path: &Path, dummy: &str) -> Result<Vec<Hit>, String> {
    let trace = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    parse_trace(&trace, dummy)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RaidBuff {
    pub name: String,
    pub spell_id: u32,
}

pub(crate) fn read_raid_buffs(path: &Path, player: &str) -> Result<Vec<RaidBuff>, String> {
    let trace = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    parse_raid_buffs(&trace, player)
}

fn parse_raid_buffs(trace: &str, player: &str) -> Result<Vec<RaidBuff>, String> {
    const NAMES: &[&str] = &[
        "mark_of_the_wild",
        "arcane_intellect",
        "power_word_fortitude",
        "battle_shout",
        "skyfury",
        "blessing_of_the_bronze",
    ];
    let mut buffs = Vec::new();
    for &name in NAMES {
        let enabled = trace
            .lines()
            .any(|line| line.trim() == format!("override.{name}=1"));
        if !enabled {
            continue;
        }
        let prefix = if name == "blessing_of_the_bronze" {
            format!("Player '{player}' gains Buff '{name}' (")
        } else {
            format!("Raid gains Buff '{name}' (")
        };
        let ids: Vec<&str> = trace
            .lines()
            .filter_map(|line| line.split_once(' '))
            .filter(|(time, _)| *time == "0.000")
            .filter_map(|(_, event)| event.strip_prefix(&prefix))
            .filter_map(|rest| rest.split_once(')').map(|(id, _)| id))
            .collect();
        if ids.len() != 1 {
            return Err(format!(
                "expected one initial SimC raid buff event for {name}, found {}",
                ids.len()
            ));
        }
        let spell_id: u32 = ids[0]
            .parse()
            .map_err(|error| format!("invalid {name} spell ID: {error}"))?;
        if spell_id == 0 {
            return Err(format!("invalid zero spell ID for {name}"));
        }
        buffs.push(RaidBuff {
            name: name.into(),
            spell_id,
        });
    }
    Ok(buffs)
}

pub(crate) fn read_dummy_debuffs(path: &Path, dummy: &str) -> Result<Vec<RaidBuff>, String> {
    let trace = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    parse_dummy_debuffs(&trace, dummy)
}

fn parse_dummy_debuffs(trace: &str, dummy: &str) -> Result<Vec<RaidBuff>, String> {
    let mut debuffs = Vec::new();
    for (name, expected_id) in [
        ("chaos_brand", 1490),
        ("mystic_touch", 113746),
        ("hunters_mark", 259556),
        ("mortal_wounds", 115804),
    ] {
        if !trace
            .lines()
            .any(|line| line.trim() == format!("override.{name}=1"))
        {
            continue;
        }
        let event = format!("0.000 Enemy '{dummy}' gains Buff '{name}' ({expected_id})");
        let count = trace
            .lines()
            .filter(|line| line.starts_with(&event))
            .count();
        if count != 1 {
            return Err(format!(
                "expected one initial SimC dummy debuff event for {name}, found {count}"
            ));
        }
        debuffs.push(RaidBuff {
            name: name.into(),
            spell_id: expected_id,
        });
    }
    Ok(debuffs)
}

pub(crate) fn read_dummy_bleeding(path: &Path, dummy: &str) -> Result<bool, String> {
    let trace = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    parse_dummy_bleeding(&trace, dummy)
}

fn parse_dummy_bleeding(trace: &str, dummy: &str) -> Result<bool, String> {
    if !trace
        .lines()
        .any(|line| line.trim() == "override.bleeding=1")
    {
        return Ok(false);
    }
    let event = format!("0.000 Enemy '{dummy}' gains Buff 'bleeding' (0)");
    let count = trace
        .lines()
        .filter(|line| line.starts_with(&event))
        .count();
    if count != 1 {
        return Err(format!(
            "expected one initial SimC dummy bleeding event, found {count}"
        ));
    }
    Ok(true)
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
    let mut melee_table = None;
    let mut reading_melee = false;
    for line in trace.lines() {
        let Some((time, rest)) = line.split_once(' ') else {
            continue;
        };
        if let Some(action) = rest.strip_prefix(&format!("{source}Action 'spell_dot_"))
            && action.contains("(0) hits Player '")
            && action.ends_with("for 0.000000 physical damage (hit)")
        {
            let seconds: f64 = time
                .parse()
                .map_err(|error| format!("invalid SimC time {time}: {error}"))?;
            if !seconds.is_finite() || seconds < 0.0 {
                return Err(format!("invalid SimC hit: {line}"));
            }
            hits.push(Hit {
                time_ms: (seconds * 1000.0).round() as u64,
                amount: 0.0,
                kind: Kind::BleedApplication,
                melee_table: None,
            });
            continue;
        }
        if let Some(action) = rest.strip_prefix(&format!("{source}performs Action '")) {
            reading_melee =
                action.starts_with("melee_main_hand_") || action.starts_with("melee_nuke_");
            melee_table = None;
        }
        if reading_melee && let Some(fields) = rest.strip_prefix("attack_t::build_table: ") {
            let value = |name: &str| -> Result<f32, String> {
                let prefix = format!("{name}=");
                let raw = fields
                    .split_whitespace()
                    .find_map(|field| field.strip_prefix(&prefix))
                    .ok_or_else(|| format!("missing {name} in SimC attack table: {line}"))?;
                let value: f32 = raw
                    .parse()
                    .map_err(|error| format!("invalid {name}: {error}"))?;
                if !value.is_finite() || value > 1.0 || (name != "miss" && value < 0.0) {
                    return Err(format!("invalid {name} in SimC attack table: {line}"));
                }
                // SimC attack.cpp197–211 omits nonpositive miss entries.
                Ok(value.max(0.0))
            };
            melee_table = Some(MeleeTable {
                miss: value("miss")?,
                crit: value("crit")?,
            });
        }
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
        let table = if kind == Kind::Bleed {
            None
        } else {
            Some(
                melee_table
                    .take()
                    .ok_or_else(|| format!("missing SimC attack table: {line}"))?,
            )
        };
        reading_melee = false;
        hits.push(Hit {
            time_ms: (seconds * 1000.0).round() as u64,
            amount,
            kind,
            melee_table: table,
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
fn reads_actual_dummy_level_armor_and_healing() {
    let trace = "target_level=93\n0.000 Enemy 'Fluffy_Pillow' base armor coefficient set to 3430.\n0.000 Enemy 'Fluffy_Pillow' actions.default=/auto_attack,damage=1000000\n0.000 Enemy 'Fluffy_Pillow' Action 'tank_heal' (0) heals Player 'Tank'layn' for 300000 (0) (hit)\n5.000 Enemy 'Fluffy_Pillow' Action 'tank_heal' (0) heals Player 'Tank'layn' for 300000 (1) (hit)";
    assert_eq!(
        parse_scenario(trace).unwrap(),
        Scenario {
            dummy: "Fluffy_Pillow".into(),
            player: "Tank'layn".into(),
            target_level: Some(93),
            armor_constant: 3430.0,
            heals: vec![(0, 300000.0), (5000, 300000.0)],
        }
    );
}

#[test]
fn reads_enabled_raid_buffs_and_their_trace_spell_ids() {
    let trace = "override.skyfury=1\noverride.mark_of_the_wild=1\noverride.battle_shout=0\n0.000 Raid gains Buff 'skyfury' (462854) (stacks=1)\n0.000 Raid gains Buff 'mark_of_the_wild' (1126) (stacks=1)\n0.000 Raid gains Buff 'battle_shout' (6673) (stacks=1)";
    assert_eq!(
        parse_raid_buffs(trace, "tank").unwrap(),
        vec![
            RaidBuff {
                name: "mark_of_the_wild".into(),
                spell_id: 1126
            },
            RaidBuff {
                name: "skyfury".into(),
                spell_id: 462854
            },
        ]
    );
    assert!(parse_raid_buffs("override.skyfury=1", "tank").is_err());
}

#[test]
fn reads_the_named_players_class_specific_bronze_buff() {
    let trace = "override.blessing_of_the_bronze=1\n0.000 Player 'other' gains Buff 'blessing_of_the_bronze' (381752)\n0.000 Player 'tank' gains Buff 'blessing_of_the_bronze' (381741)";
    assert_eq!(
        parse_raid_buffs(trace, "tank").unwrap(),
        vec![RaidBuff {
            name: "blessing_of_the_bronze".into(),
            spell_id: 381741
        }]
    );
    assert!(parse_raid_buffs(trace, "missing").is_err());
    assert!(
        parse_raid_buffs("override.blessing_of_the_bronze=0", "tank")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn reads_only_enabled_dummy_debuffs() {
    let trace = "override.chaos_brand=1\noverride.mystic_touch=0\noverride.hunters_mark=1\n0.000 Enemy 'dummy' gains Buff 'chaos_brand' (1490) (stacks=1)\n0.000 Enemy 'dummy' gains Buff 'hunters_mark' (259556) (stacks=1)\n0.000 Enemy 'dummy' gains Buff 'mystic_touch' (113746) (stacks=1)";
    assert_eq!(
        parse_dummy_debuffs(trace, "dummy").unwrap(),
        vec![
            RaidBuff {
                name: "chaos_brand".into(),
                spell_id: 1490
            },
            RaidBuff {
                name: "hunters_mark".into(),
                spell_id: 259556
            }
        ]
    );
    assert!(parse_dummy_debuffs("override.chaos_brand=1", "dummy").is_err());
    assert!(parse_dummy_debuffs(trace, "other").is_err());
}

#[test]
fn reads_trace_declared_intellect_mortal_wounds_and_bleeding() {
    let trace = "override.arcane_intellect=1\noverride.mortal_wounds=1\noverride.bleeding=1\n0.000 Raid gains Buff 'arcane_intellect' (1459)\n0.000 Enemy 'dummy' gains Buff 'mortal_wounds' (115804)\n0.000 Enemy 'dummy' gains Buff 'bleeding' (0)";
    assert_eq!(
        parse_raid_buffs(trace, "tank").unwrap(),
        vec![RaidBuff {
            name: "arcane_intellect".into(),
            spell_id: 1459
        }]
    );
    assert_eq!(
        parse_dummy_debuffs(trace, "dummy").unwrap(),
        vec![RaidBuff {
            name: "mortal_wounds".into(),
            spell_id: 115804
        }]
    );
    assert!(parse_dummy_bleeding(trace, "dummy").unwrap());
    assert!(!parse_dummy_bleeding("override.bleeding=0", "dummy").unwrap());
    assert!(parse_dummy_bleeding(trace, "other").is_err());
}

#[test]
fn reads_bloodlust_only_for_the_named_player() {
    let trace = "0.000 Player 'tank' gains Buff 'bloodlust' (2825) (stacks=1)\n3.000 Player 'other' gains Buff 'bloodlust' (2825)";
    assert_eq!(parse_bloodlust_time(trace, "tank").unwrap(), Some(0));
    assert_eq!(parse_bloodlust_time(trace, "other").unwrap(), Some(3000));
    assert_eq!(parse_bloodlust_time(trace, "missing").unwrap(), None);
}

#[test]
fn reads_each_dummy_attacker_table_and_rejects_missing_tables() {
    let trace = "1.000 Enemy 'Tank_Dummy_Dungeon' performs Action 'melee_main_hand_tank' (0)\n1.000 attack_t::build_table: miss=0.1 dodge=0 parry=0 glance=0 crit=0.2\n1.000 Enemy 'Tank_Dummy_Dungeon' direct amount for Action 'melee_main_hand_tank' (0): amount=100\n2.000 Enemy 'Tank_Dummy_Dungeon' performs Action 'melee_nuke_tank' (0)\n2.000 attack_t::build_table: miss=0 dodge=0 parry=0 glance=0 crit=0.3\n2.000 Enemy 'Tank_Dummy_Dungeon' direct amount for Action 'melee_nuke_tank' (0): amount=200";
    let hits = parse_trace(trace, "Tank_Dummy_Dungeon").unwrap();
    assert_eq!(
        hits[0].melee_table,
        Some(MeleeTable {
            miss: 0.1,
            crit: 0.2
        })
    );
    assert_eq!(
        hits[1].melee_table,
        Some(MeleeTable {
            miss: 0.0,
            crit: 0.3
        })
    );
    assert!(parse_trace("1.000 Enemy 'Tank_Dummy_Dungeon' direct amount for Action 'melee_main_hand_tank' (0): amount=100", "Tank_Dummy_Dungeon").is_err());
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
                kind: Kind::Swing,
                melee_table: Some(MeleeTable {
                    miss: 0.0,
                    crit: 0.0
                })
            },
            Hit {
                time_ms: 4000,
                amount: 505757.0,
                kind: Kind::Strike,
                melee_table: Some(MeleeTable {
                    miss: 0.0,
                    crit: 0.0
                })
            },
            Hit {
                time_ms: 5003,
                amount: 0.0,
                kind: Kind::BleedApplication,
                melee_table: None,
            },
            Hit {
                time_ms: 7004,
                amount: 25000.0,
                kind: Kind::Bleed,
                melee_table: None
            },
        ]
    );
    assert!(parse_trace("not a damage trace", "Tank_Dummy_Dungeon").is_err());
}
