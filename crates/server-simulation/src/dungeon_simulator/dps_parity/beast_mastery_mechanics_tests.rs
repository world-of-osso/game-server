use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn beast_mastery_barbed_and_cobra_arm_consumable_natures_ally() {
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    for producer in [217200, 193455] {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            producer,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary native producer");
        preparation::finish_pending_setup(&mut run).unwrap();
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, 1276720),
            "successful native Barbed/Cobra must arm selected Nature's Ally: {producer}"
        );
        if producer == 217200 {
            assert!(
                crate::auras::has_aura(run.app.world(), run.actor, 246152),
                "Nature's Ally must not replace the existing native Barbed Focus hook"
            );
        }
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            34026,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary Kill Command");
        preparation::finish_pending_setup(&mut run).unwrap();
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, 1276720),
            "Kill Command consumes the actual ready buff"
        );
        for _ in 0..200 {
            run.app.update();
        }
    }
    let combat = report::combat(run.app.world(), &run.damage_targets, 20.0);
    assert!(
        combat["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["spell_id"] == 83381
                && ["hits", "misses", "dodges", "parries"]
                    .iter()
                    .map(|key| action[key].as_u64().unwrap())
                    .sum::<u64>()
                    == 2),
        "both admitted Kill Commands must attempt real pet attacks, including native avoidance: {combat}"
    );
}
