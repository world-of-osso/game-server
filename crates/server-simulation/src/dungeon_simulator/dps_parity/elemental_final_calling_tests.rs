//! SimC76f8118 sc_shaman.cpp4421–4433: a naturally expiring Ancestor executes Final Calling.
use super::elemental_mechanics_tests::{fixture, own_aura};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::{
    components::{Npc, UnitSummonedBy},
    protocol::CombatLogKind,
};

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_final_calling_natural_expiry_delivers_owned_native_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(
        own_aura(&run, 443446).is_some(),
        "selected native Final Calling"
    );
    crate::spell_cast::scripts::summon_ancestor(run.app.world_mut(), run.actor, 0);
    let ancestor = run
        .app
        .world_mut()
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(run.app.world())
        .find(|(_, npc, owner)| npc.template_id == 221177 && owner.0 == run.actor.to_bits())
        .map(|(entity, _, _)| entity)
        .expect("actual native Ancestor");
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..500 {
        run.app.update();
        if run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(ancestor.to_bits())
                    && event.spell_id == Some(465717)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            break;
        }
    }
    assert!(
        run.app.world().get_entity(ancestor).is_err(),
        "original Ancestor expired naturally"
    );
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(ancestor.to_bits())
                && event.spell_id == Some(465717)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "the expiring actual Ancestor delivers Final Calling damage"
    );
}
