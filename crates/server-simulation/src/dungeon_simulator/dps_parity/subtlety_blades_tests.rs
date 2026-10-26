//! SimC76f8118 sc_rogue.cpp2819–2853/5508–5520/8501–8508.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_blades_actual_backstab_produces_one_native_shadow_hit_only_during_buff() {
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    for active in [false, true] {
        if active {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: run.actor,
                    target: run.actor,
                    spell_id: 121471,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        {
            let mut pools = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
                .unwrap();
            let energy = pools.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
        }
        run.app
            .world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(53, run.targets[0], Some(run.targets[0])),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let hits = |spell| {
            events
                .iter()
                .filter(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(run.targets[0].to_bits())
                        && event.spell_id == Some(spell)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .count()
        };
        assert_eq!(hits(53), 1, "actual native Backstab must hit");
        assert_eq!(
            hits(279043),
            usize::from(active),
            "earned Shadow Blades emits one nonrecursive native hit; events: {events:?}"
        );
    }
}
