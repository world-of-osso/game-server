//! Native earned-resource and proc-recipient boundaries for the pinned263 profile.
use super::*;
use crate::auras::ActiveAuras;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use server::simc_apl::Facts;

fn cast(run: &mut Run, spell: u32, target: Entity) {
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => {
                preparation::finish_pending_setup(run).unwrap();
                return;
            }
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("native generation fixture {spell}: {reason:?}"),
        }
    }
    panic!("native generation fixture {spell} did not become ready");
}

fn stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 344179 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

fn full_recharge(run: &Run) -> f64 {
    let bindings = Bindings {
        actions: [("stormstrike".into(), 17364)].into(),
        global_cooldown_spell: 17364,
        ..Default::default()
    };
    NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        1,
    )
    .value("cooldown.stormstrike.full_recharge_time")
    .unwrap()
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_voltaic_blaze_does_not_trigger_ascendance_only_descending_skies() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(1264688)
    );
    engage(&mut run);
    let target = run.targets[0];
    cast(&mut run, 470057, target);
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 454015),
        "neither Ascendance-only Descending Skies nor strike-only Awakening Storms applies to Voltaic Blaze"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_doom_winds_static_accumulation_earns_native_stacks_without_melee() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    let actor = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), actor, 384411));
    cast(&mut run, 384352, actor);
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 384437),
        "native mode owns its periodic aura"
    );
    assert!(
        run.app
            .world()
            .get::<crate::combat::AutoAttack>(actor)
            .is_none()
    );
    for _ in 0..60 {
        run.app.update();
    }
    assert!(
        stacks(&run) > 0,
        "real Static Accumulation ticks must fund the native MW counter"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_elemental_assault_funds_every_ordinary_stormstrike() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    for _ in 0..4 {
        let before = stacks(&run);
        cast(&mut run, 17364, target);
        assert!(
            stacks(&run) > before,
            "selected native100% Elemental Assault grants its real one-stack effect on every root"
        );
        cast(&mut run, 188196, target);
    }
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_elemental_tempo_reduces_running_strike_recharge_per_spent_stack() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    cast(&mut run, 17364, target);
    cast(&mut run, 470057, target);
    let spent = stacks(&run);
    assert!(spent > 0);
    let before = full_recharge(&run);
    assert!(
        before > 0.0,
        "actual Stormstrike must have a missing charge"
    );
    let milliseconds = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1250364)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap()
        .amount;
    let started = crate::spell_cast::now(run.app.world());
    cast(&mut run, 188196, target);
    let elapsed = crate::spell_cast::now(run.app.world()) - started;
    let expected =
        (before - elapsed - f64::from(milliseconds) * f64::from(spent) / 1000.0).max(0.0);
    assert!(
        (full_recharge(&run) - expected).abs() < 0.051,
        "native Tempo must reduce actual recharge beyond elapsed time: before={before}, elapsed={elapsed}, spent={spent}, expected={expected}, actual={}",
        full_recharge(&run)
    );
}
