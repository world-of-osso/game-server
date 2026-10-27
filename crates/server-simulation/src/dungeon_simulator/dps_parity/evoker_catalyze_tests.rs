//! Selected Catalyze speeds Fire Breath ticks while Disintegrate ticks on the same target.
use super::*;

const CATALYZE: u32 = 386283;
const FIRE_BREATH: u32 = 357208;
const FIRE_DAMAGE: u32 = 357209;
const DISINTEGRATE: u32 = 356995;

fn has_own_aura(run: &Run, unit: Entity, spell: u32) -> bool {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(unit)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == spell && aura.caster == run.actor)
        })
}

/// Steps real frames until `done`, returning each actual Fire Breath tick time
/// and whether the actor's Disintegrate was ticking on the target at that tick.
fn record_fire_ticks(run: &mut Run, done: impl Fn(&Run, usize) -> bool) -> Vec<(f64, bool)> {
    let primary = run.targets[0];
    let mut seen = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let mut ticks = Vec::new();
    for _ in 0..3000 {
        if done(run, ticks.len()) {
            return ticks;
        }
        run.app.update();
        let now = crate::auras::aura_now(run.app.world());
        let channel = has_own_aura(run, primary, DISINTEGRATE);
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let fire = events[seen..].iter().filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(primary.to_bits())
                && event.spell_id == Some(FIRE_DAMAGE)
                && event.periodic
                && event.kind == shared::protocol::CombatLogKind::Damage
        });
        ticks.extend(fire.map(|_| (now, channel)));
        seen = events.len();
    }
    panic!("Fire Breath tick window did not finish: {ticks:?}");
}

#[test]
#[ignore = "requires exact native Devastation profile and DB"]
fn evoker_catalyze_actual_disintegrate_speeds_fire_breath_ticks() {
    for selected in [false, true] {
        let mut run = prepare_literal(1467, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let primary = run.targets[0];
        assert!(crate::auras::has_aura(run.app.world(), actor, CATALYZE));
        let rate = 1.0
            + f64::from(crate::spell_cast::scripts::script_effect_value(
                run.app.world(),
                actor,
                CATALYZE,
                0,
            )) / 100.0;
        assert!(rate > 1.0);
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == CATALYZE,
            );
        }
        crate::spell_cast::creature_cast_empowered_with_targets(
            run.app.world_mut(),
            actor,
            FIRE_BREATH,
            crate::spell_cast::CastTargets {
                unit: actor,
                primary: Some(primary),
                destination: None,
            },
            1,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let before = record_fire_ticks(&mut run, |_, ticks| ticks >= 2);
        let ordinary = before[1].0 - before[0].0;
        // Start the channel just before the next ordinary tick, so its later
        // intervals are scheduled while Disintegrate ticks.
        while crate::auras::aura_now(run.app.world()) < before[1].0 + ordinary - 0.1 {
            run.app.update();
        }
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            actor,
            DISINTEGRATE,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        let during =
            record_fire_ticks(&mut run, |run, _| !has_own_aura(run, primary, DISINTEGRATE));
        // SimC76f8118 sc_evoker.cpp5365–5374: each interval scheduled while the
        // actor's Disintegrate ticks on the target divides by 1 + Catalyze.
        let catalyzed: Vec<f64> = during
            .windows(2)
            .filter(|pair| pair[0].1 && pair[1].1)
            .map(|pair| pair[1].0 - pair[0].0)
            .collect();
        assert!(
            !selected || !catalyzed.is_empty(),
            "fixture needs consecutive Fire Breath ticks inside Disintegrate: {during:?}"
        );
        let expected = if selected { ordinary / rate } else { ordinary };
        for interval in &catalyzed {
            assert!(
                (interval - expected).abs() < 0.05,
                "selected={selected}: Fire Breath interval {interval} inside Disintegrate, ordinary {ordinary}, expected {expected}; ticks {during:?}"
            );
        }
    }
}
