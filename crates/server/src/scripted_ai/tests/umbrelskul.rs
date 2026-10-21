//! LittleWigs2777a9d Umbrelskul.lua + native70448; partial Lua-clock combat.
use super::*;

const UMBRELSKUL: u32 = 186738;
const MAP: u32 = 2515;
const ORDINAL: u32 = 3;
const DRAGON_STRIKE: u32 = 384978;
const CRYSTALLINE_ROAR: u32 = 384699;

fn umbrelskul_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(UMBRELSKUL, MAP, ORDINAL, difficulty, None);
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
fn encounter_coverage_umbrelskul_native_inputs_qualify() {
    let all = crate::spell_cast::test_harness::world_db().expect("native70448");
    for id in [
        DRAGON_STRIKE,
        CRYSTALLINE_ROAR,
        384696,
        385075,
        385399,
        388804,
        386746,
    ] {
        assert!(all.store.get(id).is_some(), "native spell{id}");
    }
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let strike:Vec<(u32,u32,u32)>=conn.prepare(
        "SELECT EffectIndex,Effect,EffectAura FROM spell_effect WHERE SpellID=384978 AND DifficultyID=0 ORDER BY EffectIndex").unwrap()
        .query_map([],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap().map(Result::unwrap).collect();
    assert_eq!(strike, [(0, 2, 0), (1, 6, 3), (2, 138, 0)]);
    let roar:(u32,u32,u32)=conn.query_row(
        "SELECT Effect,ImplicitTarget_0,EffectRadiusIndex_0 FROM spell_effect WHERE SpellID=384699 AND DifficultyID=0 AND EffectIndex=1",[],
        |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(roar.0, 2);
    assert_eq!(
        roar.1, 129,
        "native caster-facing line, not fabricated area"
    );
}

#[test]
fn encounter_coverage_umbrelskul_ordinary_strike_damage_and_periodic_qualify() {
    let (mut h, boss, _) = umbrelskul_copy(205);
    let player = h.player();
    h.engage(boss);
    h.advance(0.05);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, DRAGON_STRIKE, Some(player), false)
        .expect("ordinary actual384978 admission");
    h.advance(4.6);
    assert_eq!(cast_count(&h, boss, DRAGON_STRIKE), 1);
    assert!(
        h.has_aura(player, DRAGON_STRIKE),
        "native damage-over-time aura"
    );
    let damage: Vec<_> = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event.spell_id == Some(DRAGON_STRIKE)
                && event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
        })
        .collect();
    assert!(
        damage.len() >= 2,
        "actual direct damage plus periodic tick(s): {damage:?}"
    );
    assert!(
        !h.has_aura(h.players[1].0, DRAGON_STRIKE),
        "other copy excluded"
    );
}

#[test]
#[ignore = "UMBRELSKUL_LUA_CLOCK_PORT_PENDING"]
fn encounter_coverage_umbrelskul_first_strike_uses_lua_clock() {
    let (mut h, boss, _) = umbrelskul_copy(205);
    h.engage(boss);
    h.advance(10.1);
    assert_eq!(
        cast_count(&h, boss, DRAGON_STRIKE),
        1,
        "Lua7.4s first request/native2.5s cast"
    );
}
