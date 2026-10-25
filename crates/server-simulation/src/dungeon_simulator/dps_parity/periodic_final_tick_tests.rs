//! SimC76f8118 dot.cpp1154–1170/action.cpp5223: fractional final periodic ticks.
use super::*;
use crate::followers::simulator::ParityActor;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn native_run(spec: u32, actor: ParityActor) -> Run {
    let (profile, traits) = read_reference(spec).expect("exact native profile and traits");
    let mut run = prepare_literal_reference(profile, &traits, 1, actor).unwrap();
    engage(&mut run);
    run
}

fn cast_periodic(run: &mut Run, spell: u32, target: Entity) {
    run.app.world_mut().insert_resource(FactorCapture::new(
        run.actor,
        target,
        spell,
        "native_final_tick",
        100,
    ));
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, spell, Some(target), 0);
}

fn read_tick_expectations(run: &Run, spell: u32, target: Entity) -> (u32, f64) {
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
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
    let elapsed = aura.duration_ms % period;
    assert_ne!(
        elapsed, 0,
        "native profile must leave a fractional final interval"
    );
    let count = aura.duration_ms.div_ceil(period) + read_initial_periodic_ticks(run, spell);
    (count, f64::from(elapsed) / f64::from(period))
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

fn periodic_ticks(run: &Run, spell: u32, target: Entity, kind: &CombatLogKind) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(spell)
                && &event.kind == kind
                && event.periodic
        })
        .count()
}

fn assert_expiry_ticks(run: &mut Run, spell: u32, target: Entity, kind: CombatLogKind) -> f64 {
    let (expected, fraction) = read_tick_expectations(run, spell, target);
    for _ in 0..1600 {
        run.app.update();
        if !crate::auras::has_aura(run.app.world(), target, spell) {
            break;
        }
    }
    assert!(!crate::auras::has_aura(run.app.world(), target, spell));
    assert_eq!(
        periodic_ticks(run, spell, target, &kind),
        expected as usize,
        "native initial/full intervals plus one fractional final event"
    );
    fraction
}

fn assert_fractional_damage(run: &Run, spell: u32, fraction: f64) {
    let capture = run.app.world().resource::<FactorCapture>().export();
    let result = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|event| event["event"] == "result" && event["spell_id"] == spell)
        .unwrap();
    let factors = &result["factors"];
    let source = factors["native_source_amount"].as_f64().unwrap();
    let critical = 1.0 + factors["crit_bonus"].as_f64().unwrap();
    let target = factors["target_product"].as_f64().unwrap();
    let expected = (source * critical * target * fraction).round();
    let actual = factors["actual_amount"].as_f64().unwrap();
    assert!(
        (actual - expected).abs() <= 1.0,
        "final damage must scale the real modified amount by{fraction}: expected{expected}, actual{actual}"
    );
}

fn assert_native_final_damage(spec: u32, spell: u32, actor: ParityActor) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_run(spec, actor);
    let target = run.targets[0];
    cast_periodic(&mut run, spell, target);
    let fraction = assert_expiry_ticks(&mut run, spell, target, CombatLogKind::Damage);
    assert_fractional_damage(&run, spell, fraction);
}

#[test]
#[ignore = "native spec70 profile/traits and prepared world DB"]
fn native_periodic_final_dawnlight_tick() {
    assert_native_final_damage(70, 431380, ParityActor::Follower);
}

#[test]
#[ignore = "native spec258 profile/traits and prepared world DB"]
fn native_periodic_final_shadow_word_pain_tick() {
    let (profile, _) = read_reference(258).unwrap();
    let race = inputs::read_reference_race(&profile.race).unwrap();
    assert_native_final_damage(258, 589, ParityActor::Player { race });
}

#[test]
#[ignore = "native Priest profile/traits and prepared world DB"]
fn native_periodic_final_renew_tick() {
    let _seed = crate::random::seed_scope(70448);
    let (profile, _) = read_reference(258).unwrap();
    let race = inputs::read_reference_race(&profile.race).unwrap();
    let mut run = native_run(258, ParityActor::Player { race });
    let target = run.actor;
    cast_periodic(&mut run, 139, target);
    assert_expiry_ticks(&mut run, 139, target, CombatLogKind::Heal);
}
