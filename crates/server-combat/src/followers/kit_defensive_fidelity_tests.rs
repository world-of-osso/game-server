//! Native four-spec defensive choices under the existing project health/victim policy.
use super::kit_tests::at_level;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::Health;

fn spawn_wounded_follower(entry: u32, attacked: bool) -> (Harness, Entity) {
    let (mut h, unit) = at_level(entry, 90, |_| {});
    let victim = if attacked { unit } else { h.caster };
    h.app.world_mut().entity_mut(h.npc).insert((
        crate::networking::CombatTarget(Some(victim)),
        crate::combat::AutoAttack::new(victim),
        // A durable hostile target keeps both controls engaged for the observation.
        Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        },
    ));
    let mut health = h.app.world_mut().get_mut::<Health>(unit).unwrap();
    // Below the existing defensive50% policy, above emergency self-heal40%.
    health.current = health.max * 0.45;
    (h, unit)
}

fn assert_native_defense(entry: u32, spell: u32) {
    for attacked in [false, true] {
        let (mut h, unit) = spawn_wounded_follower(entry, attacked);
        for _ in 0..20 {
            super::update_ai(h.app.world_mut());
            h.advance(1);
        }
        let cast = h
            .inbox
            .go
            .iter()
            .any(|event| event.caster == unit.to_bits() && event.spell_id == spell);
        assert_eq!(
            cast, attacked,
            "native defensive{spell} must react to attacks on this wounded follower, not just its owner"
        );
        assert_eq!(
            crate::auras::has_aura(h.app.world(), unit, spell),
            attacked,
            "selected defensive must apply its real native aura, not only emit a cast receipt"
        );
    }
}

#[test]
fn native_utility_fire_uses_blazing_barrier_only_when_wounded_and_attacked() {
    assert_native_defense(209059, 235313);
}

#[test]
fn native_utility_elemental_uses_astral_shift_only_when_wounded_and_attacked() {
    assert_native_defense(209053, 108271);
}

#[test]
fn native_utility_beast_mastery_uses_turtle_only_when_wounded_and_attacked() {
    assert_native_defense(209065, 186265);
}

#[test]
fn native_utility_devastation_uses_scales_only_when_wounded_and_attacked() {
    assert_native_defense(215240, 363916);
}
