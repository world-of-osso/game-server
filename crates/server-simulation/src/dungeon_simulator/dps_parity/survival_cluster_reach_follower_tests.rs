use super::*;

#[test]
#[ignore = "requires frozen native Survival profile and selected traits"]
fn survival_follower_reaches_all_three_clustered_targets() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(255, 3).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let mut checked = false;
    advance_combat_observed(&mut run, |run| {
        let world = run.app.world();
        if checked || world.resource::<Time>().elapsed_secs_f64() < 33.0 {
            return;
        }
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(186270)
            .unwrap();
        assert_eq!(run.targets.len(), 3);
        for &target in &run.targets {
            assert_eq!(
                crate::spell_cast::validate_target(
                    world,
                    actor,
                    spell,
                    Some(target),
                    crate::spell_cast::Sight::Start,
                ),
                Ok(Some(target)),
                "the follower must reach every clustered enemy"
            );
        }
        checked = true;
    })
    .unwrap();
    assert!(checked, "the follower must complete its approach");
}
