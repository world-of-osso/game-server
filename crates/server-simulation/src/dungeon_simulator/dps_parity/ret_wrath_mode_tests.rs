//! Native1241288 description plus SimC76f8118 paladin.cpp314–315/3634–3641.
use super::ret_tier_tests::{has, native_ret};
use super::*;

const TALENT: u32 = 1241288;
const MODE: u32 = 1277026;
const WRATH: u32 = 31884;
const BACKGROUND_WRATH: u32 = 454351;

fn selected_fixture() -> Run {
    let run = native_ret(4);
    let traits = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap();
    assert!(
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == TALENT),
        "validated active Hammer talent includes native grants"
    );
    assert!(!has(&run, MODE));
    assert!(!has(&run, WRATH));
    assert!(!has(&run, BACKGROUND_WRATH));
    run
}

#[test]
#[ignore = "actual ordinary Wrath with validated Hammer talent must produce native mode"]
fn dps_parity_ret_wrath_mode_native_ordinary_wrath_grants_real_mode() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = selected_fixture();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        WRATH,
        Some(run.actor),
        false,
    )
    .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        has(&run, WRATH),
        "ordinary native Wrath must actually apply"
    );
    assert!(
        has(&run, MODE),
        "selected1241288 must produce actual1277026 during Wrath"
    );
}

#[test]
#[ignore = "actual background454351 follows the same selected native Hammer-mode relationship"]
fn dps_parity_ret_wrath_mode_native_background_wrath_grants_real_mode() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = selected_fixture();
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        BACKGROUND_WRATH,
        Some(run.actor),
        0,
    );
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        has(&run, BACKGROUND_WRATH),
        "actual native background Wrath must apply"
    );
    assert!(
        has(&run, MODE),
        "selected1241288 must produce actual1277026 during background Wrath"
    );
}

#[test]
#[ignore = "native selected Hammer talent alone must not manufacture permanent mode"]
fn dps_parity_ret_wrath_mode_native_without_wrath_keeps_mode_absent() {
    let mut run = selected_fixture();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(!has(&run, MODE));
}
