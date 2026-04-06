use super::*;

// Race IDs (matching WoW convention)
const HUMAN: u8 = 1;
const ORC: u8 = 2;
const DWARF: u8 = 3;
const NIGHT_ELF: u8 = 4;
const UNDEAD: u8 = 5;
const TAUREN: u8 = 6;

fn stormwind() -> FactionData {
    FactionData {
        id: 72,
        name: "Stormwind".to_string(),
        base_standing: vec![
            (HUMAN, 21_000),     // Friendly
            (DWARF, 21_000),     // Friendly
            (NIGHT_ELF, 12_000), // Neutral
            (ORC, 0),            // Hated
            (UNDEAD, 0),         // Hated
            (TAUREN, 6_000),     // Hostile
        ],
    }
}

fn orgrimmar() -> FactionData {
    FactionData {
        id: 76,
        name: "Orgrimmar".to_string(),
        base_standing: vec![
            (ORC, 21_000),       // Friendly
            (TAUREN, 21_000),    // Friendly
            (UNDEAD, 12_000),    // Neutral
            (HUMAN, 0),          // Hated
            (DWARF, 0),          // Hated
        ],
    }
}

// --- Standing tiers ---

#[test]
fn standing_hated() {
    assert_eq!(standing_for_value(0), Standing::Hated);
    assert_eq!(standing_for_value(5_999), Standing::Hated);
}

#[test]
fn standing_hostile() {
    assert_eq!(standing_for_value(6_000), Standing::Hostile);
    assert_eq!(standing_for_value(8_999), Standing::Hostile);
}

#[test]
fn standing_unfriendly() {
    assert_eq!(standing_for_value(9_000), Standing::Unfriendly);
    assert_eq!(standing_for_value(11_999), Standing::Unfriendly);
}

#[test]
fn standing_neutral() {
    assert_eq!(standing_for_value(12_000), Standing::Neutral);
    assert_eq!(standing_for_value(20_999), Standing::Neutral);
}

#[test]
fn standing_friendly() {
    assert_eq!(standing_for_value(21_000), Standing::Friendly);
    assert_eq!(standing_for_value(29_999), Standing::Friendly);
}

#[test]
fn standing_honored() {
    assert_eq!(standing_for_value(30_000), Standing::Honored);
    assert_eq!(standing_for_value(41_999), Standing::Honored);
}

#[test]
fn standing_revered() {
    assert_eq!(standing_for_value(42_000), Standing::Revered);
    assert_eq!(standing_for_value(62_999), Standing::Revered);
}

#[test]
fn standing_exalted() {
    assert_eq!(standing_for_value(63_000), Standing::Exalted);
    assert_eq!(standing_for_value(83_999), Standing::Exalted);
}

#[test]
fn standing_clamps_to_bounds() {
    assert_eq!(standing_for_value(-100), Standing::Hated);
    assert_eq!(standing_for_value(100_000), Standing::Exalted);
}

// --- Threshold lookup ---

#[test]
fn threshold_values() {
    assert_eq!(threshold_for_standing(Standing::Hated), 0);
    assert_eq!(threshold_for_standing(Standing::Neutral), 12_000);
    assert_eq!(threshold_for_standing(Standing::Exalted), 63_000);
}

// --- Tier progress ---

#[test]
fn progress_at_tier_start() {
    let (into, size) = tier_progress(12_000);
    assert_eq!(into, 0);
    assert_eq!(size, 9_000); // Neutral: 12000–20999
}

#[test]
fn progress_midway() {
    let (into, size) = tier_progress(15_000);
    assert_eq!(into, 3_000);
    assert_eq!(size, 9_000);
}

#[test]
fn progress_exalted() {
    let (into, size) = tier_progress(70_000);
    assert_eq!(into, 7_000);
    assert_eq!(size, 21_000); // Exalted: 63000–83999
}

// --- FactionData ---

#[test]
fn base_rep_for_known_race() {
    let sw = stormwind();
    assert_eq!(sw.base_rep_for_race(HUMAN), 21_000);
    assert_eq!(sw.base_rep_for_race(ORC), 0);
    assert_eq!(sw.base_rep_for_race(TAUREN), 6_000);
}

#[test]
fn base_rep_unknown_race_defaults_neutral() {
    let sw = stormwind();
    // Race 99 not in base_standing → neutral
    assert_eq!(sw.base_rep_for_race(99), REP_NEUTRAL);
}

#[test]
fn starting_standing_for_race() {
    let sw = stormwind();
    assert_eq!(sw.starting_standing(HUMAN), Standing::Friendly);
    assert_eq!(sw.starting_standing(ORC), Standing::Hated);
    assert_eq!(sw.starting_standing(TAUREN), Standing::Hostile);
    assert_eq!(sw.starting_standing(NIGHT_ELF), Standing::Neutral);
}

// --- FactionRegistry ---

#[test]
fn registry_add_and_get() {
    let mut reg = FactionRegistry::new();
    reg.add(stormwind());
    reg.add(orgrimmar());
    assert_eq!(reg.len(), 2);

    let sw = reg.get(72).unwrap();
    assert_eq!(sw.name, "Stormwind");
}

#[test]
fn registry_find_by_name() {
    let mut reg = FactionRegistry::new();
    reg.add(stormwind());
    assert_eq!(reg.find_by_name("Stormwind").unwrap().id, 72);
    assert!(reg.find_by_name("Unknown").is_none());
}

#[test]
fn registry_replace_existing() {
    let mut reg = FactionRegistry::new();
    reg.add(stormwind());
    let mut updated = stormwind();
    updated.name = "Stormwind City".to_string();
    reg.add(updated);
    assert_eq!(reg.len(), 1);
    assert_eq!(reg.get(72).unwrap().name, "Stormwind City");
}

#[test]
fn registry_unknown_id() {
    let reg = FactionRegistry::new();
    assert!(reg.get(999).is_none());
}

#[test]
fn registry_iter() {
    let mut reg = FactionRegistry::new();
    reg.add(stormwind());
    reg.add(orgrimmar());
    let ids: Vec<u32> = reg.iter().map(|f| f.id).collect();
    assert!(ids.contains(&72));
    assert!(ids.contains(&76));
}

// --- Standing ordering ---

#[test]
fn standing_order() {
    assert!(Standing::Hated < Standing::Hostile);
    assert!(Standing::Hostile < Standing::Neutral);
    assert!(Standing::Neutral < Standing::Friendly);
    assert!(Standing::Friendly < Standing::Exalted);
}

// --- CharacterReputation ---

fn sample_registry() -> FactionRegistry {
    let mut reg = FactionRegistry::new();
    reg.add(stormwind());
    reg.add(orgrimmar());
    reg
}

#[test]
fn new_for_race_human() {
    let reg = sample_registry();
    let rep = CharacterReputation::new_for_race(&reg, HUMAN);

    assert_eq!(rep.get_value(72), 21_000); // Stormwind: Friendly for humans
    assert_eq!(rep.get_standing(72), Standing::Friendly);
    assert_eq!(rep.get_value(76), 0); // Orgrimmar: Hated for humans
    assert_eq!(rep.get_standing(76), Standing::Hated);
    assert_eq!(rep.faction_count(), 2);
}

#[test]
fn new_for_race_orc() {
    let reg = sample_registry();
    let rep = CharacterReputation::new_for_race(&reg, ORC);

    assert_eq!(rep.get_standing(72), Standing::Hated);
    assert_eq!(rep.get_standing(76), Standing::Friendly);
}

#[test]
fn gain_rep_quest_turnin() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    let base = rep.get_value(72); // 21000

    let result = rep.gain_rep(72, 500, RepGainSource::QuestTurnIn);
    assert_eq!(result.old_value, base);
    assert_eq!(result.new_value, base + 500);
    assert_eq!(result.actual_change(), 500);
    assert!(!result.tier_changed()); // Still Friendly
}

#[test]
fn gain_rep_crosses_tier() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    // Human starts at 21000 (Friendly). Need 9000 more for Honored (30000).
    let result = rep.gain_rep(72, 9_000, RepGainSource::MobKill);

    assert_eq!(result.new_value, 30_000);
    assert_eq!(result.old_standing, Standing::Friendly);
    assert_eq!(result.new_standing, Standing::Honored);
    assert!(result.tier_changed());
}

#[test]
fn gain_rep_mob_kill() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);

    rep.gain_rep(72, 10, RepGainSource::MobKill);
    assert_eq!(rep.get_value(72), 21_010);
}

#[test]
fn gain_rep_repeatable_turnin() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);

    rep.gain_rep(72, 250, RepGainSource::RepeatableTurnIn);
    rep.gain_rep(72, 250, RepGainSource::RepeatableTurnIn);
    assert_eq!(rep.get_value(72), 21_500);
}

#[test]
fn lose_rep() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);

    let result = rep.lose_rep(72, 1_000, RepGainSource::MobKill);
    assert_eq!(result.new_value, 20_000);
    assert_eq!(result.actual_change(), -1_000);
}

#[test]
fn lose_rep_crosses_tier_down() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    // 21000 (Friendly) - 2000 = 19000 (Neutral)
    let result = rep.lose_rep(72, 2_000, RepGainSource::MobKill);
    assert_eq!(result.old_standing, Standing::Friendly);
    assert_eq!(result.new_standing, Standing::Neutral);
    assert!(result.tier_changed());
}

#[test]
fn rep_clamps_at_max() {
    let mut rep = CharacterReputation::default();
    rep.set_rep(72, 80_000);
    let result = rep.gain_rep(72, 10_000, RepGainSource::Admin);
    assert_eq!(result.new_value, REP_MAX);
    assert_eq!(result.actual_change(), REP_MAX - 80_000);
}

#[test]
fn rep_clamps_at_min() {
    let mut rep = CharacterReputation::default();
    rep.set_rep(72, 1_000);
    let result = rep.lose_rep(72, 5_000, RepGainSource::MobKill);
    assert_eq!(result.new_value, REP_MIN);
}

#[test]
fn unknown_faction_defaults_neutral() {
    let rep = CharacterReputation::default();
    assert_eq!(rep.get_value(999), REP_NEUTRAL);
    assert_eq!(rep.get_standing(999), Standing::Neutral);
}

#[test]
fn has_standing_check() {
    let reg = sample_registry();
    let rep = CharacterReputation::new_for_race(&reg, HUMAN);

    // Stormwind at Friendly
    assert!(rep.has_standing(72, Standing::Friendly));
    assert!(rep.has_standing(72, Standing::Neutral));
    assert!(!rep.has_standing(72, Standing::Honored));
}

#[test]
fn set_rep_directly() {
    let mut rep = CharacterReputation::default();
    rep.set_rep(72, 42_000);
    assert_eq!(rep.get_standing(72), Standing::Revered);
}

#[test]
fn tracked_factions() {
    let reg = sample_registry();
    let rep = CharacterReputation::new_for_race(&reg, HUMAN);
    let factions = rep.tracked_factions();
    assert!(factions.contains(&72));
    assert!(factions.contains(&76));
}

#[test]
fn gain_rep_ensures_positive() {
    let mut rep = CharacterReputation::default();
    rep.set_rep(72, 20_000);
    // gain_rep takes abs, so negative input still gains
    let result = rep.gain_rep(72, -500, RepGainSource::QuestTurnIn);
    assert_eq!(result.new_value, 20_500);
}

#[test]
fn lose_rep_ensures_negative() {
    let mut rep = CharacterReputation::default();
    rep.set_rep(72, 20_000);
    // lose_rep takes abs, so positive input still loses
    let result = rep.lose_rep(72, 500, RepGainSource::MobKill);
    assert_eq!(result.new_value, 19_500);
}
