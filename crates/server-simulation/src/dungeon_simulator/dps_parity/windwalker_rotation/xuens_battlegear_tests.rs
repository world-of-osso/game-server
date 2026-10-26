//! SIMC_WW_XUENS_BATTLEGEAR_FOF_RECOVERY: actual casts, impacts and running timers.
//! No aura/stack/power/stat/crit grants, clock rewinds or coefficient changes.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::powers::power_value;
use crate::spell_cast::test_harness::{ObservedCombatEvents, ObservedSelectedTargets};
use shared::components::{Position, PowerType};
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::HashSet;

type Run = super::super::Run;
const TALENT: u32 = 392993;
const TALENT_ENTRY: u32 = 125017;
const FOF: u32 = 113656;
const SCK: u32 = 101546;
const SCK_TICK: u32 = 107270;
const RSK: u32 = 107428;
const RSK_DAMAGE: u32 = 185099;
const RWK: u32 = 467307;
const RWK_DAMAGE: u32 = 468179;

fn prepare(count: usize, selected: bool) -> Run {
    let (profile, mut traits) = super::super::read_reference(269).unwrap();
    if !selected {
        let original = traits.len();
        traits.retain(|entry| entry.entry_id != TALENT_ENTRY);
        assert_eq!(traits.len() + 1, original, "native70448 Xuen entry125017");
    }
    let identity = super::super::inputs::read_actor(&profile).unwrap();
    let mut run =
        super::super::prepare_literal_reference(profile, &traits, count, identity).unwrap();
    run.spec_driver = Some(super::prepare_driver(&mut run).unwrap());
    run.app.init_resource::<ObservedSelectedTargets>();
    assert_eq!(
        crate::auras::has_aura(run.app.world(), run.actor, TALENT),
        selected
    );
    assert_eq!(budget(&run, 1), 4.0);
    assert_eq!(budget(&run, 3), 0.5);
    assert_eq!(budget(&run, 4), 2.5);
    run
}

fn budget(run: &Run, index: u8) -> f64 {
    f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        TALENT,
        index,
    )) / 1000.0
}

fn remaining(run: &Run) -> f64 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(FOF)
        .unwrap();
    // No history means no timer exists; this also covers the no-new-cooldown control.
    world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .map_or(0.0, |history| {
            history.remaining_cooldown_ms(spell, crate::spell_cast::history_now(world)) as f64
                / 1000.0
        })
}

fn deadline(run: &Run) -> f64 {
    crate::spell_cast::history_now(run.app.world()) + remaining(run)
}

fn chi(run: &Run) -> f32 {
    power_value(run.app.world(), run.actor, PowerType::Chi)
        .unwrap()
        .0
}

fn cast(run: &mut Run, spell: u32, target: Entity) {
    use crate::spell_cast::CastFailure;
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => {
                run.app.update();
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(reason) => panic!("ordinary native spell{spell}: {reason:?}"),
        }
    }
    panic!("ordinary native spell{spell} failed to become ready");
}

fn earn_chi(run: &mut Run, amount: f32) {
    for _ in 0..16 {
        if chi(run) >= amount {
            return;
        }
        let target = run.targets[0];
        cast(run, 100780, target);
    }
    panic!("real Tiger Palm failed to earn{amount} Chi");
}

fn wait_fof_ready(run: &mut Run) {
    for _ in 0..1200 {
        if remaining(run) == 0.0 {
            return;
        }
        run.app.update();
    }
    panic!("real FoF timer did not recover");
}

fn prime_fof(run: &mut Run) {
    wait_fof_ready(run);
    earn_chi(run, 5.0);
    let target = run.targets[0];
    cast(run, FOF, target);
    assert!(
        remaining(run) > 10.0,
        "actual FoF must start its native cooldown"
    );
}

fn advance(run: &mut Run, seconds: f64) {
    let end = crate::spell_cast::history_now(run.app.world()) + seconds;
    for _ in 0..1200 {
        if crate::spell_cast::history_now(run.app.world()) >= end {
            return;
        }
        run.app.update();
    }
    panic!("ordinary fixture time did not advance{seconds}s");
}

fn damage_since(run: &Run, cursor: usize, spell: u32) -> Vec<CombatLogEvent> {
    run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
        })
        .cloned()
        .collect()
}

struct Sample {
    before: f64,
    after: f64,
    selected: Vec<Entity>,
    damage: Vec<CombatLogEvent>,
    glory_criticals: usize,
}

fn sample_cast(run: &mut Run, root: u32, damage: u32) -> Sample {
    use crate::spell_cast::CastFailure;
    for _ in 0..1200 {
        let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
        let before = deadline(run);
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            root,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => {
                run.app.update();
                let selected = run
                    .app
                    .world()
                    .resource::<ObservedSelectedTargets>()
                    .0
                    .iter()
                    .rev()
                    .find(|(caster, spell, _)| *caster == run.actor && *spell == damage)
                    .expect("actual native damage child must resolve a TargetInfo list")
                    .2
                    .clone();
                return Sample {
                    before,
                    after: deadline(run),
                    selected,
                    damage: damage_since(run, cursor, damage),
                    glory_criticals: damage_since(run, cursor, 392959)
                        .iter()
                        .filter(|event| {
                            event.crit && event.target == Some(run.targets[0].to_bits())
                        })
                        .count(),
                };
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(reason) => panic!("native root{root} failed: {reason:?}"),
        }
    }
    panic!("native root{root} never became ready");
}

fn receipt(label: &str, value: Value) {
    let path = super::super::inputs::directory()
        .unwrap()
        .join(format!("{label}.json"));
    std::fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

fn assert_sample(label: &str, sample: &Sample, expected: f64) {
    let actual = sample.before - sample.after;
    receipt(
        label,
        json!({"deadline_before":sample.before,"deadline_after":sample.after,
        "reduction":actual,"expected":expected,"selected_native_order":sample.selected.iter().map(|u|u.to_bits()).collect::<Vec<_>>(),
        "damage":sample.damage.iter().map(|e|json!({"target":e.target,"amount":e.amount,"crit":e.crit})).collect::<Vec<_>>()}),
    );
    assert!(
        (actual - expected).abs() < 0.002,
        "{label}: actual running FoF reduction{actual}, expected{expected}"
    );
}

fn expected_kick_reduction(run: &Run, sample: &Sample, primary_critical: bool) -> f64 {
    let criticals = usize::from(primary_critical) + sample.glory_criticals;
    let available = (sample.before - crate::spell_cast::history_now(run.app.world())).max(0.0);
    (criticals as f64 * budget(run, 1)).min(available)
}

fn observe_rsk(selected: bool, wanted_crit: bool, label: &str) {
    let mut run = prepare(1, selected);
    for _ in 0..8 {
        prime_fof(&mut run);
        // Let native post-FoF temporary crit buffs expire, not rewrite crit stats.
        advance(&mut run, 10.0);
        earn_chi(&mut run, 2.0);
        let sample = sample_cast(&mut run, RSK, RSK_DAMAGE);
        if sample.damage.iter().any(|event| event.crit == wanted_crit) {
            assert!(!sample.selected.is_empty());
            assert_sample(
                label,
                &sample,
                if selected {
                    expected_kick_reduction(&run, &sample, wanted_crit)
                } else {
                    0.0
                },
            );
            return;
        }
    }
    panic!(
        "natural RSK did not produce requested crit={wanted_crit} through eight real FoF/RSK cycles"
    );
}

fn earn_rwk(run: &mut Run) {
    for _ in 0..64 {
        if crate::auras::has_aura(run.app.world(), run.actor, 1250554) {
            return;
        }
        let target = run.targets[0];
        cast(run, 100780, target);
        if crate::auras::has_aura(run.app.world(), run.actor, 116768) {
            cast(run, 100784, target);
        }
    }
    panic!("native Combo Breaker/BOK must earn real RWK override");
}

fn observe_rwk(primary_critical: bool) {
    let mut run = prepare(3, true);
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        RWK_DAMAGE,
        "xuen_rwk",
        64,
    ));
    // Keep advancing the same native trajectory; never reroll/reset an outcome.
    for attempt in 0..32 {
        wait_fof_ready(&mut run);
        earn_rwk(&mut run);
        prime_fof(&mut run);
        advance(&mut run, 10.0);
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, 1250554),
            "earned RWK still live"
        );
        let sample = sample_cast(&mut run, RWK, RWK_DAMAGE);
        let primary = sample
            .selected
            .first()
            .expect("native RWK cone has actual targets")
            .to_bits();
        let primary_crit = sample
            .damage
            .iter()
            .any(|event| event.target == Some(primary) && event.crit);
        let secondary_crit = sample
            .damage
            .iter()
            .any(|event| event.target != Some(primary) && event.crit);
        receipt(
            &format!("xuen-rwk-{primary_critical}-{attempt}"),
            json!({
                "primary_crit":primary_crit,"secondary_crit":secondary_crit,
                "selected":sample.selected.iter().map(|unit|unit.to_bits()).collect::<Vec<_>>(),
                "damage":sample.damage.iter().map(|event|json!({"target":event.target,"crit":event.crit,"amount":event.amount})).collect::<Vec<_>>(),
                "factors":run.app.world().resource::<FactorCapture>().export(),
            }),
        );
        if primary_critical && primary_crit {
            assert_sample(
                "xuen-rwk-primary-crit",
                &sample,
                expected_kick_reduction(&run, &sample, true),
            );
            return;
        }
        if !primary_critical && !primary_crit && secondary_crit {
            assert_sample(
                "xuen-rwk-secondary-only-crit",
                &sample,
                expected_kick_reduction(&run, &sample, false),
            );
            return;
        }
    }
    panic!("natural earned RWK did not produce requested primary/secondary critical boundary");
}

fn finish_sck(run: &mut Run) {
    for _ in 0..200 {
        run.app.update();
        if !crate::auras::has_aura(run.app.world(), run.actor, SCK) {
            return;
        }
    }
    panic!("actual native SCK did not expire");
}

fn sample_sck(run: &mut Run, no_hits: bool) -> (f64, usize, usize) {
    let before = deadline(run);
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let selected_cursor = run
        .app
        .world()
        .resource::<ObservedSelectedTargets>()
        .0
        .len();
    let target = if no_hits { run.actor } else { run.targets[0] };
    cast(run, SCK, target);
    finish_sck(run);
    let damage = damage_since(run, cursor, SCK_TICK);
    let selected: HashSet<_> = run.app.world().resource::<ObservedSelectedTargets>().0
        [selected_cursor..]
        .iter()
        .filter(|(caster, spell, _)| *caster == run.actor && *spell == SCK_TICK)
        .flat_map(|(_, _, units)| units.iter().copied())
        .collect();
    (before - deadline(run), selected.len(), damage.len())
}

fn move_targets_out(run: &mut Run) {
    for &target in &run.targets {
        run.app.world_mut().entity_mut(target).insert(Position {
            x: 100.0,
            y: 0.0,
            z: 0.0,
        });
    }
}

fn assert_sck(count: usize, selected: bool) {
    let mut run = prepare(count, selected);
    prime_fof(&mut run);
    let (actual, unique, impacts) = sample_sck(&mut run, false);
    assert_eq!(
        unique, count,
        "actual native impact TargetInfos, not declared target count"
    );
    assert!(
        impacts > unique,
        "real repeated native damage ticks must revisit actors"
    );
    let expected = if selected {
        (unique as f64 * budget(&run, 3)).min(budget(&run, 4))
    } else {
        0.0
    };
    receipt(
        &format!("xuen-sck-{count}-{selected}"),
        json!({"unique_impacted":unique,"damage_events":impacts,"actual":actual,"expected":expected}),
    );
    assert!(
        (actual - expected).abs() < 0.002,
        "SCK{count}: actual recovery{actual}, expected{expected} from unique actors"
    );
}

#[test]
#[ignore = "real primary RSK critical impact must reduce an existing native FoF timer"]
fn literal_ww_native_xuen_recovery_primary_rsk_critical() {
    let _seed = crate::random::seed_scope(70448);
    observe_rsk(true, true, "xuen-rsk-primary-crit");
}

#[test]
#[ignore = "real noncritical RSK control, with unchanged native crit stats"]
fn literal_ww_native_xuen_recovery_noncritical_rsk_control() {
    let _seed = crate::random::seed_scope(70448);
    observe_rsk(true, false, "xuen-rsk-noncrit");
}

#[test]
#[ignore = "native RWK inherits RSK base impact: primary critical reduces once"]
fn literal_ww_native_xuen_recovery_primary_rwk_critical() {
    let _seed = crate::random::seed_scope(70448);
    observe_rwk(true);
}

#[test]
#[ignore = "native RWK secondary critical with no primary critical must not reduce FoF"]
fn literal_ww_native_xuen_recovery_secondary_rwk_critical_control() {
    let _seed = crate::random::seed_scope(70448);
    observe_rwk(false);
}

#[test]
#[ignore = "one actual SCK actor across repeated native ticks counts once"]
fn literal_ww_native_xuen_recovery_sck_one_unique_actor() {
    let _seed = crate::random::seed_scope(70448);
    assert_sck(1, true);
}

#[test]
#[ignore = "three real SCK actors yield the native per-actor amount"]
fn literal_ww_native_xuen_recovery_sck_three_unique_actors() {
    let _seed = crate::random::seed_scope(70448);
    assert_sck(3, true);
}

#[test]
#[ignore = "six actual SCK actors reach the native maximum, not six tick reductions"]
fn literal_ww_native_xuen_recovery_sck_native_cap() {
    let _seed = crate::random::seed_scope(70448);
    assert_sck(6, true);
}

#[test]
#[ignore = "actual no-target SCK expiry must leave the existing FoF timer unchanged"]
fn literal_ww_native_xuen_recovery_sck_no_hit_control() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1, true);
    prime_fof(&mut run);
    move_targets_out(&mut run);
    let (actual, unique, damage) = sample_sck(&mut run, true);
    assert_eq!((unique, damage), (0, 0));
    assert!(actual.abs() < 0.002, "no impacts means no recovery");
}

#[test]
#[ignore = "actual stop-cast after SCK impacts is not natural completion and grants no recovery"]
fn literal_ww_native_xuen_recovery_interrupted_sck_control() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1, true);
    prime_fof(&mut run);
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let target = run.targets[0];
    cast(&mut run, SCK, target);
    for _ in 0..20 {
        if !damage_since(&run, cursor, SCK_TICK).is_empty() {
            break;
        }
        run.app.update();
    }
    assert!(!damage_since(&run, cursor, SCK_TICK).is_empty());
    assert!(crate::auras::has_aura(run.app.world(), run.actor, SCK));
    let before = deadline(&run);
    crate::spell_cast::cancel_cast(run.app.world_mut(), run.actor);
    run.app.update();
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, SCK));
    advance(&mut run, 3.0);
    let actual = before - deadline(&run);
    receipt(
        "xuen-sck-interrupted",
        json!({"actual":actual,"expected":0.0}),
    );
    assert!(
        actual.abs() < 0.002,
        "stopping a real channel must not grant expiry recovery"
    );
}

#[test]
#[ignore = "unselected native Xuen entry, actual critical RSK and existing FoF timer"]
fn literal_ww_native_xuen_recovery_unselected_rsk_control() {
    let _seed = crate::random::seed_scope(70448);
    observe_rsk(false, true, "xuen-rsk-unselected");
}

#[test]
#[ignore = "unselected native Xuen entry, actual repeated SCK impacts"]
fn literal_ww_native_xuen_recovery_unselected_sck_control() {
    let _seed = crate::random::seed_scope(70448);
    assert_sck(3, false);
}

#[test]
#[ignore = "actual SCK hits cannot start a FoF cooldown that does not exist"]
fn literal_ww_native_xuen_recovery_does_not_start_cooldown() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1, true);
    earn_chi(&mut run, 2.0);
    assert_eq!(remaining(&run), 0.0);
    let (_, unique, damage) = sample_sck(&mut run, false);
    assert_eq!(unique, 1);
    assert!(
        damage > 1,
        "real repeated SCK impacts without an existing FoF timer"
    );
    assert_eq!(remaining(&run), 0.0);
}

#[test]
#[ignore = "after one actual SCK recovery, a later no-hit parent must not reuse its set"]
fn literal_ww_native_xuen_recovery_later_parent_clears_unique_actors() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1, true);
    prime_fof(&mut run);
    let (actual, unique, _) = sample_sck(&mut run, false);
    assert_eq!(unique, 1);
    assert!((actual - budget(&run, 3)).abs() < 0.002);
    earn_chi(&mut run, 2.0);
    move_targets_out(&mut run);
    let (later, unique, damage) = sample_sck(&mut run, true);
    assert_eq!((unique, damage), (0, 0));
    assert!(
        later.abs() < 0.002,
        "previous unique actor set must be cleared"
    );
}
