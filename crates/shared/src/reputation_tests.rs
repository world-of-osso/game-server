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
