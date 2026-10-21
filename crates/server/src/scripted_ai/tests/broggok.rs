//! TC a352b1fa boss_broggok.cpp; actual 70448 spells, no historical replacements.
use super::*;
use shared::components::UnitFlags;

const BROGGOK: u32 = 17380;
const MAP: u32 = 542;
const BIT: u32 = 2;
const ENCOUNTER: u32 = 1924;
const SLIME_SPRAY: u32 = 30913;
const POISON_CLOUD: u32 = 30916;

fn broggok_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    bound_boss_at_difficulty(BROGGOK, MAP, BIT, difficulty, None)
}

#[test]
fn encounter_coverage_broggok_native_inputs_qualify() {
    let conn = Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("read-only source data");
    let spells = crate::spell_cast::test_harness::world_db().expect("native70448 spells");
    for id in [SLIME_SPRAY, POISON_CLOUD] {
        assert!(spells.store.get(id).is_some(), "native spell{id}");
    }
    for (id, gap) in [
        (30917, "BROGGOK_POISON_BOLT_30917_ABSENT_70448"),
        (26837, "BROGGOK_SUMMON_TRIGGER_26837_ABSENT_70448"),
        (26838, "BROGGOK_DESPAWN_TRIGGER_26838_ABSENT_70448"),
        (30914, "BROGGOK_POISON_AURA_30914_ABSENT_70448"),
        (38462, "BROGGOK_POISON_AURA_38462_ABSENT_70448"),
    ] {
        assert!(spells.store.get(id).is_none(), "{gap}");
    }
    let cloud: (u32, u32) = conn
        .query_row(
            "SELECT Effect,EffectMiscValue_0 FROM spell_effect WHERE SpellID=30916 AND DifficultyID=0 AND EffectIndex=0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        cloud,
        (179, 33896),
        "actual native area trigger, not historical17662 summon"
    );
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    assert_eq!(
        maps.raid_creature_encounters.get(&(MAP, BROGGOK)),
        Some(&BIT)
    );
    assert_eq!(
        conn.query_row(
            "SELECT Name_lang FROM dungeon_encounter WHERE ID=?",
            [ENCOUNTER],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "Broggok"
    );
}

#[test]
#[ignore = "BROGGOK_SOURCE_PORT_PENDING: JustAppeared passive/uninteractible"]
fn encounter_coverage_broggok_appears_dormant_in_both_difficulties() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = broggok_copy(difficulty);
        assert!(
            h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::NOT_SELECTABLE != 0,
            "TC JustAppeared sets uninteractible in difficulty{difficulty}"
        );
        assert_eq!(
            h.app.world().get::<ReactState>(boss),
            Some(&ReactState::Passive)
        );
        h.advance(20.0);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::NotStarted)
        );
        assert!(h.inbox(0).starts.is_empty());
        assert!(h.inbox(0).spell_gos.is_empty());
        assert!(h.inbox(1).starts.is_empty());
    }
}
