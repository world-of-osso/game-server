//! Actual paid Bite snapshots must survive into delayed native Unseen damage.
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_paid_bites_produce_delayed_unseen_slash_and_native_bleed() {
    let _seed = crate::random::seed_scope(1031263657);
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), actor, 1263657));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let power = |run: &Run, kind| {
        crate::spell_cast::powers::power_value(run.app.world(), actor, kind)
            .unwrap()
            .0
    };
    for _ in 0..10 {
        while power(&run, PowerType::ComboPoints) < 5.0 {
            feral_growth_tests::cast_native_spell_after_regeneration(&mut run, 5221);
        }
        let maximum =
            crate::spell_cast::read_ferocious_bite_maximum_energy(run.app.world(), actor).unwrap();
        while power(&run, PowerType::Energy) < maximum {
            run.app.update();
        }
        feral_growth_tests::cast_native_spell_after_regeneration(&mut run, 22568);
        assert_eq!(
            power(&run, PowerType::ComboPoints),
            0.0,
            "real Bite pays its earned points"
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
