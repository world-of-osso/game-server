//! SimC76f8118 sc_rogue.cpp5377–5385: Secret Technique's intrinsic
//! clone attacks receive the caster's native guardian multiplier.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

fn clone_to_player_ratio(with_passive: bool) -> f32 {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 0.0;
    if !with_passive {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 137035 && aura.caster == run.actor,
        );
        run.app.update();
    }
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
    for _ in 0..100 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage = |spell| {
        let hit = events
            .iter()
            .find(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .expect("real Secret Technique hit");
        hit.amount as f32 / if hit.crit { 2.0 } else { 1.0 }
    };
    damage(282449) / damage(280720)
}

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_secret_guardian_clones_receive_native_owner_passive() {
    let unbuffed = clone_to_player_ratio(false);
    let buffed = clone_to_player_ratio(true);
    assert!(
        buffed > unbuffed * 1.2 && buffed < unbuffed * 1.4,
        "native guardian passive must increase clone/player ratio: without={unbuffed}, with={buffed}"
    );
}
