use super::*;

#[test]
#[ignore = "requires canonical 266 inputs; real Stormbound channel grants haste"]
fn demonology_stormbound_channel_grants_native_haste() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(266, 1).unwrap();
    let owner = run.actor;
    assert!(
        crate::auras::has_aura(run.app.world(), owner, 1294744),
        "actual equipped Stormbound passive"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, 1295275, Some(owner), false)
        .expect("actual native Stormbound channel");
    for _ in 0..60 {
        run.app.update();
    }
    let now = crate::spell_cast::now(run.app.world());
    let buff = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(owner)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1294745 && aura.caster == owner)
        .expect("completed native Stormbound channel grants haste");
    let amount = buff
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let remaining = buff.expires_at.unwrap() - now;
    assert!(
        amount > 0.0,
        "haste amount must use equipped item scaling: {amount}"
    );
    assert!(
        (18.0..=20.1).contains(&remaining),
        "four completed ticks grant 20 seconds: {remaining}"
    );
}
