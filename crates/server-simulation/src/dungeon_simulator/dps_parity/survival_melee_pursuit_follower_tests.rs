use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires frozen native Survival profile and selected traits"]
fn survival_follower_keeps_melee_attacking_through_the_fight() {
    assert_sustained_melee(1);
}

#[test]
#[ignore = "requires frozen native Survival profile and selected traits"]
fn survival_follower_keeps_melee_and_cleave_with_three_targets() {
    assert_sustained_melee(3);
}

fn assert_sustained_melee(targets: usize) {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(255, targets).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let mut seen = 0;
    let mut late_swings = 0;
    let mut late_cleave_hits = 0;
    let mut target_at_200 = false;
    advance_combat_observed(&mut run, |run| {
        let world = run.app.world();
        let now = world.resource::<Time>().elapsed_secs_f64();
        if (200.0..200.1).contains(&now) {
            target_at_200 = world
                .get::<crate::combat::AutoAttack>(actor)
                .is_some_and(|attack| attack.victim == target);
        }
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if now > 200.0 {
            late_swings += events[seen..]
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.kind == CombatLogKind::Damage
                        && event.spell_id.is_none()
                        && event.amount > 0
                })
                .count();
            late_cleave_hits += events[seen..]
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.kind == CombatLogKind::Damage
                        && event.spell_id == Some(1262293)
                        && event.amount > 0
                })
                .count();
        }
        seen = events.len();
    })
    .unwrap();
    assert!(
        target_at_200,
        "follower must retain its melee target after 200 seconds"
    );
    assert!(
        late_swings > 30,
        "follower must keep swinging after 200 seconds: {late_swings}"
    );
    if targets == 3 {
        assert!(
            late_cleave_hits > 30,
            "cleave must keep hitting the pack: {late_cleave_hits}"
        );
    }
}
