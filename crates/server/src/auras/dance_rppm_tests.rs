//! Actual learned Dance/native PPM57/mod221 and production-derived haste.
use super::proc_chance;
use crate::auras::{ActiveAuras, aura_now};

#[test]
fn rotation_fidelity_windwalker_dance_rppm_uses_native_regen_haste_modifier() {
    // Native Martial Agility1272685 is learned55 and supplies aura342/base30.
    // Level41 has genuinely zero haste; do not fabricate a stat to exercise scaling.
    let (mut h, unit) = crate::followers::kit_tests::at_level(227734, 55, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    h.advance(20);
    let world = h.app.world();
    assert!(
        crate::auras::has_aura(world, unit, 1272685),
        "actual learned55 Martial Agility"
    );
    let aura = world
        .get::<ActiveAuras>(unit)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 325201)
        .unwrap()
        .clone();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(325201)
        .unwrap();
    let ppm = spell
        .aura_options
        .as_ref()
        .unwrap()
        .procs_per_minute
        .as_ref()
        .unwrap()
        .base_proc_rate;
    let haste = world
        .get::<crate::stats::CharacterStats>(unit)
        .unwrap()
        .haste_pct;
    assert!(
        haste > 0.0,
        "actual native derived haste, never injected stats"
    );
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let coefficient: f32 = db.query_row(
        "SELECT Coeff FROM spell_procs_per_minute_mod WHERE ID=221 AND Type=1 AND Param=4 AND PlayerConditionID=0 AND SpellProcsPerMinuteID=57",
        [], |row| row.get(0)).unwrap();
    let effective_ppm = f64::from(ppm * (1.0 + haste / 100.0 * coefficient));
    let now = aura_now(world);
    let elapsed = (now - aura.proc.last_attempt_at).min(10.0);
    assert!(
        elapsed > 0.0,
        "ordinary fixture time elapsed since passive initialization"
    );
    let since_success = (now - aura.proc.last_success_at).min(1000.0);
    let bad_luck = (1.0 + (since_success / (60.0 / effective_ppm) - 1.5) * 3.0).max(1.0);
    let expected = (bad_luck * effective_ppm * elapsed / 60.0).clamp(0.0, 1.0) as f32 * 100.0;
    let actual = proc_chance(h.app.world_mut(), unit, aura.instance_id, now);
    assert!(
        (actual - expected).abs() < 0.00001,
        "native PPM57/mod221 must use actual regen-haste scaling: haste={haste}, coefficient={coefficient}, expected={expected}, actual={actual}"
    );
}
