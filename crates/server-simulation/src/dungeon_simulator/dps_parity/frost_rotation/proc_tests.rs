//! Earned Frost proc relationships through ordinary native casts and weapon swings.
use super::*;
use crate::auras::{self, ActiveAuras, RemoveReason};

fn clear_proc(run: &mut Run, spell: u32) {
    auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        RemoveReason::Consumed,
        |aura| aura.spell_id == spell,
    );
}

fn cast(run: &mut Run, spell: u32) {
    let target = run.targets[0];
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
                super::super::preparation::finish_pending_setup(run).unwrap();
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(reason) => panic!("native Frost cast{spell}: {reason:?}"),
        }
    }
    panic!("native Frost cast{spell} never became ready");
}

#[test]
#[ignore = "native251 profile, real Pillar and ordinary Frost Strike"]
fn frost_obliteration_frost_strike_earns_killing_machine() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let rotation = Rotation::load(run.app.world_mut(), run.actor, &run.profile, encounter).unwrap();
    super::super::engage(&mut run);
    cast(&mut run, 49020);
    cast(&mut run, 49020);
    let actor = run.actor;
    super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
    assert!(auras::has_aura(run.app.world(), actor, 281238));
    assert!(auras::has_aura(run.app.world(), actor, 51271));
    clear_proc(&mut run, 51124);
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49143, Some(target), false)
        .unwrap();
    assert!(
        auras::has_aura(run.app.world(), actor, 51124),
        "successful Frost Strike during actual Pillar must earn native Killing Machine"
    );
    let facts = NativeFacts::new(run.app.world(), actor, &rotation.bindings, encounter, 1);
    assert_eq!(
        facts.value("buff.killing_machine.react").unwrap(),
        1.0,
        "a predictable Obliteration proc reacts at acquisition without advancing time"
    );
}

#[test]
#[ignore = "native251 profile, repeated native-resource-funded Frost Strike"]
fn frost_frost_strikes_roll_native_rime_chance() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 59057));
    let mut procs = 0;
    for _ in 0..24 {
        cast(&mut run, 49020);
        cast(&mut run, 49020);
        clear_proc(&mut run, 59052);
        cast(&mut run, 49143);
        if auras::has_aura(run.app.world(), run.actor, 59052) {
            procs += 1;
        }
    }
    assert!(
        (1..24).contains(&procs),
        "native45% Rime is neither absent nor an unconditional grant: {procs}/24"
    );
}

#[test]
#[ignore = "native251 profile, actual critical white swings"]
fn frost_critical_white_swings_earn_killing_machine() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    cast(&mut run, 49020);
    clear_proc(&mut run, 51124);
    let earned = (0..2000).any(|_| {
        run.app.update();
        run.app
            .world()
            .get::<ActiveAuras>(run.actor)
            .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == 51124))
    });
    assert!(
        earned,
        "real critical white swings must roll Killing Machine PRD"
    );
}

fn earn_rime(run: &mut Run) {
    for _ in 0..24 {
        cast(run, 49020);
        cast(run, 49020);
        cast(run, 49143);
        if auras::has_aura(run.app.world(), run.actor, 59052) {
            return;
        }
    }
    panic!("ordinary native Frost Strikes never earned Rime");
}

fn erw_full_recharge(run: &Run) -> f64 {
    let bindings = super::bindings::build();
    NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
        1,
    )
    .value("cooldown.empower_rune_weapon.full_recharge_time")
    .unwrap()
}

#[test]
#[ignore = "native251 earned Rime must be spent by one ordinary Howling Blast"]
fn frost_howling_blast_spends_earned_rime() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    earn_rime(&mut run);
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 49184, Some(target), false)
        .expect("earned Rime admits HB without waiting for resources");
    run.app.update();
    assert!(
        !auras::has_aura(run.app.world(), run.actor, 59052),
        "one actual HB must spend its earned Rime, not leave fifteen seconds of free casts"
    );
}

#[test]
#[ignore = "native251 Rime HB applies Frostbound Will's native recharge adjustment"]
fn frost_rime_howling_blast_reduces_empower_recharge() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    earn_rime(&mut run);
    let milliseconds = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1238680)
        .expect("selected native Frostbound Will")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(milliseconds < 0.0);
    cast(&mut run, 47568);
    let before = erw_full_recharge(&run);
    assert!(before > f64::from(-milliseconds) / 1000.0);
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 49184, Some(target), false)
        .unwrap();
    let after = erw_full_recharge(&run);
    let expected = before + f64::from(milliseconds) / 1000.0;
    assert!(
        (after - expected).abs() < 0.002,
        "native Rime HB recharge {before} -> {after}, expected {expected}"
    );
}
