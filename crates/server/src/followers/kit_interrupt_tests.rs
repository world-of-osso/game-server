//! Joint planner/picker acceptance: tests the existing production AI boundary.
//! Planner/caller changes belong to9468; kit readiness/casts belong here.
use super::kit_tests::level32;
use super::kit_utility_tests::wait_for_native_school_unlock;
use super::{update_ai, utility_tests};
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

#[test]
fn follower_kit_interrupt_does_not_spend_on_noninterruptible_cast() {
    let (mut h, unit) = level32(209059);
    utility_tests::start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    // Protection allows ordinary control selection; provide the bare harness NPC's
    // actual CreatureType instead of inventing or disabling the follow-up control.
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[6]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    utility_tests::protect_enemy_cast_with_native_interrupt_immunity(&mut h, enemy);
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(h.app.world().get::<CastState>(h.npc).is_some());
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 2139)
    );
}

#[test]
#[ignore = "RED: kit spends a buff GCD before the ready shield fallback"]
fn follower_kit_interrupt_rebuke_cooldown_uses_ready_shield_before_buffs() {
    let (mut h, unit) = level32(209057);
    utility_tests::start_enemy_fireball(&mut h, unit);
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt && event.spell_id == Some(96231))
    );
    wait_for_native_school_unlock(&mut h, 96231);
    utility_tests::start_enemy_fireball(&mut h, unit);
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt
                && event.source == Some(unit.to_bits())
                && event.spell_id == Some(31935))
    );
}

#[test]
#[ignore = "RED: selected noncaster hides the second engaged caster from the kit"]
fn follower_kit_interrupt_uses_second_engaged_caster_without_chasing() {
    let (mut h, unit) = level32(209059);
    let caster = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 10.0);
    utility_tests::prepare_enemy_spec(&mut h, caster, 63);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::networking::CombatTarget(Some(unit)));
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 133, Some(unit), false).unwrap();
    let position = *h
        .app
        .world()
        .get::<shared::components::Position>(unit)
        .unwrap();
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt
                && event.source == Some(unit.to_bits())
                && event.target == Some(caster.to_bits())
                && event.spell_id == Some(2139))
    );
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::Position>(unit)
            .unwrap(),
        position
    );
}
