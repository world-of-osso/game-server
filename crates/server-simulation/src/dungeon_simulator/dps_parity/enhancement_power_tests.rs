//! Actual MW spending earns Unlimited Power stacks without extending the current buff epoch.
use super::*;
use crate::auras::ActiveAuras;

fn power_state(run: &Run) -> Option<(u8, f64)> {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 454394 && aura.caster == run.actor)
        .map(|aura| {
            assert_eq!(
                aura.effects
                    .iter()
                    .find(|effect| effect.index == 0)
                    .unwrap()
                    .amount,
                1.0,
                "Unlimited Power retains native one-percent haste per stack"
            );
            (
                aura.stacks,
                aura.expires_at.expect("native Unlimited Power is finite"),
            )
        })
}

fn earn_and_spend(run: &mut Run) {
    let target = run.targets[0];
    for _ in 0..5000 {
        let spent = enhancement_generation_tests::try_spend_native_maelstrom(run, target);
        if spent > 0 {
            return;
        }
        enhancement_doom_deck_tests::try_cast_native_generator(run, target);
        run.app.update();
    }
    panic!("actual native generators did not fund a MW-spending cast");
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_unlimited_power_real_spending_stacks_without_refresh_and_restarts_after_expiry() {
    // SimC76f8118 sc_shaman.cpp11766–11770 grants once per spending execution;
    // 12655–12658 sets native454394 with refresh behavior DISABLED, not asynchronous stacks.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(454391)
    );
    engage(&mut run);
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
    assert!(
        power_state(&run).is_none(),
        "learning the talent does not fabricate an earned haste stack"
    );
    earn_and_spend(&mut run);
    let (first_stacks, deadline) =
        power_state(&run).expect("actual MW spending must earn native Unlimited Power");
    assert_eq!(
        first_stacks, 1,
        "one cast grants one stack, not one per consumed MW"
    );
    earn_and_spend(&mut run);
    let (second_stacks, second_deadline) =
        power_state(&run).expect("the next real spend must retain Unlimited Power");
    assert_eq!(second_stacks, 2);
    assert!(
        (second_deadline - deadline).abs() < 0.001,
        "stack gain must preserve the original epoch deadline: first={deadline}, second={second_deadline}"
    );
    for _ in 0..1200 {
        if crate::spell_cast::now(run.app.world()) > deadline + 0.1 {
            break;
        }
        run.app.update();
    }
    assert!(
        power_state(&run).is_none(),
        "all stacks expire at the original native deadline"
    );
    earn_and_spend(&mut run);
    let (fresh_stacks, fresh_deadline) = power_state(&run).unwrap();
    assert_eq!(fresh_stacks, 1);
    assert!(
        fresh_deadline > deadline,
        "a later real spend starts a new native buff epoch"
    );
}
