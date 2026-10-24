//! Native Player facts: actual selected traits, recipient HP and launched travel.
use super::super::{Run, literal_rotation::equipment::Equipment, prepare_literal};
use super::{bindings, facts::FireFacts};
use crate::simc_apl_native::{Encounter, FightStyle};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use bevy::prelude::Entity;
use server::simc_apl::{Action, ActionFacts};
use shared::casting::CastState;
use shared::components::{Health, Player, Position};
use shared::protocol::CombatLogKind;

const FIREBALL: u32 = 133;
const PYROBLAST: u32 = 11366;
const MAX_CAST_FRAMES: usize = 400;

fn native_player() -> Run {
    let run = prepare_literal(63, 2).expect("current native90 Fire profile/private DB");
    assert!(run.app.world().get::<Player>(run.actor).is_some());
    run
}

fn value(run: &Run, actor: Entity, target: Entity, name: &str) -> Result<f64, String> {
    let mut bindings = bindings::build();
    let world = run.app.world();
    let equipment = Equipment::load(world, run.actor, &mut bindings)?;
    let facts = FireFacts::new(
        world,
        actor,
        &bindings,
        &equipment,
        Encounter::Fixture {
            started_at: crate::spell_cast::now(world),
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        &run.targets,
    );
    facts.value_for(
        &Action {
            name: "fireball".into(),
            target: Some(target.to_bits()),
            options: Default::default(),
        },
        name,
    )
}

fn set_health(run: &mut Run, target: Entity, percent: f32) {
    let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
    health.current = health.max * percent / 100.0;
}

#[test]
#[ignore = "current native90 Player traits/private DB; native70448 threshold90"]
fn fire_local_native90_firestarter_uses_selected_recipient_hp_boundary() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = native_player();
    let first = run.targets[0];
    let second = run.targets[1];
    for (percent, expected) in [(89.0, 0.0), (90.0, 1.0), (100.0, 1.0)] {
        set_health(&mut run, first, percent);
        assert_eq!(
            value(&run, run.actor, first, "firestarter.active"),
            Ok(expected),
            "recipientHP{percent}"
        );
    }
    set_health(&mut run, first, 89.0);
    set_health(&mut run, second, 100.0);
    assert_eq!(
        value(&run, run.actor, second, "firestarter.active"),
        Ok(1.0),
        "explicit second recipient, not primary target"
    );
}

#[test]
#[ignore = "current native90 Player selected Scorch/private DB; native70448 threshold30"]
fn fire_local_native90_scorch_execute_uses_selected_recipient_hp_boundary() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = native_player();
    let first = run.targets[0];
    let second = run.targets[1];
    for (percent, expected) in [(31.0, 0.0), (30.0, 1.0), (29.0, 1.0)] {
        set_health(&mut run, first, percent);
        assert_eq!(
            value(&run, run.actor, first, "scorch_execute.active"),
            Ok(expected),
            "recipientHP{percent}"
        );
    }
    set_health(&mut run, first, 100.0);
    set_health(&mut run, second, 30.0);
    assert_eq!(
        value(&run, run.actor, second, "scorch_execute.active"),
        Ok(1.0),
        "explicit second recipient, not primary target"
    );
}

fn cast_native(run: &mut Run, spell: u32, target: Entity) {
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, spell, Some(target), false)
        .unwrap_or_else(|error| panic!("ordinary native{spell} admission: {error:?}"));
}

fn finish_cast(run: &mut Run) {
    for _ in 0..MAX_CAST_FRAMES {
        if run.app.world().get::<CastState>(run.actor).is_none() {
            return;
        }
        run.app.update();
    }
    panic!("native hardcast did not finish");
}

fn saw_impact(run: &Run, spell: u32) -> bool {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
}

fn distant_target(run: &mut Run) -> Entity {
    let target = run.targets[0];
    let caster = *run.app.world().get::<Position>(run.actor).unwrap();
    let mut position = run.app.world_mut().get_mut::<Position>(target).unwrap();
    position.x = caster.x + 40.0;
    position.y = caster.y;
    position.z = caster.z;
    target
}

#[test]
#[ignore = "ordinary native Player Pyroblast cast/travel/private DB, no injected pending hits"]
fn fire_local_native90_hot_streak_count_tracks_real_launch_actor_and_impact() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = native_player();
    let target = distant_target(&mut run);
    cast_native(&mut run, PYROBLAST, target);
    assert!(run.app.world().get::<CastState>(run.actor).is_some());
    assert_eq!(
        value(&run, run.actor, target, "hot_streak_spells_in_flight"),
        Ok(0.0),
        "hardcast is not launched travel"
    );
    finish_cast(&mut run);
    assert!(!saw_impact(&run, PYROBLAST));
    assert_eq!(
        value(&run, run.actor, target, "hot_streak_spells_in_flight"),
        Ok(1.0)
    );
    assert_eq!(
        value(&run, run.targets[1], target, "hot_streak_spells_in_flight"),
        Ok(0.0),
        "other native unit owns no projectile"
    );
    for _ in 0..MAX_CAST_FRAMES {
        run.app.update();
    }
    assert!(saw_impact(&run, PYROBLAST));
    assert_eq!(
        value(&run, run.actor, target, "hot_streak_spells_in_flight"),
        Ok(0.0),
        "burns are not pending travel"
    );
}

fn wait_cast_window(run: &mut Run) {
    for _ in 0..MAX_CAST_FRAMES {
        let world = run.app.world();
        let busy = world.get::<CastState>(run.actor).is_some()
            || world
                .get::<crate::spell_cast::SpellHistory>(run.actor)
                .is_some_and(|history| {
                    history.has_active_gcd(crate::spell_cast::history_now(world))
                });
        if !busy {
            return;
        }
        run.app.update();
    }
    panic!("ordinary native cast window did not open");
}

fn has_hot_streak(run: &Run) -> bool {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .any(|aura| aura.spell_id == 48108)
}

fn earn_hot_streak(run: &mut Run) {
    // The nearby recipient remains100% HP; actual Firestarter crits feed the
    // ordinary native Hot Streak driver. No aura, crit or charge grant.
    let nearby = run.targets[1];
    for _ in 0..16 {
        wait_cast_window(run);
        cast_native(run, FIREBALL, nearby);
        finish_cast(run);
        for _ in 0..40 {
            run.app.update();
            if has_hot_streak(run) {
                return;
            }
        }
    }
    panic!("ordinary native Fireballs did not earn Hot Streak");
}

#[test]
#[ignore = "ordinary native Player Fireball/earned Hot Streak Pyro overlapping travel/private DB"]
fn fire_local_native90_hot_streak_count_counts_overlapping_native_events() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = native_player();
    let target = distant_target(&mut run);
    earn_hot_streak(&mut run);
    wait_cast_window(&mut run);
    cast_native(&mut run, FIREBALL, target);
    finish_cast(&mut run);
    wait_cast_window(&mut run);
    assert!(has_hot_streak(&run));
    assert_eq!(
        value(&run, run.actor, target, "hot_streak_spells_in_flight"),
        Ok(1.0),
        "native Fireball remains in flight when the real GCD opens"
    );
    cast_native(&mut run, PYROBLAST, target);
    assert!(
        run.app.world().get::<CastState>(run.actor).is_none(),
        "earned Hot Streak makes the ordinary Pyro instant"
    );
    assert!(
        !has_hot_streak(&run),
        "ordinary Pyro consumes its real native proc"
    );
    assert!(!saw_impact(&run, PYROBLAST));
    assert_eq!(
        value(&run, run.actor, target, "hot_streak_spells_in_flight"),
        Ok(2.0),
        "two real launched events, not a bool"
    );
}
