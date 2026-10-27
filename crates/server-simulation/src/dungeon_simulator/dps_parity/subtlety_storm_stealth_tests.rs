//! SimC76f8118 sc_rogue.cpp5872–5881: Shuriken Storm's native
//! rank-2 damage applies while in the Shadow Dance stance.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

fn storm_damage(dance: bool) -> i32 {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(261, 3).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 0.0;
    if dance {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 185313,
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
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(197835, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(197835)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .expect("real primary Storm damage")
        .amount
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_storm_stealth_real_dance_applies_native_rank_two_bonus() {
    let ordinary = storm_damage(false);
    let danced = storm_damage(true);
    assert!(
        (danced as f32) >= (ordinary as f32) * 1.8,
        "native 100% stance bonus must apply: ordinary={ordinary}, dance={danced}"
    );
}
