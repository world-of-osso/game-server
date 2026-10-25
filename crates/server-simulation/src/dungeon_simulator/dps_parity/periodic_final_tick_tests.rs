//! SimC76f8118 dot.cpp1154–1170/action.cpp5223: finite DoTs pay a fractional final tick.
use super::*;
use crate::followers::simulator::ParityActor;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn native_dot(spec: u32, spell: u32, actor: ParityActor) -> Run {
    let (profile, traits) = read_reference(spec).expect("exact native profile and traits");
    let mut run = prepare_literal_reference(profile, &traits, 1, actor).unwrap();
    engage(&mut run);
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.targets[0]),
        0,
    );
    run
}

fn expected_ticks(run: &Run, spell: u32) -> u32 {
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
        .expect("real native periodic application");
    let period = aura
        .effects
        .iter()
        .find(|effect| effect.period_ms > 0)
        .unwrap()
        .period_ms;
    assert_ne!(
        aura.duration_ms % period,
        0,
        "native profile must leave a fractional final interval"
    );
    aura.duration_ms.div_ceil(period) + read_initial_periodic_ticks(run, spell)
}

fn read_initial_periodic_ticks(run: &Run, spell: u32) -> u32 {
    // TC SharedDefines.h630; SimC action.cpp673 reads SX_TICK_ON_APPLICATION.
    const EXTRA_INITIAL_PERIOD: u32 = 0x200;
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap();
    u32::from(
        spell
            .misc
            .as_ref()
            .is_some_and(|misc| misc.attributes[5] & EXTRA_INITIAL_PERIOD != 0),
    )
}

fn periodic_damage_ticks(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.periodic
        })
        .count()
}

fn assert_native_final_tick(spec: u32, spell: u32, actor: ParityActor) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_dot(spec, spell, actor);
    let expected = expected_ticks(&run, spell);
    for _ in 0..1600 {
        run.app.update();
        if !crate::auras::has_aura(run.app.world(), run.targets[0], spell) {
            break;
        }
    }
    assert!(!crate::auras::has_aura(
        run.app.world(),
        run.targets[0],
        spell
    ));
    assert_eq!(
        periodic_damage_ticks(&run, spell),
        expected as usize,
        "native full intervals plus one fractional final damage event"
    );
}

#[test]
#[ignore = "native spec70 profile/traits and prepared world DB"]
fn native_periodic_final_dawnlight_tick() {
    assert_native_final_tick(70, 431380, ParityActor::Follower);
}

#[test]
#[ignore = "native spec258 profile/traits and prepared world DB"]
fn native_periodic_final_shadow_word_pain_tick() {
    assert_native_final_tick(258, 589, ParityActor::Player);
}
