//! An actual passive Commando's Pyre inherits its owner's live critical chance.
use super::*;
use crate::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

#[test]
#[ignore = "requires native Devastation profile and canonical-source Commando templates"]
fn evoker_commando_crit_actual_flight_pyre_reads_live_owner_critical_chance() {
    let mut run = prepare_literal(1467, 1).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1260745));
    // Fix the real owner's final sheet at100% to make each actual pet hit deterministic.
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 100.0;
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        run.actor,
        433874,
        crate::spell_cast::CastTargets {
            unit: run.actor,
            primary: Some(primary),
            destination: None,
        },
    )
    .unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    let owner = run.actor;
    let pets: Vec<_> = run
        .app
        .world_mut()
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(run.app.world())
        .filter(|(_, npc, summon)| npc.template_id == 219827 && summon.owner == owner)
        .map(|(entity, _, _)| entity)
        .collect();
    assert_eq!(
        pets.len(),
        2,
        "actual Command Squadron must produce two real owned actors"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for pet in pets {
        let hits: Vec<_> = events
            .iter()
            .filter(|event| {
                event.source == Some(pet.to_bits())
                    && event.target == Some(primary.to_bits())
                    && event.spell_id == Some(1236970)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.amount > 0
            })
            .collect();
        assert!(
            !hits.is_empty(),
            "actual pet flight must produce its native Pyre"
        );
        // SimC76f8118 pet.cpp217–220: live owner spell crit applies to guardian casts.
        assert!(
            hits.iter().all(|event| event.crit),
            "each actual Commando Pyre must crit at its real owner's100% chance: {hits:?}"
        );
    }
}
