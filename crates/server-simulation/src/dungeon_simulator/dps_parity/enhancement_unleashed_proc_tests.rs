//! Real delayed Unleashed weapon hits must retain imbue callbacks, independently of white swings.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;
use std::time::Duration;

fn hits(run: &Run, spell: Option<u32>) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == spell
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

fn defer_white_swings(run: &mut Run) {
    const DORMANT_SWING: Duration = Duration::from_secs(60);
    // Keep the actual victim so MELEE_COMBAT_START cannot recreate a ready white swing.
    // Only unrelated fixture swing clocks are deferred; native weapon procs/timers are untouched.
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::combat::AutoAttack {
            timer: DORMANT_SWING,
            off_hand_timer: DORMANT_SWING,
            victim: run.targets[0],
        });
}

fn cast_crash_with_dormant_white_swings(run: &mut Run) -> usize {
    let target = run.targets[0];
    defer_white_swings(run);
    for _ in 0..1200 {
        let windfury_before = hits(run, Some(25504));
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            187874,
            Some(target),
            false,
        ) {
            Ok(()) => {
                for _ in 0..6 {
                    run.app.update();
                }
                return windfury_before;
            }
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("native Crash imbue fixture: {reason:?}"),
        }
    }
    panic!("native Crash imbue fixture did not become ready");
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_delayed_unleashed_hits_proc_real_flametongue_and_windfury_without_white_swings() {
    // SimC76f8118 sc_shaman.cpp3039–3055/3095–3101: eligible main-hand weapon impacts
    // run imbue callbacks, including the actual1252431 delayed ground strikes.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(263, 3).unwrap();
    engage(&mut run);
    for buff in [319773, 319778] {
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, buff),
            "native imbue{buff} must be equipped"
        );
    }
    for _ in 0..8 {
        cast_crash_with_dormant_white_swings(&mut run);
        let before_unleashed = hits(&run, Some(1252431));
        let before_flametongue = hits(&run, Some(10444));
        let before_windfury = hits(&run, Some(25504));
        let before_white = hits(&run, None);
        for _ in 0..40 {
            run.app.update();
        }
        let unleashed = hits(&run, Some(1252431)) - before_unleashed;
        let flametongue = hits(&run, Some(10444)) - before_flametongue;
        let windfury = hits(&run, Some(25504)) - before_windfury;
        assert_eq!(
            hits(&run, None),
            before_white,
            "white swings cannot explain the delayed imbue hits"
        );
        assert!(
            unleashed > 0,
            "real Crash must supply delayed native weapon hits"
        );
        assert!(
            flametongue >= unleashed,
            "each real Unleashed weapon hit must trigger native Flametongue: unleashed={unleashed}, flametongue={flametongue}"
        );
        if windfury > 0 {
            return;
        }
    }
    panic!(
        "delayed Unleashed weapon hits must also roll native Windfury within eight actual Crashes"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_stormwell_primary_crash_guarantees_native_windfury_without_white_swings() {
    // SimC76f8118 sc_shaman.cpp6141–6145: selected Stormwell guarantees one
    // Windfury proc on the landed primary, separately from ordinary proc rolls.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(263, 1).unwrap();
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(1264762),
        "actual Stormwell must be selected"
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 319773));
    for cast_index in 0..8 {
        let roots_before = hits(&run, Some(187874));
        let white_before = hits(&run, None);
        let windfury_before = cast_crash_with_dormant_white_swings(&mut run);
        assert!(
            hits(&run, Some(187874)) > roots_before,
            "actual Crash must land"
        );
        assert_eq!(
            hits(&run, None),
            white_before,
            "white swings cannot supply the proc"
        );
        assert!(
            hits(&run, Some(25504)) > windfury_before,
            "selected Stormwell must guarantee Windfury for actual primary Crash {cast_index}"
        );
    }
}
