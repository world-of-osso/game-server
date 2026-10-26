//! Pinned SimC strike relationships must execute through real native casts.

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_selected_unruly_winds_always_adds_its_native_third_attack() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(263, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(390288)
    );
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    let mut before = 0;
    let mut procs = 0;
    for _ in 0..6000 {
        run.app.update();
        let current = damage_count(&run, 25504);
        if current == before {
            continue;
        }
        assert_eq!(
            current - before,
            3,
            "selected390288 native100% must add the third attack, not384355 Elemental Weapons50%"
        );
        before = current;
        procs += 1;
        if procs == 8 {
            break;
        }
    }
    assert_eq!(procs, 8, "eight actual native Windfury procs must occur");
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_tempest_requires_its_actual_proc_not_only_the_selected_talent() {
    let mut run = prepare_literal(263, 1).unwrap();
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 454015));
    let target = run.targets[0];
    let result = crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        452201,
        Some(target),
        false,
    );
    assert!(
        matches!(result, Err(crate::spell_cast::CastFailure::CasterAuraState)),
        "Tempest must require the native replacement proc: {result:?}"
    );
}
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn launch(run: &mut Run, spell: u32, target: Entity) {
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => return,
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("native strike fixture {spell}: {reason:?}"),
        }
    }
    panic!("native strike fixture {spell} did not become ready");
}

fn cast(run: &mut Run, spell: u32, target: Entity) {
    launch(run, spell, target);
    preparation::finish_pending_setup(run).unwrap();
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

fn damage_count(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_stormstrike_triggers_one_native_crash_lightning_payload() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    cast(&mut run, 187874, target);
    let storm_unleashed = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap()
        .contains(1262713);
    let buff = if storm_unleashed { 1252415 } else { 187878 };
    assert!(crate::auras::has_aura(run.app.world(), run.actor, buff));
    let before = damage_count(&run, 195592);
    launch(&mut run, 17364, target);
    // SimC76f8118 sc_shaman.cpp5597–5598/4536–4598: this root owns one
    // immediate payload; later Stormflurry executions own their own payloads.
    assert_eq!(
        damage_count(&run, 195592) - before,
        1,
        "one crash payload per landed root, not per weapon hand"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        damage_count(&run, 32175) > 0,
        "actual Stormstrike hit prerequisite"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_doom_winds_stormstrike_invokes_actual_lightning_and_spends_earned_maelstrom() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    let actor = run.actor;
    cast(&mut run, 470057, target);
    assert!(
        stacks(&run) > 0,
        "real Voltaic Blaze earned the native counter"
    );
    cast(&mut run, 384352, actor);
    assert!(crate::auras::has_aura(run.app.world(), actor, 466772));
    let payload = if crate::auras::has_aura(run.app.world(), actor, 454015) {
        452201
    } else {
        188196
    };
    let before = damage_count(&run, payload);
    let first_event = run.app.world().resource::<ObservedCombatEvents>().0.len();
    cast(&mut run, 17364, target);
    assert!(
        damage_count(&run, 32175) > 0,
        "actual Stormstrike hit prerequisite"
    );
    assert_eq!(
        damage_count(&run, payload) - before,
        1,
        "Thorim invokes actual Tempest when earned, otherwise its default native Bolt"
    );
    let consumed = run.app.world().resource::<ObservedCombatEvents>().0[first_event..]
        .iter()
        .any(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(344179)
                && event.kind == CombatLogKind::AuraRemoved
        });
    assert!(
        consumed,
        "background invocation spends the old counter; later weapon procs may earn a fresh one"
    );
}
