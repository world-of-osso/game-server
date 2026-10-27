//! Native fragment heal impact and out-of-Meta stack production.
use super::*;
use server::soul_fragments::{self, FragmentFilter, FragmentKind};
use shared::{components::Health, protocol::CombatLogKind};

fn meta_stacks(world: &World, actor: Entity) -> u8 {
    world.get::<crate::auras::ActiveAuras>(actor).and_then(|auras| {
        auras.iter().find(|aura| aura.spell_id == 1225789 && aura.caster == actor).map(|aura| aura.stacks)
    }).unwrap_or(0)
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_fragment_heal_increments_meta_stacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1217607), "out-of-Meta fixture");
    let initial = meta_stacks(run.app.world(), actor);
    {
        let mut health = run.app.world_mut().get_mut::<Health>(actor).expect("native health");
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
    assert!(soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser) > 0, "native fragment must become active");
    assert_eq!(soul_fragments::consume_active(run.app.world_mut(), actor, FragmentFilter::Lesser, 1, false).expect("consume native fragment"), 1);
    for _ in 0..200 {
        run.app.update();
        if run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter().any(|event| event.source == Some(actor.to_bits()) && event.spell_id == Some(1266301) && event.kind == CombatLogKind::Heal && event.amount > 0) {
            break;
        }
    }
    assert!(run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter().any(|event| event.source == Some(actor.to_bits()) && event.spell_id == Some(1266301) && event.kind == CombatLogKind::Heal && event.amount > 0), "native fragment heal must land before checking its side effect");
    // SimC76f8118 sc_demon_hunter.cpp3678–3692 triggers the stack at heal impact.
    assert_eq!(meta_stacks(run.app.world(), actor), initial + 1, "one consumed fragment must add one Meta stack");
}
