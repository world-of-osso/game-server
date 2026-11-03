//! Fixed-fight companions have engaged owner targets, not a dungeon pull plan.
use super::*;

#[test]
#[ignore = "requires MID2 hunter and demonology profiles and native world data"]
fn fixed_fight_pets_cast_on_the_owners_engaged_targets() {
    for spec in [253, 266] {
        for count in [1, 3] {
            let _seed = crate::random::seed_scope(70448);
            let mut run = prepare_measurement(spec, count).unwrap();
            let owner = fixture_player(&run);
            assert!(
                run.app
                    .world()
                    .get::<crate::followers::AdmittedPull>(owner)
                    .is_none()
            );
            engage(&mut run);
            let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
                .expect("prepared follower pet");
            for &target in &run.targets {
                assert!(
                    crate::followers::action_safe(run.app.world(), pet, target, None),
                    "spec{spec} must admit every engaged fixed-fight target"
                );
            }
            let outsider = spawn_level_dummy(&mut run.app, run.actor, 90, 5).unwrap();
            assert!(
                !crate::followers::action_safe(run.app.world(), pet, outsider, None),
                "spec{spec} must not admit an unengaged target"
            );
            let mut driver = run.spec_driver.take().unwrap();
            for _ in 0..1200 {
                driver
                    .drive_frame(run.app.world_mut(), run.actor, &run.targets)
                    .unwrap();
                run.app.update();
                let pet_report = report::pet(run.app.world(), run.actor, &run.targets);
                if !pet_report["casts"].as_array().unwrap().is_empty()
                    && pet_report["damage"].as_i64().unwrap() > 0
                {
                    break;
                }
            }
            let pet_report = report::pet(run.app.world(), run.actor, &run.targets);
            assert!(
                !pet_report["casts"].as_array().unwrap().is_empty(),
                "spec{spec} {count}T: {pet_report}"
            );
            assert!(
                pet_report["damage"].as_i64().unwrap() > 0,
                "spec{spec} {count}T: {pet_report}"
            );
        }
    }
}
