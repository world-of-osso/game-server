//! Native Frost reactions must observe passive acquisition during food setup.
use super::*;

#[test]
#[ignore = "requires private native70448 Frost player profile"]
fn frost_mage_native_seed7_tracks_early_icicle_through_full_fight() {
    for targets in [1, 3] {
        let _seed = crate::random::seed_scope(7);
        let mut run = prepare_measurement(64, targets)
            .expect("seed7 native Frost passive acquisition must not precede reaction policy");
        engage(&mut run);
        let (seconds, _, _) = advance_combat(&mut run).expect("native Frost combat");
        assert!(seconds >= 299.9, "full seed7 native {targets}T fight");
    }
}
