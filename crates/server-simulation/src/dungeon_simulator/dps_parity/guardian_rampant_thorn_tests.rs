//! Guardian MID2 4pc extends the real Incarnation from real Thrash casts.
use super::*;
use crate::auras::ActiveAuras;

#[test]
#[ignore = "requires Guardian MID2 profile and native world data"]
fn guardian_thrash_extends_incarnation_to_native_five_second_cap() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 102558, Some(actor), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let expiry = |run: &Run| {
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 102558 && aura.caster == actor)
            .unwrap()
            .expires_at
            .unwrap()
    };
    let initial = expiry(&run);
    for _ in 0..11 {
        crate::spell_cast::scripts::reduce_cooldown(run.app.world_mut(), actor, 77758, f64::MAX);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
            .unwrap();
        for _ in 0..35 {
            run.app.update();
        }
    }
    let final_expiry = expiry(&run);
    assert!(
        (final_expiry - initial - 5.0).abs() < 0.001,
        "eleven real Thrash casts extend Incarnation only 5 seconds: {initial} -> {final_expiry}"
    );
}
