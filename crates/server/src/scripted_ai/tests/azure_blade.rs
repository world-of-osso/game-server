//! LittleWigs2777a9d Azureblade.lua + native70448; partial Lua-clock port.
use super::*;

const AZUREBLADE: u32 = 186739;
const MAP: u32 = 2515;
/// Preserve installed Azure saved ordinal1; DB2 encounter2585 completion bit3.
const ORDINAL: u32 = 1;
const ARCANE_CLEAVE: u32 = 372222;
const IMAGE: u32 = 384223;
const ANCIENT_ORB: u32 = 385578;

fn azureblade_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(AZUREBLADE, MAP, ORDINAL, difficulty, None);
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
fn encounter_coverage_azureblade_native_inputs_qualify() {
    let all = crate::spell_cast::test_harness::world_db().expect("native70448");
    for id in [ARCANE_CLEAVE, IMAGE, ANCIENT_ORB, 373658, 373582, 384132] {
        assert!(all.store.get(id).is_some(), "native spell{id}");
    }
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let cleave: (u32, u32, u32) = conn.query_row(
        "SELECT Effect,ImplicitTarget_0,EffectRadiusIndex_0 FROM spell_effect WHERE SpellID=372222 AND DifficultyID=0 AND EffectIndex=0",
        [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(cleave, (2, 104, 13), "native destination-facing cone");
    let image: (u32,u32,u32) = conn.query_row(
        "SELECT Effect,EffectMiscValue_0,EffectMiscValue_1 FROM spell_effect WHERE SpellID=373582 AND DifficultyID=0 AND EffectIndex=0",
        [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(image, (28, 190187, 64), "native image chain actor/property");
}

#[test]
fn encounter_coverage_azureblade_ordinary_cleave_damage_qualifies() {
    let (mut h, boss, _) = azureblade_copy(205);
    let player = h.player();
    h.engage(boss);
    h.advance(0.05);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, ARCANE_CLEAVE, Some(player), false)
        .expect("actual ordinary372222 admission");
    h.advance(2.1);
    assert_eq!(cast_count(&h, boss, ARCANE_CLEAVE), 1);
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(ARCANE_CLEAVE)
        }),
        "actual native cone damages intended player"
    );
    assert_eq!(cast_count(&h, boss, IMAGE), 0);
    assert_eq!(cast_count(&h, boss, ANCIENT_ORB), 0);
}

#[test]
#[ignore = "AZUREBLADE_LUA_CLOCK_PORT_PENDING"]
fn encounter_coverage_azureblade_first_cleave_uses_lua_clock() {
    let (mut h, boss, _) = azureblade_copy(205);
    h.engage(boss);
    h.advance(8.0);
    assert_eq!(
        cast_count(&h, boss, ARCANE_CLEAVE),
        1,
        "Lua OnEngage5.2s request + actual native2s cast"
    );
}
