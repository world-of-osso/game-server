//! SimC76f8118 priest1683–1685/3540–3541/3571–3576; shadow1491–1496.
//! Actual Torrent/Rift earns the native buff and selected post-Rift tail.
use super::*;
use crate::auras::{ActiveAuras, AuraInstance};

fn owned(run: &Run, spell: u32) -> Option<&AuraInstance> {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_voidheart_real_torrent_earns_native_bonus_full_tail_and_touch_expiry() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 449880));
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1266856));
    assert!(owned(&run, 449887).is_none());
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(263165, run.targets[0], None),
    )
    .expect("ordinary native Torrent");
    assert!(owned(&run, 459314).is_some(), "actual Torrent earns Rift");
    let heart = owned(&run, 449887)
        .expect("actual Rift must earn native Voidheart before periodic updates");
    let duration = heart.duration_ms;
    for index in 0..=1 {
        let expected = crate::spell_cast::scripts::script_effect_value(
            run.app.world(),
            run.actor,
            449887,
            index,
        );
        let amount = heart
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .unwrap()
            .amount;
        assert!(
            (amount - expected).abs() < 0.001,
            "Voidheart keeps native effect{index}: {amount} versus{expected}"
        );
    }
    for _ in 0..240 {
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
        {
            break;
        }
        run.app.update();
    }
    assert!(run
        .app
        .world()
        .get::<shared::casting::CastState>(run.actor)
        .is_none());
    let now = crate::spell_cast::now(run.app.world());
    let expiry = owned(&run, 449887).unwrap().expires_at.unwrap();
    let expected = now + f64::from(duration) / 1000.0;
    assert!(
        (expiry - expected).abs() < 0.051,
        "channel end restores full native Voidheart tail: {expiry} versus{expected}"
    );
    for _ in 0..400 {
        if owned(&run, 459314).is_none() {
            break;
        }
        run.app.update();
    }
    assert!(owned(&run, 459314).is_none(), "actual finite Rift expires");
    let seconds =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1266856, 0);
    assert_eq!(seconds, 8.0, "native selected Touch duration");
    let now = crate::spell_cast::now(run.app.world());
    let tail_expiry = owned(&run, 449887)
        .expect("real Rift expiry earns selected Touch tail")
        .expires_at
        .unwrap();
    assert!(
        (tail_expiry - now - f64::from(seconds)).abs() < 0.051,
        "Touch uses its native duration: {tail_expiry} at{now}"
    );
    let frames = (seconds * 20.0).ceil() as usize + 2;
    for _ in 0..frames {
        run.app.update();
    }
    assert!(
        owned(&run, 449887).is_none(),
        "selected native tail expires without another Rift"
    );
}
