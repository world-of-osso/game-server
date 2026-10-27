//! SimC76f8118 sc_shaman.cpp4421–4433: a naturally expiring Ancestor executes Final Calling.
use super::elemental_mechanics_tests::{cast, fixture, own_aura};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::{
    components::{Npc, UnitSummonedBy},
    protocol::CombatLogKind,
};

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_dismissed_summon_drains_its_already_launched_missile() {
    let mut run = fixture();
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
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        ancestor,
        447419,
        Some(run.targets[0]),
        1
    ));
    crate::spell_cast::summon::despawn_summon(run.app.world_mut(), ancestor);
    assert!(
        run.app.world().get_entity(ancestor).is_ok(),
        "in-flight source retained"
    );
    assert!(
        !crate::spell_cast::targets::is_alive(run.app.world(), ancestor),
        "dismissed source cannot act or be targeted"
    );
    assert_ne!(
        run.app
            .world()
            .get::<shared::components::UnitFlags>(ancestor)
            .unwrap()
            .0
            & shared::components::UnitFlags::NOT_SELECTABLE,
        0
    );
    for _ in 0..20 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(ancestor.to_bits())
                && event.spell_id == Some(447419)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "launched missile retains its original source"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(ancestor.to_bits())
                && event.spell_id == Some(465717)
                && event.kind == CombatLogKind::Damage),
        "manual dismissal does not fire Final Calling"
    );
    assert!(
        run.app.world().get_entity(ancestor).is_err(),
        "source is released after the last hit"
    );
}

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
        .init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
    for _ in 0..50 {
        run.app.update();
    }
    cast(&mut run, 188196);
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
    for _ in 0..3 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
            .0
            .iter()
            .any(|(caster, spell, _)| *caster == ancestor && *spell == 465717),
        "actual expiry launched Final Calling"
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
    assert!(
        run.app.world().get_entity(ancestor).is_err(),
        "original Ancestor expired naturally"
    );
}
