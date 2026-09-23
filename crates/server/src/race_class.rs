//! Playable race/class combinations.
//!
//! world.db has no `chr_races` or `CharBaseInfo` table, so the combinations are
//! kept here. Classes per race match the client character-creation table
//! (`game-engine src/scenes/char_create/data.rs` RACES). `race_bit` is
//! `ChrRaces.PlayableRaceBit` (`game-engine data/ChrRaces.csv`), the bit that
//! DB2 `RaceMasks` use.

/// ChrClasses IDs with class spell data (no Monk, Demon Hunter or Evoker).
pub const PLAYABLE_CLASSES: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 11];

struct PlayableRace {
    id: u8,
    race_bit: u8,
    classes: &'static [u8],
}

const PANDAREN_CLASSES: &[u8] = &[1, 3, 4, 5, 7, 8];

const PLAYABLE_RACES: &[PlayableRace] = &[
    race(1, 0, &[1, 2, 3, 4, 5, 6, 8, 9]),       // Human
    race(2, 1, &[1, 3, 4, 6, 7, 8, 9]),          // Orc
    race(3, 2, &[1, 2, 3, 4, 5, 6]),             // Dwarf
    race(4, 3, &[1, 3, 4, 5, 6, 11]),            // Night Elf
    race(5, 4, &[1, 4, 5, 6, 8, 9]),             // Undead
    race(6, 5, &[1, 3, 6, 7, 11]),               // Tauren
    race(7, 6, &[1, 4, 6, 8, 9]),                // Gnome
    race(8, 7, &[1, 3, 4, 5, 6, 7, 8]),          // Troll
    race(9, 8, &[1, 3, 4, 5, 6, 7, 8, 9]),       // Goblin
    race(10, 9, &[2, 3, 4, 5, 6, 8, 9]),         // Blood Elf
    race(11, 10, &[1, 2, 3, 5, 6, 7, 8]),        // Draenei
    race(22, 21, &[1, 3, 4, 5, 6, 8, 9, 11]),    // Worgen
    race(24, 23, PANDAREN_CLASSES),              // Pandaren (neutral)
    race(25, 24, PANDAREN_CLASSES),              // Pandaren (Alliance)
    race(26, 25, PANDAREN_CLASSES),              // Pandaren (Horde)
    race(27, 26, &[1, 3, 4, 5, 6, 8, 9]),        // Nightborne
    race(28, 27, &[1, 3, 5, 6, 7, 11]),          // Highmountain Tauren
    race(29, 28, &[1, 3, 4, 5, 6, 8, 9]),        // Void Elf
    race(30, 29, &[1, 2, 3, 5, 6, 8]),           // Lightforged Draenei
    race(31, 30, &[1, 2, 3, 4, 5, 6, 7, 8, 11]), // Zandalari Troll
    race(34, 11, &[1, 2, 3, 4, 5, 6, 7, 8, 9]),  // Dark Iron Dwarf
    race(35, 12, &[1, 3, 4, 5, 7, 8, 9]),        // Vulpera
    race(36, 13, &[1, 3, 4, 5, 6, 7, 8]),        // Mag'har Orc
    race(37, 14, &[1, 3, 4, 5, 6, 8, 9]),        // Mechagnome
];

const fn race(id: u8, race_bit: u8, classes: &'static [u8]) -> PlayableRace {
    PlayableRace {
        id,
        race_bit,
        classes,
    }
}

fn playable_race(race: u8) -> Option<&'static PlayableRace> {
    PLAYABLE_RACES.iter().find(|entry| entry.id == race)
}

/// Reject classes outside `PLAYABLE_CLASSES`, unknown races, and combinations
/// the race cannot play.
pub fn validate_race_class(race: u8, class: u8) -> Result<(), String> {
    let Some(entry) = playable_race(race) else {
        return Err(format!("Invalid race id {race}"));
    };
    if !PLAYABLE_CLASSES.contains(&class) {
        return Err(format!("Invalid class id {class}"));
    }
    if !entry.classes.contains(&class) {
        return Err(format!("Class {class} is not available for race {race}"));
    }
    Ok(())
}

/// True when a DB2 `RaceMasks` pair (low, high 32 bits) allows the race.
/// An all-zero mask allows every race.
pub fn race_mask_allows(race_masks: [u32; 2], race: u8) -> bool {
    if race_masks == [0, 0] {
        return true;
    }
    let Some(entry) = playable_race(race) else {
        return false;
    };
    let mask = u64::from(race_masks[0]) | (u64::from(race_masks[1]) << 32);
    mask & (1 << entry.race_bit) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_race_class_accepts_listed_combinations_only() {
        assert_eq!(validate_race_class(1, 2), Ok(()));
        assert_eq!(validate_race_class(6, 11), Ok(()));
        assert_eq!(
            validate_race_class(1, 10),
            Err("Invalid class id 10".into())
        );
        assert_eq!(
            validate_race_class(1, 12),
            Err("Invalid class id 12".into())
        );
        assert_eq!(
            validate_race_class(3, 11),
            Err("Class 11 is not available for race 3".into())
        );
        assert_eq!(
            validate_race_class(255, 1),
            Err("Invalid race id 255".into())
        );
    }

    #[test]
    fn race_mask_uses_playable_race_bit() {
        // Summon Warhorse 13819: RaceMasks_0 2053 = Human, Dwarf, Dark Iron Dwarf.
        assert!(race_mask_allows([2053, 0], 1));
        assert!(race_mask_allows([2053, 0], 34));
        assert!(!race_mask_allows([2053, 0], 10));
        assert!(race_mask_allows([0, 0], 10));
    }
}
