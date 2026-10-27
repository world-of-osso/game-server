//! Source-defined integer tick count on an uninterrupted real Missiles channel.
use super::*;
use crate::auras::{ActiveAuras, SpellModOp, spell_mods};
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::casting::{CastState, CastType};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native70448 Arcane player profile"]
fn arcane_missiles_native_amplification_rounds_full_channel_ticks() {
    // SimC76f8118 sc_mage.cpp3267–3296: round duration / modified tick
    // interval to a whole tick, then add the real initial tick. Amplification
    // uses native236628 Period op19; native haste/stats/actor remain unchanged.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(62, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 236628),
        "native profile selects Amplification"
    );
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(run.actor)
            .unwrap()
            .has_spell(236628),
        "native Amplification aura is active"
    );
    for aura in [1296581, 414379] {
        assert!(
            run.app
                .world()
                .get::<ActiveAuras>(run.actor)
                .unwrap()
                .has_spell(aura),
            "native timing aura {aura} is active"
        );
    }
    super::arcane_tests::grant(&mut run, 263725, 1);
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(5143)
        .unwrap()
        .clone();
    let base_duration = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .unwrap()
        .duration_ms as f32;
    let period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms as f32;
    let modified_duration = spell_mods::modify(
        run.app.world(),
        run.actor,
        &spell,
        SpellModOp::Duration,
        base_duration,
    );
    let modified_period = spell_mods::modify(
        run.app.world(),
        run.actor,
        &spell,
        SpellModOp::Period,
        period,
    );
    let scheduled = (modified_duration / modified_period).round() as usize;
    assert_eq!(
        scheduled, 7,
        "native Amplification and tier bonus must produce seven scheduled ticks"
    );
    let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    super::arcane_tests::start_cast(&mut run, 5143);
    let cast = run
        .app
        .world()
        .get::<CastState>(run.actor)
        .expect("actual Missiles channel");
    assert_eq!((cast.spell_id, cast.cast_type), (5143, CastType::Channel));
    let duration_ms = cast.duration * 1000.0;
    let interval_ms = cast.channel_tick_interval * 1000.0;
    preparation::finish_pending_setup(&mut run).unwrap();
    let end = crate::spell_cast::now(run.app.world()) + 3.0;
    while crate::spell_cast::now(run.app.world()) < end {
        run.app.update();
    }
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .skip(before)
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(7268)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        hits,
        scheduled + 1,
        "full uninterrupted native channel must deliver seven rounded scheduled ticks plus tick_zero; duration={duration_ms}ms period={interval_ms}ms"
    );
    assert!(
        (duration_ms - scheduled as f32 * interval_ms).abs() <= 1.0,
        "actual native channel duration must align with rounded source ticks; duration={duration_ms}ms period={interval_ms}ms scheduled={scheduled}"
    );
}
