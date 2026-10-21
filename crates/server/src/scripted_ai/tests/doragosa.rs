//! Current Retail LittleWigs2777a9d EchoOfDoragosa.lua and native70448.
use super::*;

const DORAGOSA: u32 = 190609;
const MAP: u32 = 2526;
const BIT: u32 = 3;
const MISSILES: u32 = 373326;

fn doragosa_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(DORAGOSA, MAP, BIT, difficulty, None);
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
fn encounter_coverage_doragosa_current_native_inputs_qualify() {
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    for (spell, expected) in [
        (MISSILES, (2, 25, 0)),
        (1282251, (32, 6, 1282252)),
        (374343, (3, 18, 374350)),
        (388822, (2, 87, 0)),
    ] {
        let actual: (u32,u32,u32) = conn.query_row("SELECT Effect,ImplicitTarget_0,EffectTriggerSpell FROM spell_effect WHERE SpellID=? AND DifficultyID=0 AND EffectIndex=0", [spell], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        assert_eq!(actual, expected, "current native {spell}");
    }
    assert_eq!(
        conn.query_row(
            "SELECT MapID,Bit FROM dungeon_encounter WHERE ID=2565",
            [],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
        )
        .unwrap(),
        (MAP, BIT)
    );
}

#[test]
fn encounter_coverage_doragosa_ordinary_native_missiles_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = doragosa_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, MISSILES, Some(player), false)
            .expect("native373326 admission");
        h.advance(4.0);
        assert_eq!(cast_count(&h, boss, MISSILES), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(MISSILES)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())),
            "native missiles damage difficulty{difficulty}"
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
#[ignore = "RED: current Retail Doragosa Lua-clock port absent"]
fn encounter_coverage_doragosa_first_missiles_uses_current_retail_clock() {
    let (mut h, boss, _) = doragosa_copy(205);
    h.engage(boss);
    h.advance(11.0);
    assert_eq!(
        cast_count(&h, boss, MISSILES),
        1,
        "current Retail first7s request, not legacy Astral Breath"
    );
}
