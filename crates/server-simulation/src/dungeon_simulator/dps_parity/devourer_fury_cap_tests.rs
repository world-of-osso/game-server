//! Selected-profile Fury caps with native Untethered Fury.
use super::*;
use shared::components::PowerType;

fn assert_fury_cap(spec: u32, expected: f32) {
    let mut run = prepare(spec, 1).expect("native Demon Hunter profile");
    for _ in 0..5 {
        run.app.update();
    }
    let (_, max) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::Fury)
            .expect("native Fury pool");
    assert_eq!(max, expected, "selected profile Fury cap for spec {spec}");
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_selected_profile_fury_cap() {
    assert_fury_cap(1480, 140.0);
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec577 exact profile/traits"]
fn havoc_selected_profile_fury_cap() {
    assert_fury_cap(577, 170.0);
}
