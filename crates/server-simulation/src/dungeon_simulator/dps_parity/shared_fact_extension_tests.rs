//! Shared native fact gaps: real Feral aura/cast/proc state, no granted buffs or powers.
use super::literal_rotation::capture_fixture;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::casting::CastState;

const TIGERS_FURY: u32 = 5217;
const OMEN_OF_CLARITY: u32 = 16864;
const CLEARCASTING: u32 = 135700;
// Pinned player.cpp1017–1020/7763–7766: source default reaction-time upper bound.
const SOURCE_REACTION_MAX_SECONDS: f64 = 1.4;

fn encounter(world: &World) -> Encounter {
    Encounter::Fixture {
        started_at: crate::spell_cast::now(world),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

fn feral() -> (App, Entity, Bindings) {
    let mut app = super::super::runtime::build_app();
    let copy = super::super::runtime::create_copy(&mut app);
    let actor = crate::followers::simulator::spawn_parity_attacker(
        app.world_mut(),
        103,
        90,
        copy,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    let clock = encounter(app.world());
    capture_fixture(app.world_mut(), actor, clock).unwrap();
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(crate::creature_ai::CreatureState::Idle);
    for _ in 0..4 {
        app.update();
    }
    let mut bindings = Bindings::default();
    bindings.buffs.insert("clearcasting".into(), CLEARCASTING);
    bindings.buffs.insert("tigers_fury".into(), TIGERS_FURY);
    (app, actor, bindings)
}

fn buff<'a>(world: &'a World, actor: Entity, spell: u32) -> Option<&'a crate::auras::AuraInstance> {
    world
        .get::<crate::auras::ActiveAuras>(actor)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == actor)
}

fn fact(world: &World, actor: Entity, bindings: &Bindings, name: &str) -> f64 {
    NativeFacts::new(world, actor, bindings, encounter(world), 1)
        .value(name)
        .unwrap()
}

#[test]
#[ignore = "shared native buff.react RED: genuine Feral absent-aura state"]
fn native_apl_shared_buff_react_absence_is_zero_without_granting_clearcasting() {
    let (app, actor, bindings) = feral();
    assert!(buff(app.world(), actor, CLEARCASTING).is_none());
    assert_eq!(
        fact(app.world(), actor, &bindings, "buff.clearcasting.up"),
        0.0
    );
    assert_eq!(
        fact(app.world(), actor, &bindings, "buff.clearcasting.react"),
        0.0
    );
}

#[test]
#[ignore = "shared native buff.duration RED: actual TigerFury, not remaining time"]
fn native_apl_shared_buff_duration_retains_native_full_duration_as_remaining_time_declines() {
    let (mut app, actor, bindings) = feral();
    let mut started = false;
    for _ in 0..6000 {
        match crate::spell_cast::creature_cast(
            app.world_mut(),
            actor,
            TIGERS_FURY,
            Some(actor),
            false,
        ) {
            Ok(()) => {
                started = true;
                break;
            }
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => app.update(),
            Err(reason) => panic!("ordinary native TigerFury setup: {reason:?}"),
        }
    }
    assert!(started, "native TigerFury admission never became ready");
    let aura =
        buff(app.world(), actor, TIGERS_FURY).expect("actual native self cast applied TigerFury");
    let full = f64::from(aura.duration_ms) / 1000.0;
    let expiry = aura.expires_at.expect("native TigerFury is finite");
    assert!(full > 0.0);
    assert_eq!(
        fact(app.world(), actor, &bindings, "buff.tigers_fury.duration"),
        full
    );
    let first_remaining = fact(app.world(), actor, &bindings, "buff.tigers_fury.remains");
    app.update();
    assert!(fact(app.world(), actor, &bindings, "buff.tigers_fury.remains") < first_remaining);
    assert_eq!(
        fact(app.world(), actor, &bindings, "buff.tigers_fury.duration"),
        full
    );
    while crate::spell_cast::now(app.world()) <= expiry {
        app.update();
    }
    assert!(buff(app.world(), actor, TIGERS_FURY).is_none());
    assert_eq!(
        fact(app.world(), actor, &bindings, "buff.tigers_fury.duration"),
        full
    );
}

#[test]
#[ignore = "shared native buff.react RED: earn Feral proc and observe reaction/expiry"]
fn native_apl_shared_buff_react_uses_earned_proc_deadline_not_buff_presence() {
    let (mut app, actor, bindings) = feral();
    assert!(
        crate::auras::has_aura(app.world(), actor, OMEN_OF_CLARITY),
        "real Feral Omen must be installed before proc qualification"
    );
    let target = super::spawn_dummy(&mut app, actor, 0).unwrap();
    super::literal_rotation::execution::execute(
        app.world_mut(),
        actor,
        super::literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
    for _ in 0..12000 {
        app.update();
        if buff(app.world(), actor, CLEARCASTING).is_some() {
            break;
        }
    }
    let aura = buff(app.world(), actor, CLEARCASTING)
        .expect("ordinary native white hits must earn Feral Clearcasting");
    let expiry = aura.expires_at.expect("native Clearcasting is finite");
    let stacks = f64::from(aura.stacks);
    let acquired = crate::spell_cast::now(app.world());
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::combat::AutoAttack>()
        .insert(crate::creature_ai::CreatureState::Idle);
    assert_eq!(
        fact(app.world(), actor, &bindings, "buff.clearcasting.up"),
        1.0
    );
    assert_eq!(
        fact(app.world(), actor, &bindings, "buff.clearcasting.react"),
        0.0
    );
    while crate::spell_cast::now(app.world()) <= acquired + SOURCE_REACTION_MAX_SECONDS {
        app.update();
    }
    assert!(buff(app.world(), actor, CLEARCASTING).is_some());
    assert_eq!(
        fact(app.world(), actor, &bindings, "buff.clearcasting.react"),
        stacks
    );
    while crate::spell_cast::now(app.world()) <= expiry {
        app.update();
    }
    assert_eq!(
        fact(app.world(), actor, &bindings, "buff.clearcasting.react"),
        0.0
    );
}

#[test]
#[ignore = "shared named execute_time RED: native rated cast/channel duration, no second formula"]
fn native_apl_shared_named_execute_time_matches_real_hasted_cast_and_channel() {
    for (name, spell) in [("living_flame", 361469), ("disintegrate", 356995)] {
        let mut run = super::prepare(1467, 1).unwrap();
        let clock = encounter(run.app.world());
        capture_fixture(run.app.world_mut(), run.actor, clock).unwrap();
        run.app
            .world_mut()
            .entity_mut(run.actor)
            .remove::<crate::creature_ai::FollowOwner>()
            .insert(crate::creature_ai::CreatureState::Idle);
        run.app.update();
        assert!(
            run.app
                .world()
                .get::<crate::stats::CharacterStats>(run.actor)
                .unwrap()
                .haste_pct
                > 0.0,
            "actual reference gear supplies native haste"
        );
        let mut bindings = Bindings::default();
        bindings.actions.insert(name.into(), spell);
        let action = Action {
            name: name.into(),
            target: None,
            options: Default::default(),
        };
        let mut started = false;
        for _ in 0..6000 {
            match crate::spell_cast::creature_cast(
                run.app.world_mut(),
                run.actor,
                spell,
                Some(run.targets[0]),
                false,
            ) {
                Ok(()) => {
                    started = true;
                    break;
                }
                Err(
                    crate::spell_cast::CastFailure::OnGlobalCooldown
                    | crate::spell_cast::CastFailure::InProgress,
                ) => run.app.update(),
                Err(reason) => panic!("ordinary native {name} setup: {reason:?}"),
            }
        }
        assert!(started);
        let world = run.app.world();
        let cast = world
            .get::<CastState>(run.actor)
            .expect("real native cast/channel exists");
        let metadata = world
            .resource::<crate::spell_info::SpellStore>()
            .get(spell)
            .unwrap();
        let gcd = crate::spell_cast::native_apl::gcd_value(world, run.actor, metadata, "gcd.max")
            .unwrap();
        let actual = if name == "disintegrate" {
            f64::from(cast.duration)
        } else {
            f64::from(cast.duration).max(gcd)
        };
        let facts = NativeFacts::new(world, run.actor, &bindings, clock, 1);
        let named = facts.value(&format!("action.{name}.execute_time")).unwrap();
        assert!(
            (named - actual).abs() < 0.001,
            "{name}: named{named}, actual{actual}"
        );
        assert_eq!(named, facts.value_for(&action, "execute_time").unwrap());
    }
}
