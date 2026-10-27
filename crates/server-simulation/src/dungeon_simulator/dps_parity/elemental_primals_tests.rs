//! SimC76f8118 sc_shaman.cpp4070–4139/4248–4315/8638–8640/8752–8755.
use super::elemental_mechanics_tests::{fixture, own_aura, settle};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::{
    components::{Npc, UnitSummonedBy},
    protocol::CombatLogKind,
};

fn assert_native_elemental(root: u32, talent: u32, entry: u32, attacks: &[u32]) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(
        own_aura(&run, talent).is_some(),
        "selected native elemental-summon talent"
    );
    assert!(
        own_aura(&run, 117013).is_some(),
        "selected native Primal Elementalist"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, root, Some(run.actor), false)
        .unwrap();
    settle(&mut run);
    let elemental = run
        .app
        .world_mut()
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(run.app.world())
        .find(|(_, npc, owner)| npc.template_id == entry && owner.0 == run.actor.to_bits())
        .map(|(entity, _, _)| entity)
        .expect("ordinary native cooldown summons its actual owned Primal Elemental");
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(elemental.to_bits())
                && event.spell_id.is_some_and(|spell| attacks.contains(&spell))
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "actual elemental deals native owned damage"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_primal_fire_ascendance_summons_and_attacks() {
    assert_native_elemental(1219480, 378255, 61029, &[117588, 57984, 118297]);
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_primal_storm_stormkeeper_summons_and_attacks() {
    assert_native_elemental(191634, 191717, 77942, &[157331, 157348, 269005]);
}
