//! LittleWigs2777a9d Emberon.lua + native70448; no synthetic spell effects.
use super::*;

const EMBERON: u32 = 184422;
const MAP: u32 = 2451;
const BIT: u32 = 3;
const CLAP: u32 = 369061;
const EMBERS: u32 = 369110;
const PURGING: u32 = 368990;
const PURGING_DAMAGE: u32 = 369025;

fn emberon_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(EMBERON, MAP, BIT, difficulty, None);
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

fn damage_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event.spell_id == Some(spell)
                && event.source == Some(boss.to_bits())
                && event.target == Some(h.player().to_bits())
        })
        .count()
}

#[test]
#[ignore = "encounter_coverage Emberon native qualification before source port"]
fn encounter_coverage_emberon_native_clap_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = emberon_copy(difficulty);
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, CLAP, Some(boss), false)
            .expect("ordinary native369061");
        h.advance(2.5);
        assert_eq!(cast_count(&h, boss, CLAP), 1);
        assert!(
            damage_count(&h, boss, CLAP) > 0,
            "native target104 cone difficulty{difficulty}"
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
#[ignore = "encounter_coverage Emberon native qualification before source port"]
fn encounter_coverage_emberon_native_unstable_embers_aura_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = emberon_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, EMBERS, Some(boss), false)
            .expect("ordinary native369110");
        h.advance(3.0);
        assert_eq!(cast_count(&h, boss, EMBERS), 1);
        assert!(
            h.has_aura(player, EMBERS),
            "native dummy aura difficulty{difficulty}"
        );
        assert!(!h.has_aura(h.players[1].0, EMBERS));
    }
}

#[test]
#[ignore = "encounter_coverage Emberon native qualification before source port"]
fn encounter_coverage_emberon_native_purging_periodic_and_indefinite_duration_qualify() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = emberon_copy(difficulty);
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, PURGING, Some(boss), false)
            .expect("ordinary native368990");
        h.advance(6.0);
        assert!(h.has_aura(boss, PURGING));
        let first = damage_count(&h, boss, PURGING_DAMAGE);
        assert!(
            first > 0,
            "native periodic child369025 difficulty{difficulty}"
        );
        h.advance(20.0);
        assert!(
            h.has_aura(boss, PURGING),
            "native DurationIndex21 is indefinite, not24.3s"
        );
        assert!(damage_count(&h, boss, PURGING_DAMAGE) > first);
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
    }
}

#[test]
#[ignore = "encounter_coverage Emberon first sourced request RED before combat implementation"]
fn encounter_coverage_emberon_first_clap_uses_current_lua_clock() {
    let (mut h, boss, _) = emberon_copy(205);
    h.engage(boss);
    h.advance(4.2);
    assert_eq!(cast_count(&h, boss, CLAP), 0, "no early Clap");
    h.advance(2.6);
    assert_eq!(
        cast_count(&h, boss, CLAP),
        1,
        "Lua4.5s request + native2s cast"
    );
    assert!(damage_count(&h, boss, CLAP) > 0);
}
