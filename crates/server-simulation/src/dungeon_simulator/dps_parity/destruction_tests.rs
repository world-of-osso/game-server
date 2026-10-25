//! Native Destruction fixture control and actual damage, using the pinned profile.
use super::*;

#[test]
#[ignore = "requires private267 profile and native world DB"]
fn destruction_literal_fixture_executes_profile_instead_of_native_kit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(267, 1).unwrap();
    assert!(
        run.app.world().get::<literal_rotation::LiteralParityFixture>(run.actor).is_some(),
        "267 must capture literal control before native setup"
    );
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.targets, 20.0);
    let actions = combat["actions"].as_array().unwrap();
    assert!(actions.iter().any(|row| row["spell_id"] == 29722 && row["damage"].as_u64().unwrap() > 0),
        "literal Incinerate must damage the actual recipient");
    assert!(!actions.iter().any(|row| row["spell_id"] == 702),
        "Curse of Weakness is absent from the pinned APL");
}
