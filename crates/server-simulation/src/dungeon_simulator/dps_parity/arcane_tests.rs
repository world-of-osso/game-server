//! Literal Arcane profile control must replace the native follower priority.
use super::*;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec62 profile/traits"]
fn arcane_literal_fixture_executes_profile_cooldowns_and_filler() {
    let result = measure(62, 1).expect("native Arcane profile fixture");
    assert_eq!(result["driver"], "LITERAL_SIMC_ARCANE_76f8118");
    assert_eq!(result["assisted_decisions"], 0);
    assert!(
        result["arcane_rotation"]["snapshot_stats"]["native"]["spell_power"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    let actions = result["combat"]["actions"].as_array().unwrap();
    for spell in [30451, 365350, 321507] {
        assert!(
            actions
                .iter()
                .any(|action| action["spell_id"] == spell
                    && action["count"].as_u64().unwrap_or(0) > 0),
            "literal profile must execute native spell{spell}: {actions:?}"
        );
    }
}
