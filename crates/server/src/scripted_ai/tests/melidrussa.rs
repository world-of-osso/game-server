//! Current LittleWigs2777a9d Retail wrappers and native70448, not legacy clocks.
use super::*;

const MELIDRUSSA: u32 = 188252;
const MAP: u32 = 2521;
const BIT: u32 = 1;
const HAILBURST: u32 = 396044;

fn melidrussa_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(MELIDRUSSA, MAP, BIT, difficulty, None);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

#[test]
fn encounter_coverage_melidrussa_native_retail_inputs_qualify() {
    let all = crate::spell_cast::test_harness::world_db().expect("native70448");
    for id in [1307297, 1307308, HAILBURST, 1311543, 396042, 372851] {
        assert!(all.store.get(id).is_some(), "actual currentnative{id}");
    }
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    for (wrapper, child) in [(1307297, HAILBURST), (1307308, 1311543)] {
        let effect:(u32,u32)=conn.query_row("SELECT Effect,EffectTriggerSpell FROM spell_effect WHERE SpellID=? AND DifficultyID=0 AND EffectIndex=0",[wrapper],|row|Ok((row.get(0)?,row.get(1)?))).unwrap();
        assert_eq!(
            effect,
            (3, child),
            "native explicit child field, not guessed from a name"
        );
    }
    assert_eq!(
        conn.query_row(
            "SELECT MapID,Bit FROM dungeon_encounter WHERE ID=2609",
            [],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
        )
        .unwrap(),
        (MAP, BIT)
    );
}

#[test]
fn encounter_coverage_melidrussa_ordinary_hailburst_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = melidrussa_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, HAILBURST, Some(boss), false)
            .expect("ordinary native396044 admission");
        h.advance(3.1);
        assert_eq!(cast_count(&h, boss, HAILBURST), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(HAILBURST)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())),
            "actual area enemy damage in difficulty{difficulty}"
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
    }
}

#[test]
#[ignore = "MELIDRUSSA_PARTIAL_PORT_PENDING"]
fn encounter_coverage_melidrussa_first_hailburst_runs_through_native_cast() {
    let (mut h, boss, _) = melidrussa_copy(205);
    h.engage(boss);
    h.advance(10.0);
    assert_eq!(
        cast_count(&h, boss, HAILBURST),
        1,
        "source initial ability/native3s cast, not melee-only"
    );
}
