//! Real Deep Breath, native summon identities and actual pet-owned Pyre impacts.
use super::*;
use crate::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

#[test]
#[ignore = "requires exact Devastation native profile and authentic class-summon templates"]
fn evoker_command_squadron_actual_maneuverability_owns_two_pets_and_three_flight_pyres() {
    for selected in [false, true] {
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        let primary = run.targets[0];
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 1260745));
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 1260745,
            );
        }
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
        // SimC76f8118 sc_evoker.cpp3795–3839/6256–6261: two actual guardians,
        // uninterrupted flight executes tick0 plus scheduled ticks1–3.
        assert_eq!(
            pets.len(),
            if selected { 2 } else { 0 },
            "actual selected Maneuverability must summon native Dracthyr Commandos"
        );
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..20 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for pet in &pets {
            for target in &run.targets {
                let hits = events
                    .iter()
                    .filter(|event| {
                        event.source == Some(pet.to_bits())
                            && event.target == Some(target.to_bits())
                            && event.spell_id == Some(1236970)
                            && event.kind == shared::protocol::CombatLogKind::Damage
                            && event.amount > 0
                    })
                    .count();
                assert_eq!(
                    hits, 4,
                    "each actual Commando must damage each native pack recipient four times on uninterrupted flight"
                );
            }
            assert!(
                run.app.world().get::<SummonedCreature>(*pet).is_none(),
                "actual command flight end must despawn its owned Commando"
            );
        }
    }
}
