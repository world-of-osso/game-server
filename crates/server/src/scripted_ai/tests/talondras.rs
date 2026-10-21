//! Current LittleWigs2777a9d SentinelTalondras.lua + native70448.
use super::*;

const TALONDRAS: u32 = 184124;
const MAP: u32 = 2451;
const BIT: u32 = 2;
const STOMP: u32 = 372701;
const SHARDS: u32 = 372718;

fn talondras_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(TALONDRAS, MAP, BIT, difficulty, None);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|g| g.caster == boss.to_bits() && g.spell_id == spell)
        .count()
}

#[test]
fn encounter_coverage_talondras_native_stomp_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = talondras_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, STOMP, Some(boss), false)
            .expect("native372701");
        h.advance(2.5);
        assert_eq!(cast_count(&h, boss, STOMP), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(STOMP)
                    && e.source == Some(boss.to_bits())
                    && e.target == Some(player.to_bits())),
            "native Stomp damage difficulty{difficulty}"
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|e| e.source != Some(boss.to_bits()))
        );
    }
}

#[test]
fn encounter_coverage_talondras_native_shards_direct_and_periodic_qualify() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = talondras_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SHARDS, Some(player), false)
            .expect("native372718");
        h.advance(2.5);
        assert!(h.has_aura(player, SHARDS));
        for periodic in [false, true] {
            assert!(
                h.inbox(0)
                    .log
                    .iter()
                    .any(|e| e.kind == CombatLogKind::Damage
                        && e.amount > 0
                        && e.periodic == periodic
                        && e.spell_id == Some(SHARDS)
                        && e.source == Some(boss.to_bits())
                        && e.target == Some(player.to_bits())),
                "native direct/dot periodic={periodic} difficulty{difficulty}"
            );
        }
        assert!(!h.has_aura(h.players[1].0, SHARDS));
    }
}

#[test]
#[ignore = "RED: Talondras current Lua-clock port absent"]
fn encounter_coverage_talondras_first_stomp_uses_current_lua_clock() {
    let (mut h, boss, _) = talondras_copy(205);
    h.engage(boss);
    h.advance(8.0);
    assert_eq!(
        cast_count(&h, boss, STOMP),
        1,
        "current Lua5.1s request/native2s cast"
    );
}
