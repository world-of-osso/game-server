//! Native full-channel clock gates independent of DPS rotation coefficients.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::casting::{CastState, CastType};
use shared::protocol::CombatLogKind;

pub(super) fn remove_equipped_proc_drivers(run: &mut Run) {
    let world = run.app.world();
    let store = world.resource::<crate::spell_info::SpellStore>();
    let drivers: Vec<_> = world
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .filter(|aura| aura.cast_item_level.is_some())
        .filter(|aura| {
            store
                .get(aura.spell_id)
                .and_then(|spell| spell.aura_options.as_ref())
                .is_some_and(|options| options.proc_type_mask.iter().any(|&mask| mask != 0))
        })
        .map(|aura| aura.spell_id)
        .collect();
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| drivers.contains(&aura.spell_id),
    );
    run.app.update();
    run.app.update();
}

fn finish_observed_channel(run: &mut Run, spell: u32) -> (f32, f64) {
    let state = run
        .app
        .world()
        .get::<CastState>(run.actor)
        .expect("actual native channel state");
    assert_eq!(state.spell_id, spell);
    assert_eq!(state.cast_type, CastType::Channel);
    let duration = state.duration;
    let start = crate::spell_cast::now(run.app.world());
    for _ in 0..240 {
        run.app.update();
        if run.app.world().get::<CastState>(run.actor).is_none() {
            let elapsed = crate::spell_cast::now(run.app.world()) - start;
            assert!(
                (elapsed - f64::from(duration)).abs() <= 0.10,
                "native{spell} natural completion must agree with its advertised clock: {duration}s vs{elapsed}s"
            );
            return (duration, elapsed);
        }
    }
    panic!("native{spell} did not complete within12s");
}

#[test]
#[ignore = "requires exact native265 player/profile for full Drain Soul channel"]
fn shared_channel_native_drain_soul_completes_without_rounding_partial() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    remove_equipped_proc_drivers(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(198590)
        .unwrap();
    let duration = spell.misc.as_ref().unwrap().duration.unwrap().duration_ms as f32;
    let period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms as f32;
    let modified_duration = crate::auras::spell_mods::modify(
        run.app.world(),
        actor,
        spell,
        crate::auras::SpellModOp::Duration,
        duration,
    );
    let modified_duration = crate::auras::spell_mods::modify(
        run.app.world(),
        actor,
        spell,
        crate::auras::SpellModOp::ChangeCastTime,
        modified_duration,
    );
    let modified_period = crate::auras::spell_mods::modify(
        run.app.world(),
        actor,
        spell,
        crate::auras::SpellModOp::Period,
        period,
    );
    let expected_ticks = modified_duration / modified_period;
    assert!((expected_ticks - expected_ticks.round()).abs() < 0.00001);
    let haste = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .haste_pct;
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(198590, target, Some(target)),
    )
    .unwrap();
    let (advertised, elapsed) = finish_observed_channel(&mut run, 198590);
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .haste_pct;
    assert!(
        (after - haste).abs() < 0.00001,
        "constant-haste Drain Soul gate"
    );
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(198590)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0
        })
        .count();
    assert_eq!(
        hits,
        expected_ticks.round() as usize,
        "native Drain Soul must preserve original ticks without an expiry partial; advertised{advertised}s elapsed{elapsed}s"
    );
}

#[test]
#[ignore = "requires exact native103 player/profile for the unhasted4s Convoke gate"]
fn shared_channel_native_cat_convoke_keeps_four_second_lifetime() {
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(391528, actor, Some(target)),
    )
    .unwrap();
    let (advertised, elapsed) = finish_observed_channel(&mut run, 391528);
    assert!(
        (advertised - 4.0).abs() < 0.001,
        "native Convoke advertises4s, not a hasted lifetime"
    );
    assert!(
        (elapsed - 4.0).abs() <= 0.10,
        "actual Convoke remains active for4s, observed{elapsed}s"
    );
}
