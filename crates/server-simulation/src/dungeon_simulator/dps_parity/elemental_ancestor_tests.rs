//! Actual native Ancestors, not owner-cast substitute damage.
use super::elemental_mechanics_tests::{cast, fixture, own_aura, settle};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Npc, UnitSummonedBy};
use shared::protocol::CombatLogKind;

fn ancestors(run: &mut Run) -> Vec<Entity> {
    run.app
        .world_mut()
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(run.app.world())
        .filter(|(_, npc, owner)| npc.template_id == 221177 && owner.0 == run.actor.to_bits())
        .map(|(entity, _, _)| entity)
        .collect()
}

fn stormkeeper(run: &mut Run) -> Entity {
    let before = ancestors(run);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        191634,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(run);
    let after = ancestors(run);
    *after
        .iter()
        .find(|actor| !before.contains(actor))
        .expect("selected Stormkeeper must spawn an actual native221177 Ancestor")
}

#[test]
#[ignore = "requires private262 native DB/profile with native445624 template closure"]
fn elemental_ancestor_stormkeeper_spawns_native_owned_actor_and_buff() {
    let mut run = fixture();
    let ancestor = stormkeeper(&mut run);
    assert_ne!(ancestor, run.actor);
    assert!(
        own_aura(&run, 447244).is_some(),
        "actual Ancestor supplies its owner buff"
    );
    let summon = run
        .app
        .world()
        .get::<crate::spell_cast::summon::SummonedCreature>(ancestor)
        .unwrap();
    assert_eq!(summon.owner, run.actor);
    assert_eq!(summon.spell_id, 445624);
    assert!(summon.expires_at.is_some());
}

#[test]
#[ignore = "requires private262 native DB/profile with native445624 template closure"]
fn elemental_ancestor_echo_is_owned_damage_and_not_an_overload_recursion() {
    let mut run = fixture();
    let ancestor = stormkeeper(&mut run);
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    cast(&mut run, 188196);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let hits: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(ancestor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(447419)
                && event.amount > 0
        })
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "one foreground Bolt commands one Ancestor Lava Burst, not an overload echo"
    );
    assert!(
        hits[0].crit,
        "pinned Ancestor Lava Burst is always critical"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile with native445624 template closure"]
fn elemental_ancestor_chain_echo_reaches_native_pack_and_expires() {
    let mut run = prepare_literal(262, 3).unwrap();
    engage(&mut run);
    let ancestor = stormkeeper(&mut run);
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    cast(&mut run, 188443);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let targets: std::collections::HashSet<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(ancestor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(447425)
                && event.amount > 0
        })
        .filter_map(|event| event.target)
        .collect();
    assert_eq!(
        targets.len(),
        3,
        "native Ancestor Chain Lightning must damage all three actual opponents"
    );
    let expires = run
        .app
        .world()
        .get::<crate::spell_cast::summon::SummonedCreature>(ancestor)
        .unwrap()
        .expires_at
        .unwrap();
    for _ in 0..600 {
        run.app.update();
        if crate::auras::aura_now(run.app.world()) > expires + 0.1 {
            break;
        }
    }
    assert!(
        run.app.world().get_entity(ancestor).is_err(),
        "native summon duration retires the actual Ancestor"
    );
}
