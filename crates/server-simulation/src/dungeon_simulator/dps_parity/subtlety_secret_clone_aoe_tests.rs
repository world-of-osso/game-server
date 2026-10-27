//! Pinned SimC bugs=true, sc_rogue.cpp5333–5352: the Ancient Arts
//! Secret Technique shadow clone does not reduce secondary targets.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_secret_clone_aoe_secondary_receives_full_native_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(261, 3).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 0.0;
    for id in [196911, 1269163] {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: id,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    assert!(crate::auras::set_self_cast_stacks(
        run.app.world_mut(),
        run.actor,
        196911,
        7
    ));
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        for kind in [PowerType::Energy, PowerType::ComboPoints] {
            let pool = pools.get_mut(kind).unwrap();
            pool.current = pool.max;
        }
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(280719, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hits: Vec<_> = run
        .targets
        .iter()
        .map(|target| {
            events
                .iter()
                .find(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(1269560)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .expect("real Secret Technique shadow clone hits target")
        })
        .collect();
    let baseline = hits[0].amount as f32 / if hits[0].crit { 2.0 } else { 1.0 };
    for hit in &hits[1..] {
        let amount = hit.amount as f32 / if hit.crit { 2.0 } else { 1.0 };
        assert!(
            (amount / baseline - 1.0).abs() < 0.1,
            "native secondary clone hit must equal primary before crit: primary={baseline}, secondary={amount}"
        );
    }
}
