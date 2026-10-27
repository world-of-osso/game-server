//! Native fragment heal impact and Devourer stack production.
use super::*;
use server::soul_fragments::{self, FragmentFilter, FragmentKind};
use shared::{components::Health, protocol::CombatLogKind};

fn stacks(world: &World, actor: Entity, spell: u32) -> u8 {
    world
        .get::<crate::auras::ActiveAuras>(actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == actor)
                .map(|aura| aura.stacks)
        })
        .unwrap_or(0)
}

fn consume_fragment_and_observe_heal(run: &mut Run) {
    let actor = run.actor;
    {
        let mut health = run
            .app
            .world_mut()
            .get_mut::<Health>(actor)
            .expect("native health");
        health.current = health.max * 0.5;
    }
    soul_fragments::spawn(run.app.world_mut(), actor, FragmentKind::Lesser, false)
        .expect("native lesser fragment");
    for _ in 0..40 {
        if soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser) > 0 {
            break;
        }
        run.app.update();
    }
    assert!(
        soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser) > 0,
        "native fragment must become active"
    );
    assert_eq!(
        soul_fragments::consume_active(
            run.app.world_mut(),
            actor,
            FragmentFilter::Lesser,
            1,
            false
        )
        .expect("consume native fragment"),
        1
    );
    for _ in 0..200 {
        run.app.update();
        if has_fragment_heal(run) {
            break;
        }
    }
    assert!(
        has_fragment_heal(run),
        "native fragment heal must land before checking its side effect"
    );
}

fn has_fragment_heal(run: &Run) -> bool {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1266301)
                && event.kind == CombatLogKind::Heal
                && event.amount > 0
        })
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_fragment_heal_increments_meta_stacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1217607),
        "out-of-Meta fixture"
    );
    let initial = stacks(run.app.world(), actor, 1225789);
    consume_fragment_and_observe_heal(&mut run);
    // SimC76f8118 sc_demon_hunter.cpp3678–3692 triggers the stack at heal impact.
    assert_eq!(
        stacks(run.app.world(), actor, 1225789),
        initial + 1,
        "one consumed fragment must add one Meta stack"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_collapsing_star_threshold_grants_ready_buff() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1221167)),
        "selected Collapsing Star talent"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "native Meta active"
    );
    let threshold =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 1221167, 0) as u8;
    assert_eq!(threshold, 30, "native selected talent threshold");
    for _ in 0..threshold - 1 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1227702, Some(actor), 1);
    }
    for _ in 0..20 {
        if stacks(run.app.world(), actor, 1227702) == threshold - 1 {
            break;
        }
        run.app.update();
    }
    assert_eq!(
        stacks(run.app.world(), actor, 1227702),
        threshold - 1,
        "native stacks prepared below threshold"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1221171),
        "ready buff absent below threshold"
    );
    consume_fragment_and_observe_heal(&mut run);
    assert_eq!(
        stacks(run.app.world(), actor, 1227702),
        threshold,
        "real fragment heal crosses threshold"
    );
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1221171) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1221171),
        "crossing threshold grants ready buff"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_meta_fragment_heal_increments_collapsing_star() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1221167)),
        "selected native Collapsing Star talent"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "native Meta must be active"
    );
    let initial_star = stacks(run.app.world(), actor, 1227702);
    let initial_meta = stacks(run.app.world(), actor, 1225789);
    consume_fragment_and_observe_heal(&mut run);
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "Meta must remain active through the heal"
    );
    // SimC76f8118 sc_demon_hunter.cpp3691–3698 uses the selected Collapsing Star stack in Meta.
    assert_eq!(
        stacks(run.app.world(), actor, 1227702),
        initial_star + 1,
        "one consumed fragment must add one Collapsing Star stack"
    );
    assert_eq!(
        stacks(run.app.world(), actor, 1225789),
        initial_meta,
        "Meta fragments must not add out-of-Meta stacks"
    );
}
