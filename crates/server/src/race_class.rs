//! Retail eligibility and race-mask bits from installed `data/db2/active` CSVs.
//! Read once for this server process; missing/corrupt source never selects an old
//! Retail table. Supported gameplay classes and start-level/visage policy remain
//! unchanged. Separately contracted Forever95/96 gates retain their pinned source.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

/// ChrClasses IDs with class spell data.
pub const PLAYABLE_CLASSES: [u8; 13] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];

struct PlayableRace {
    id: u8,
    race_bit: u8,
    classes: BTreeSet<u8>,
}

/// Skyborne classes from WoW Forever 1.60.1 CharBaseInfo (FileDataID 1343386).
const SKYBORNE_95_CLASSES: &[u8] = &[1, 3, 4, 8, 11];
const SKYBORNE_96_CLASSES: &[u8] = &[1, 3, 4, 7, 11];

#[derive(Deserialize)]
struct RaceMetadataRow {
    #[serde(rename = "ID")]
    id: u8,
    #[serde(rename = "PlayableRaceBit")]
    race_bit: i16,
}

#[derive(Deserialize)]
struct RaceClassRow {
    #[serde(rename = "RaceID")]
    race: u8,
    #[serde(rename = "ClassID")]
    class: u8,
}

fn decode_playable_race(row: RaceMetadataRow) -> Result<Option<PlayableRace>, String> {
    if row.race_bit < 0 {
        return Ok(None);
    }
    let race_bit = u8::try_from(row.race_bit).map_err(|_| {
        format!(
            "race {} has invalid PlayableRaceBit {}",
            row.id, row.race_bit
        )
    })?;
    if u32::from(race_bit) >= u64::BITS {
        return Err(format!(
            "race {} PlayableRaceBit {race_bit} exceeds RaceMasks",
            row.id
        ));
    }
    Ok(Some(PlayableRace {
        id: row.id,
        race_bit,
        classes: BTreeSet::new(),
    }))
}

fn read_race_metadata(path: &Path) -> Result<BTreeMap<u8, PlayableRace>, String> {
    let mut reader =
        csv::Reader::from_path(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut races = BTreeMap::new();
    for row in reader.deserialize::<RaceMetadataRow>() {
        let row = row.map_err(|error| format!("{}: {error}", path.display()))?;
        let Some(race) = decode_playable_race(row)? else {
            continue;
        };
        let id = race.id;
        if races.insert(id, race).is_some() {
            return Err(format!("{}: duplicate race {id}", path.display()));
        }
    }
    Ok(races)
}

fn read_class_memberships(
    path: &Path,
    races: &mut BTreeMap<u8, PlayableRace>,
) -> Result<(), String> {
    let mut reader =
        csv::Reader::from_path(path).map_err(|error| format!("{}: {error}", path.display()))?;
    for row in reader.deserialize::<RaceClassRow>() {
        let row = row.map_err(|error| format!("{}: {error}", path.display()))?;
        if !PLAYABLE_CLASSES.contains(&row.class) {
            continue;
        }
        let race = races.get_mut(&row.race).ok_or_else(|| {
            format!(
                "{}: race {} has no playable ChrRaces metadata",
                path.display(),
                row.race
            )
        })?;
        race.classes.insert(row.class);
    }
    Ok(())
}

fn insert_skyborne_races(races: &mut BTreeMap<u8, PlayableRace>) -> Result<(), String> {
    // Pinned Forever product contract, not a substitute for missing Retail source.
    for (id, race_bit, classes) in [(95, 32, SKYBORNE_95_CLASSES), (96, 33, SKYBORNE_96_CLASSES)] {
        if races.contains_key(&id) {
            return Err(format!(
                "Retail playable race {id} collides with the Forever race contract"
            ));
        }
        races.insert(
            id,
            PlayableRace {
                id,
                race_bit,
                classes: classes.iter().copied().collect(),
            },
        );
    }
    Ok(())
}

fn load_playable_races(directory: &Path) -> Result<BTreeMap<u8, PlayableRace>, String> {
    let mut races = read_race_metadata(&directory.join("ChrRaces.csv"))?;
    read_class_memberships(&directory.join("CharBaseInfo.csv"), &mut races)?;
    races.retain(|_, race| !race.classes.is_empty());
    if races.is_empty() {
        return Err(format!(
            "{}: no playable Retail race/class pairs",
            directory.display()
        ));
    }
    insert_skyborne_races(&mut races)?;
    Ok(races)
}

fn read_installed_races() -> &'static BTreeMap<u8, PlayableRace> {
    static RACES: OnceLock<BTreeMap<u8, PlayableRace>> = OnceLock::new();
    RACES.get_or_init(|| {
        load_playable_races(Path::new("data/db2/active"))
            .unwrap_or_else(|error| panic!("Unable to load installed race/class source: {error}"))
    })
}

fn playable_race(race: u8) -> Option<&'static PlayableRace> {
    read_installed_races().get(&race)
}

/// Every playable (race, class) combination, by race then class.
pub fn playable_pairs() -> impl Iterator<Item = (u8, u8)> {
    read_installed_races()
        .values()
        .flat_map(|entry| entry.classes.iter().map(move |&class| (entry.id, class)))
}

/// `ChrRaces.PlayableRaceBit` of a playable race (TrinityCore `RaceMask::GetRaceBit`).
pub fn race_bit(race: u8) -> Option<u8> {
    playable_race(race).map(|entry| entry.race_bit)
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

/// Races with `ChrRacesFlag::IsAlliedRace` 0x80000 in DB2 12.1.0.69933 `ChrRaces.Flags`
/// (game-engine `godot/core/data/db2/12.1.0.69933/ChrRaces.csv`; DBCEnums.h:370).
/// Existing start-level policy is separate from installed eligibility/race-bit loading.
const ALLIED_RACES: [u8; 14] = [27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 84, 85, 86, 91];
const RACE_PANDAREN_ALLIANCE: u8 = 25;
const RACE_PANDAREN_HORDE: u8 = 26;
const RACE_DRACTHYR_ALLIANCE: u8 = 52;
const RACE_DRACTHYR_HORDE: u8 = 70;
const CLASS_DEATH_KNIGHT: u8 = 6;
const CLASS_DEMON_HUNTER: u8 = 12;
/// worldserver.conf.dist:934-958 defaults: StartPlayerLevel, StartDeathKnightPlayerLevel,
/// StartDemonHunterPlayerLevel, StartAlliedRacePlayerLevel.
const START_PLAYER_LEVEL: u8 = 1;
const START_DEATH_KNIGHT_LEVEL: u8 = 8;
const START_DEMON_HUNTER_LEVEL: u8 = 8;
const START_ALLIED_RACE_LEVEL: u8 = 10;

/// Player::GetStartLevel (Player.cpp:24384-24398): 1, or 10 for an allied race or
/// Dracthyr; a Death Knight at least 8 (a Pandaren one at least 10), a Demon Hunter at
/// least 8. Monk and Evoker take their race's level.
pub fn start_level(race: u8, class: u8) -> u8 {
    let mut level = if ALLIED_RACES.contains(&race)
        || race == RACE_DRACTHYR_ALLIANCE
        || race == RACE_DRACTHYR_HORDE
    {
        START_ALLIED_RACE_LEVEL
    } else {
        START_PLAYER_LEVEL
    };
    if class == CLASS_DEATH_KNIGHT {
        let class_level = if race == RACE_PANDAREN_ALLIANCE || race == RACE_PANDAREN_HORDE {
            START_ALLIED_RACE_LEVEL
        } else {
            START_DEATH_KNIGHT_LEVEL
        };
        level = level.max(class_level);
    } else if class == CLASS_DEMON_HUNTER {
        level = level.max(START_DEMON_HUNTER_LEVEL);
    }
    level
}

/// Dracthyr (52 Alliance, 70 Horde) also have the visage form (ChrRaces 75).
pub fn has_visage_form(race: u8) -> bool {
    matches!(race, 52 | 70)
}

/// True when a DB2 `RaceMasks` pair (low, high 32 bits) allows the race.
/// An all-zero mask allows every race.
pub fn race_mask_allows(race_masks: [u32; 2], race: u8) -> bool {
    if race_masks == [0, 0] {
        return true;
    }
    race_mask_has_race(race_masks, race)
}

/// TrinityCore `RaceMask::HasRace`: the race's bit is set (an all-zero mask has none).
pub fn race_mask_has_race(race_masks: [u32; 2], race: u8) -> bool {
    let Some(bit) = race_bit(race) else {
        return false;
    };
    let mask = u64::from(race_masks[0]) | (u64::from(race_masks[1]) << 32);
    mask & (1 << bit) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn race_source_installed_char_base_info_pairs_are_admitted() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/db2/active/CharBaseInfo.csv");
        let mut reader = csv::Reader::from_path(path).unwrap();
        let headers = reader.headers().unwrap().clone();
        let race_column = headers.iter().position(|name| name == "RaceID").unwrap();
        let class_column = headers.iter().position(|name| name == "ClassID").unwrap();
        let mut pairs = std::collections::BTreeSet::new();
        for record in reader.records() {
            let record = record.unwrap();
            let race: u8 = record[race_column].parse().unwrap();
            let class: u8 = record[class_column].parse().unwrap();
            if PLAYABLE_CLASSES.contains(&class) {
                pairs.insert((race, class));
            }
        }
        assert!(pairs.contains(&(3, 7)), "installed Dwarf/Shaman source row");
        for (race, class) in pairs {
            assert_eq!(
                validate_race_class(race, class),
                Ok(()),
                "source {race}/{class}"
            );
        }
    }

    #[test]
    fn race_source_missing_or_empty_installed_tables_do_not_use_static_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let result = load_playable_races(directory.path());
        assert!(matches!(result, Err(error) if error.contains("ChrRaces.csv")));
        std::fs::write(
            directory.path().join("ChrRaces.csv"),
            "ID,PlayableRaceBit\n3,2\n",
        )
        .unwrap();
        let result = load_playable_races(directory.path());
        assert!(matches!(result, Err(error) if error.contains("CharBaseInfo.csv")));
        std::fs::write(
            directory.path().join("ChrRaces.csv"),
            "ID,PlayableRaceBit\n",
        )
        .unwrap();
        std::fs::write(
            directory.path().join("CharBaseInfo.csv"),
            "RaceID,ClassID\n",
        )
        .unwrap();
        let result = load_playable_races(directory.path());
        assert!(matches!(result, Err(error) if error.contains("no playable Retail")));
    }

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
    fn start_levels_follow_player_get_start_level() {
        assert_eq!(start_level(1, 1), 1, "Human Warrior");
        assert_eq!(start_level(1, 10), 1, "Human Monk");
        assert_eq!(start_level(1, 6), 8, "Human Death Knight");
        assert_eq!(start_level(5, 6), 8, "Undead Death Knight");
        assert_eq!(start_level(4, 12), 8, "Night Elf Demon Hunter");
        assert_eq!(start_level(10, 12), 8, "Blood Elf Demon Hunter");
        assert_eq!(start_level(52, 13), 10, "Dracthyr Evoker");
        assert_eq!(start_level(70, 1), 10, "Dracthyr Warrior");
        assert_eq!(start_level(29, 8), 10, "Void Elf Mage (allied race)");
        assert_eq!(start_level(29, 12), 10, "Void Elf Demon Hunter");
        assert_eq!(start_level(34, 6), 10, "Dark Iron Dwarf Death Knight");
        assert_eq!(start_level(37, 10), 10, "Mechagnome Monk");
        assert_eq!(start_level(32, 1), 10, "Kul Tiran Warrior");
        assert_eq!(start_level(84, 10), 10, "Earthen Monk");
        // IsAlliedRace, whatever the authored ChrRaces.StartingLevel (1).
        assert_eq!(start_level(86, 10), 10, "Haranir Alliance Monk");
        assert_eq!(start_level(91, 10), 10, "Haranir Horde Monk");
        assert_eq!(start_level(25, 10), 1, "Pandaren Monk");
    }

    #[test]
    fn skyborne_follows_forever_char_base_info() {
        for class in [1, 3, 4, 8, 11] {
            assert_eq!(validate_race_class(95, class), Ok(()), "95/{class}");
        }
        for class in [1, 3, 4, 7, 11] {
            assert_eq!(validate_race_class(96, class), Ok(()), "96/{class}");
        }
        for (race, class) in [(95, 7), (96, 8), (95, 2), (96, 10)] {
            assert_eq!(
                validate_race_class(race, class),
                Err(format!("Class {class} is not available for race {race}")),
            );
        }
    }

    #[test]
    fn skyborne_race_bits_use_high_race_mask_word() {
        // PlayableRaceBit 32/33 live in RaceMasks_1 bits 0/1.
        assert!(race_mask_allows([0, 1], 95));
        assert!(!race_mask_allows([0, 1], 96));
        assert!(race_mask_allows([0, 2], 96));
        assert!(!race_mask_allows([u32::MAX, 0], 95));
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
