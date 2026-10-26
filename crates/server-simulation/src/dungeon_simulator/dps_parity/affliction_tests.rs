//! Native Affliction literal fixture qualification using the pinned MID2 profile.
use super::*;

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_literal_fixture_executes_profile_instead_of_assisted_kit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some(),
        "265 must capture literal control before native setup"
    );
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.targets, 20.0);
    let actions = combat["actions"].as_array().unwrap();
    assert!(
        actions
            .iter()
            .any(|row| row["spell_id"] == 980 && row["damage"].as_u64().unwrap() > 0),
        "literal Agony must damage the actual recipient"
    );
    assert!(
        !actions.iter().any(|row| row["spell_id"] == 702),
        "Curse of Weakness is absent from the pinned APL"
    );
}
