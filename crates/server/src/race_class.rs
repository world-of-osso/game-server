//! Playable race/class combinations.
//!
//! world.db has no `chr_races` or `CharBaseInfo` table, so the combinations are
//! kept here. Classes per race match the client character-creation table
//! (`game-engine src/scenes/char_create/data.rs` RACES). Monk (10), Demon Hunter
//! (12) and Evoker (13), and the Dracthyr rows, follow Retail `CharBaseInfo`
//! (FileDataID 1343386, 12.1.0.69933): Monk except Dracthyr, Demon Hunter
//! for Night Elf, Blood Elf and Void Elf, Evoker for Dracthyr only. `race_bit` is
//! `ChrRaces.PlayableRaceBit` (`game-engine data/ChrRaces.csv`), the bit that
//! DB2 `RaceMasks` use.

/// ChrClasses IDs with class spell data.
pub const PLAYABLE_CLASSES: [u8; 13] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];

struct PlayableRace {
    id: u8,
    race_bit: u8,
    classes: &'static [u8],
}

const PANDAREN_CLASSES: &[u8] = &[1, 3, 4, 5, 7, 8, 10];
const DRACTHYR_CLASSES: &[u8] = &[1, 3, 4, 5, 8, 9, 13];
/// CharBaseInfo classes of Kul Tiran, Earthen and Haranir (Adventurer 14 is not playable).
const KUL_TIRAN_CLASSES: &[u8] = &[1, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const EARTHEN_CLASSES: &[u8] = &[1, 2, 3, 4, 5, 7, 8, 9, 10];
const HARANIR_CLASSES: &[u8] = &[1, 3, 4, 5, 7, 8, 9, 10, 11];

const PLAYABLE_RACES: &[PlayableRace] = &[
    race(1, 0, &[1, 2, 3, 4, 5, 6, 8, 9, 10]),       // Human
    race(2, 1, &[1, 3, 4, 6, 7, 8, 9, 10]),          // Orc
    race(3, 2, &[1, 2, 3, 4, 5, 6, 10]),             // Dwarf
    race(4, 3, &[1, 3, 4, 5, 6, 10, 11, 12]),        // Night Elf
    race(5, 4, &[1, 4, 5, 6, 8, 9, 10]),             // Undead
    race(6, 5, &[1, 3, 6, 7, 10, 11]),               // Tauren
    race(7, 6, &[1, 4, 6, 8, 9, 10]),                // Gnome
    race(8, 7, &[1, 3, 4, 5, 6, 7, 8, 10]),          // Troll
    race(9, 8, &[1, 3, 4, 5, 6, 7, 8, 9, 10]),       // Goblin
    race(10, 9, &[2, 3, 4, 5, 6, 8, 9, 10, 12]),     // Blood Elf
    race(11, 10, &[1, 2, 3, 5, 6, 7, 8, 10]),        // Draenei
    race(22, 21, &[1, 3, 4, 5, 6, 8, 9, 10, 11]),    // Worgen
    race(24, 23, PANDAREN_CLASSES),                  // Pandaren (neutral)
    race(25, 24, PANDAREN_CLASSES),                  // Pandaren (Alliance)
    race(26, 25, PANDAREN_CLASSES),                  // Pandaren (Horde)
    race(27, 26, &[1, 3, 4, 5, 6, 8, 9, 10]),        // Nightborne
    race(28, 27, &[1, 3, 5, 6, 7, 10, 11]),          // Highmountain Tauren
    race(29, 28, &[1, 3, 4, 5, 6, 8, 9, 10, 12]),    // Void Elf
    race(30, 29, &[1, 2, 3, 5, 6, 8, 10]),           // Lightforged Draenei
    race(31, 30, &[1, 2, 3, 4, 5, 6, 7, 8, 10, 11]), // Zandalari Troll
    race(32, 31, KUL_TIRAN_CLASSES),                 // Kul Tiran
    race(34, 11, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),  // Dark Iron Dwarf
    race(35, 12, &[1, 3, 4, 5, 7, 8, 9, 10]),        // Vulpera
    race(36, 13, &[1, 3, 4, 5, 6, 7, 8, 10]),        // Mag'har Orc
    race(37, 14, &[1, 3, 4, 5, 6, 8, 9, 10]),        // Mechagnome
    race(52, 16, DRACTHYR_CLASSES),                  // Dracthyr (Alliance)
    race(70, 15, DRACTHYR_CLASSES),                  // Dracthyr (Horde)
    race(84, 17, EARTHEN_CLASSES),                   // Earthen (Horde)
    race(85, 18, EARTHEN_CLASSES),                   // Earthen (Alliance)
    race(86, 20, HARANIR_CLASSES),                   // Haranir (Alliance)
    race(91, 19, HARANIR_CLASSES),                   // Haranir (Horde)
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

/// New-class start levels are max(ChrClasses.StartingLevel,
/// ChrRaces.StartingLevel), DB2 12.1.0.69933: Monk 1, DH 8, Evoker 1;
/// allied races and Dracthyr 10. Death Knight starts remain unchanged in
/// this bounded new-class slice.
pub fn start_level(race: u8, class: u8) -> u8 {
    const NEW_CLASSES: [u8; 3] = [10, 12, 13];
    /// Races this slice made creatable: Kul Tiran, Dracthyr, Earthen, Haranir.
    const NEW_RACES: [u8; 7] = [32, 52, 70, 84, 85, 86, 91];
    if !NEW_CLASSES.contains(&class) && !NEW_RACES.contains(&race) {
        // The older classes of the older races keep level 1.
        return 1;
    }
    let class_level: u8 = if class == 12 { 8 } else { 1 };
    let race_level = match race {
        27..=32 | 34..=37 | 52 | 70 | 84 | 85 | 91 => 10,
        _ => 1,
    };
    class_level.max(race_level)
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
            validate_race_class(1, 14),
            Err("Invalid class id 14".into())
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

    /// Retail CharBaseInfo 12.1.0.69933: Monk for every race but Dracthyr,
    /// Demon Hunter for Night Elf, Blood Elf and Void Elf, Evoker for Dracthyr.
    #[test]
    fn new_classes_follow_retail_char_base_info() {
        // Every supported non-Dracthyr race has an authored Monk row.
        for race in [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 22, 24, 25, 26, 27, 28, 29, 30, 31, 34, 35, 36, 37,
        ] {
            assert_eq!(validate_race_class(race, 10), Ok(()), "Monk race {race}");
        }
        for (race, class) in [
            (1, 10),
            (2, 10),
            (25, 10),
            (26, 10),
            (37, 10),
            (32, 10),
            (84, 10),
            (85, 10),
            (86, 10),
            (91, 10),
            (4, 12),
            (10, 12),
            (29, 12),
            (52, 13),
            (70, 13),
            (52, 1),
            (70, 9),
        ] {
            assert_eq!(validate_race_class(race, class), Ok(()), "{race}/{class}");
        }
        for (race, class) in [
            (1, 12),
            (2, 12),
            (22, 12),
            (27, 12),
            (1, 13),
            (10, 13),
            (52, 10),
            (70, 12),
            (52, 2),
        ] {
            assert_eq!(
                validate_race_class(race, class),
                Err(format!("Class {class} is not available for race {race}")),
            );
        }
    }

    /// CharBaseInfo: Kul Tiran has no Paladin, Earthen no Death Knight or Druid,
    /// Haranir no Paladin or Death Knight.
    #[test]
    fn kul_tiran_earthen_and_haranir_follow_char_base_info() {
        for (race, class) in [
            (32, 1),
            (32, 6),
            (32, 11),
            (84, 2),
            (85, 9),
            (86, 11),
            (91, 7),
        ] {
            assert_eq!(validate_race_class(race, class), Ok(()), "{race}/{class}");
        }
        for (race, class) in [(32, 2), (84, 6), (85, 11), (86, 2), (91, 6)] {
            assert!(validate_race_class(race, class).is_err(), "{race}/{class}");
        }
    }

    #[test]
    fn demon_hunters_start_at_8_and_dracthyr_at_10() {
        assert_eq!(start_level(4, 12), 8);
        assert_eq!(start_level(10, 12), 8);
        assert_eq!(start_level(52, 13), 10);
        assert_eq!(start_level(70, 1), 10);
        assert_eq!(start_level(1, 10), 1);
        assert_eq!(start_level(29, 12), 10, "race level exceeds DH level");
        assert_eq!(start_level(37, 10), 10, "allied race Monk");
        assert_eq!(start_level(32, 10), 10, "Kul Tiran Monk");
        assert_eq!(start_level(84, 10), 10, "Earthen Horde Monk");
        assert_eq!(start_level(85, 10), 10, "Earthen Alliance Monk");
        assert_eq!(
            start_level(86, 10),
            1,
            "Haranir Alliance authored race level"
        );
        assert_eq!(start_level(91, 10), 10, "Haranir Horde authored race level");
        assert_eq!(start_level(1, 1), 1);
        assert_eq!(start_level(29, 8), 1, "older class on an older allied race");
        assert_eq!(start_level(32, 1), 10, "Kul Tiran Warrior");
    }

    #[test]
    fn dracthyr_race_bits_match_chr_races() {
        // ChrRaces.PlayableRaceBit: Dracthyr 52 is bit 16, 70 is bit 15.
        assert!(race_mask_allows([1 << 16, 0], 52));
        assert!(!race_mask_allows([1 << 16, 0], 70));
        assert!(race_mask_allows([1 << 15, 0], 70));
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
