//! SimC76f8118 sc_shaman.cpp3316–3351/7897–7915/7953–7968: native Earthquake overload field.
use super::elemental_mechanics_tests::{cast, own_aura, settle};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_quake_overload_ascendance_creates_native_damage_field() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(262, 3).unwrap();
    engage(&mut run);
    assert!(
        own_aura(&run, 381726).is_some(),
        "selected native Mountains Will Fall"
    );
    for _ in 0..64 {
        let (current, maximum) = crate::spell_cast::powers::power_value(
            run.app.world(),
            run.actor,
            PowerType::Maelstrom,
        )
        .unwrap();
        if current >= maximum {
            break;
        }
        cast(&mut run, 188196);
    }
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        1219480,
        Some(run.actor),
        false,
    )
    .unwrap();
    settle(&mut run);
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    cast(&mut run, 462620);
    for _ in 0..160 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let hits = |spell| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count()
    };
    assert!(hits(77478) >= 3, "ordinary native Earthquake hits the pack");
    assert!(
        hits(298765) >= 3,
        "Ascendance guarantees an actual native Earthquake overload field on the pack"
    );
}
