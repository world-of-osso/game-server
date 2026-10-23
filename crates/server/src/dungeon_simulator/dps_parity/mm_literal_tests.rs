//! Native MM fixture must run the pinned literal APL, not AssistedCombat rules.
use super::*;
use crate::followers::simulator::ParityActor;

#[test]
#[ignore = "native MM literal-driver RED/GREEN; requires prepared parity inputs"]
fn dps_parity_mm_literal_native_player_advances_without_assisted_rule_gaps() {
    let _seed = crate::random::seed_scope(70448);
    let (profile, traits) = read_reference(254).unwrap();
    assert_eq!(profile.race, "dwarf");
    let mut run = prepare_reference(profile, &traits, 1, ParityActor::Player { race: 3 })
        .expect("authentic native MM player fixture");
    engage(&mut run);
    let result = advance_combat(&mut run);
    assert!(
        result.is_ok(),
        "MM literal APL must replace the AssistedCombat boundary: {result:?}"
    );
    let (seconds, _, _) = result.unwrap();
    assert!((seconds - 300.0).abs() < 0.001);
    let combat = report::combat(run.app.world(), &run.damage_targets, seconds);
    assert!(
        combat["total_damage"].as_u64().unwrap() > 0,
        "literal MM must execute real native damage, not merely advance the clock"
    );
}
