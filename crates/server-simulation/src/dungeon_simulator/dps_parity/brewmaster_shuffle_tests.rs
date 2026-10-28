#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn brew_keg_smash_grants_and_extends_shuffle() {
    let mut run = super::prepare_literal(268, 1).unwrap();
    super::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
        .unwrap();
    let now = crate::spell_cast::now(run.app.world());
    let remaining = |world: &bevy::prelude::World| {
        world
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 215479 && aura.caster == actor)
            .and_then(|aura| aura.expires_at)
            .unwrap()
            - now
    };
    let first = remaining(run.app.world());
    assert!(first > 4.9 && first <= 5.0, "first Shuffle: {first}");
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 205523, Some(target), false)
        .unwrap();
    let second = remaining(run.app.world());
    assert!(
        second > first + 2.9,
        "Blackout Kick extends Shuffle: {first} -> {second}"
    );
}
