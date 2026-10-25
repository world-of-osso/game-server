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
    super::super::engage(&mut run);
    cast(&mut run, 49020);
    cast(&mut run, 49020);
    let actor = run.actor;
    super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
    assert!(auras::has_aura(run.app.world(), actor, 281238));
    assert!(auras::has_aura(run.app.world(), actor, 51271));
    clear_proc(&mut run, 51124);
    cast(&mut run, 49143);
    assert!(
        auras::has_aura(run.app.world(), actor, 51124),
        "successful Frost Strike during actual Pillar must earn native Killing Machine"
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
