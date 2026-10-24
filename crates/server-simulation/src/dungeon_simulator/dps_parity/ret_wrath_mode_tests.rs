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

fn explicit_native_mode_fixture(parent: u32) -> Run {
    let mut run = selected_fixture();
    if parent == WRATH {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            parent,
            Some(run.actor),
            false,
        )
        .unwrap();
    } else {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            parent,
            Some(run.actor),
            0,
        );
    }
    // Isolate cleanup of a real permanent mode independently of its missing
    // producer; no timer, amount or aura field is overwritten.
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, MODE, Some(run.actor), 0);
    for _ in 0..4 {
        run.app.update();
    }
    assert!(has(&run, parent));
    assert!(has(&run, MODE));
    let mode = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == MODE)
        .unwrap();
    assert!(
        mode.expires_at.is_none(),
        "native mode is permanent, not retimed to a guessed duration"
    );
    run
}

#[test]
#[ignore = "actual ordinary Wrath cancellation must remove its caster-owned permanent mode"]
fn dps_parity_ret_wrath_mode_native_parent_cancellation_removes_owned_mode() {
    let mut run = explicit_native_mode_fixture(WRATH);
    let actor = run.actor;
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == WRATH && aura.caster == actor,
    );
    for _ in 0..4 {
        run.app.update();
    }
    assert!(!has(&run, WRATH));
    assert!(
        !has(&run, MODE),
        "real parent cancellation must not leave permanent Hammer mode"
    );
}

#[test]
#[ignore = "actual background Wrath expiry must remove its caster-owned permanent mode"]
fn dps_parity_ret_wrath_mode_native_background_expiry_removes_owned_mode() {
    let mut run = explicit_native_mode_fixture(BACKGROUND_WRATH);
    for _ in 0..300 {
        run.app.update();
        if !has(&run, BACKGROUND_WRATH) {
            break;
        }
    }
    assert!(
        !has(&run, BACKGROUND_WRATH),
        "actual native8s parent must expire under ordinary updates"
    );
    assert!(
        !has(&run, MODE),
        "actual parent expiry must not leave permanent Hammer mode"
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
