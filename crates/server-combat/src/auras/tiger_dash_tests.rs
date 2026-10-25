//! TC a352b1fa spell_druid.cpp:2827-2843 spell_dru_tiger_dash_aura: each effect1
//! PERIODIC_DUMMY tick lowers effect0's run-speed amount by effect1's amount.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const TIGER_DASH: u32 = 252_216;
const DRUID: u8 = 11;

fn native_points(index: u8) -> f32 {
    world_db()
        .unwrap()
        .store
        .get(TIGER_DASH)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .base_points
}

fn speed(h: &Harness) -> f32 {
    h.app
        .world()
        .get::<AuraStatMods>(h.caster)
        .unwrap()
        .movement_speed
}

#[test]
fn class_completeness_native_tiger_dash_decays_each_periodic_tick() {
    let mut h = harness(Vec::new(), &[TIGER_DASH], DRUID).unwrap();
    let druid = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: druid,
            target: druid,
            spell_id: TIGER_DASH,
            effect_mask: 0b11,
            source: AuraSource::Cast,
        },
    );
    h.advance(1);
    let (bonus, decay) = (native_points(0), native_points(1));
    assert_eq!(speed(&h), 1.0 + bonus / 100.0);
    // 500 ms ticks at 50 ms frames: sample between ticks.
    h.advance(11);
    assert!(
        (speed(&h) - (1.0 + (bonus - decay) / 100.0)).abs() < 1e-4,
        "{}",
        speed(&h)
    );
    h.advance(30);
    assert!(
        (speed(&h) - (1.0 + (bonus - 4.0 * decay) / 100.0)).abs() < 1e-4,
        "{}",
        speed(&h)
    );
    h.advance(100);
    assert!(!has_aura(h.app.world(), druid, TIGER_DASH));
    assert_eq!(speed(&h), 1.0);
}
