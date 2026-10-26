//! Real Rogue auto-attack clocks under native Slice and Dice and Swift Slasher.
use super::*;
use shared::protocol::CombatLogKind;

struct SwingClock {
    remaining: Duration,
    off_hand_remaining: Duration,
    haste: f32,
    generic_time_factor: f32,
    expected_seconds: f32,
    event_count: usize,
}

fn prepare_swift_run(spec: u32) -> Run {
    let mut run = prepare_measurement(spec, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 381988));
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..1200 {
        drive_player(&mut run).unwrap();
        run.app.update();
        let swinging = run
            .app
            .world()
            .get::<crate::combat::AutoAttack>(run.actor)
            .is_some();
        if swinging && crate::auras::has_aura(run.app.world(), run.actor, 315496) {
            return run;
        }
    }
    panic!("native spec{spec} must earn Slice and Dice and start actual weapon swings");
}

fn read_native_amount(run: &Run, spell: u32, index: u8) -> f32 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .amount
}

fn read_swing_clock(run: &Run) -> SwingClock {
    let world = run.app.world();
    let stats = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap();
    let mods = world.get::<crate::auras::AuraStatMods>(run.actor).unwrap();
    let generic_time_factor =
        mods.attack_cast_time_mult * mods.attack_time_mult * mods.melee_ranged_attack_time_mult;
    let slice = read_native_amount(run, 315496, 0) / 100.0;
    let swift = read_native_amount(run, 381988, 1) / 100.0;
    let haste = stats.haste_pct / 100.0;
    assert!(haste > 0.0 && slice > 0.0 && swift > 0.0);
    // SimC76f8118 sc_rogue.cpp9140–9148: native SnD bonus plus live haste.
    let generic = stats.main_hand.speed / (1.0 + haste) * generic_time_factor;
    SwingClock {
        remaining: world
            .get::<crate::combat::AutoAttack>(run.actor)
            .unwrap()
            .timer,
        off_hand_remaining: world
            .get::<crate::combat::AutoAttack>(run.actor)
            .unwrap()
            .off_hand_timer,
        haste: stats.haste_pct,
        generic_time_factor,
        expected_seconds: generic * (1.0 + slice) / (1.0 + slice + haste * swift),
        event_count: world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len(),
    }
}

fn assert_swift_clock(spec: u32) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_swift_run(spec);
    for _ in 0..200 {
        let before = read_swing_clock(&run);
        let tick = run.app.world().resource::<Time>().delta();
        run.app.update();
        let after = read_swing_clock(&run);
        let stable =
            before.haste == after.haste && before.generic_time_factor == after.generic_time_factor;
        // An off-hand swing pushes the other timer to ATTACK_DISPLAY_DELAY.
        // Observed139ms ->200ms with off-hand42ms ->391ms is not a main swing.
        // Qualify the hand by its pre-tick deadline, not any white damage event.
        let main_was_due = before.remaining <= tick;
        if !stable || !main_was_due || after.remaining <= before.remaining {
            continue;
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events[before.event_count..].iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id.is_none()
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            }),
            "the observed reset must be an actual native weapon swing"
        );
        let actual = after.remaining.as_secs_f32();
        assert!(
            (actual - before.expected_seconds).abs() <= 0.0501,
            "spec{spec} native swing must include Swift Slasher, within one50ms tick: actual={actual}, expected={}, before_main={:?}, before_off={:?}, after_off={:?}",
            before.expected_seconds,
            before.remaining,
            before.off_hand_remaining,
            after.off_hand_remaining
        );
        return;
    }
    panic!("spec{spec} must expose a stable native main-hand swing reset");
}

#[test]
#[ignore = "requires prepared259 native player profile and traits"]
fn assassination_swift_slasher_uses_native_live_haste_in_swing_clock() {
    assert_swift_clock(259);
}

#[test]
#[ignore = "requires prepared260 native player profile and traits"]
fn outlaw_swift_slasher_uses_native_live_haste_in_swing_clock() {
    assert_swift_clock(260);
}
