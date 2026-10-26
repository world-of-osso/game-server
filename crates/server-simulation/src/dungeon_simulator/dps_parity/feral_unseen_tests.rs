//! Actual paid Bite snapshots must survive into delayed native Unseen damage.
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_paid_bites_produce_delayed_unseen_slash_and_native_bleed() {
    let _seed = crate::random::seed_scope(1031263657);
    let mut run = prepare_cat();
    for _ in 0..10 {
        cast_full_energy_bite(&mut run);
        assert!(
            power(&run, PowerType::ComboPoints) < 5.0,
            "real Bite pays its earned points; native follow-up procs may already earn new points"
        );
        for _ in 0..120 {
            run.app.update();
        }
    }
    let report = report::combat(run.app.world(), &run.targets, 1.0);
    let damage = |spell| {
        report["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["spell_id"] == spell)
            .map(|action| action["damage"].as_u64().unwrap())
            .sum::<u64>()
    };
    assert!(
        damage(1263890) > 0,
        "paid native Bite roots must earn actual delayed Unseen Slash"
    );
    assert!(
        damage(1271863) > 0,
        "the delayed native Slash must apply its actual bleed"
    );
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_native_unseen_bleed_pays_one_damage_event_per_scheduled_tick() {
    let _seed = crate::random::seed_scope(1031263657);
    let mut run = prepare_cat();
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..10 {
        cast_full_energy_bite(&mut run);
        if crate::auras::has_aura(run.app.world(), target, 1271863) {
            break;
        }
    }
    assert!(
        crate::auras::has_aura(run.app.world(), target, 1271863),
        "actual paid Bite earned the native delayed bleed"
    );
    run.app
        .world_mut()
        .entity_mut(actor)
        .remove::<crate::combat::AutoAttack>();
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    let count = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.spell_id == Some(1271863)
                    && event.periodic
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.target == Some(target.to_bits())
            })
            .count()
    };
    for _ in 0..200 {
        run.app.update();
        if count(&run) > 0 {
            break;
        }
    }
    assert_eq!(
        count(&run),
        1,
        "one actual Unseen dot tick must not deal two separate native damage events"
    );
}

fn prepare_cat() -> Run {
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), actor, 1263657));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    run
}

fn power(run: &Run, kind: PowerType) -> f32 {
    crate::spell_cast::powers::power_value(run.app.world(), run.actor, kind)
        .unwrap()
        .0
}

fn cast_full_energy_bite(run: &mut Run) {
    while power(run, PowerType::ComboPoints) < 5.0 {
        feral_growth_tests::cast_native_spell_after_regeneration(run, 5221);
    }
    let maximum =
        crate::spell_cast::read_ferocious_bite_maximum_energy(run.app.world(), run.actor).unwrap();
    while power(run, PowerType::Energy) < maximum {
        run.app.update();
    }
    feral_growth_tests::cast_native_spell_after_regeneration(run, 22568);
}
