//! Native Devourer literal controller qualification.
use super::*;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_native_measurement_selects_literal_driver() {
    let run = prepare_measurement(1480, 1).expect("native Devourer profile and precombat");
    assert_eq!(
        run.spec_driver.as_ref().map(|driver| driver.label()),
        Some("LITERAL_SIMC_DEVOURER_76f8118")
    );
}
