use super::*;

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_ray_opens_splinterstorm_at_execution_without_tick_refresh() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    let started = crate::spell_cast::now(run.app.world());
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(205021, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1247908),
        "real Ray execution must open Splinterstorm before the first periodic update"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    let native_ms = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1247908)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .as_ref()
        .unwrap()
        .duration_ms;
    let deadline = started + f64::from(native_ms) / 1000.0 + 0.2;
    while crate::spell_cast::now(run.app.world()) < deadline {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1247908),
        "Ray ticks must not refresh the window beyond its actual native execution deadline"
    );
}
