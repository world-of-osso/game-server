//! SimC76f8118 rogue6103–6125/7419–7444: actual Vanish lifecycle.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[derive(Resource, Default)]
struct VanishRemovals(Vec<crate::auras::RemoveReason>);

#[derive(Resource, Default)]
struct VanishProcs(Vec<(f64, crate::auras::procs::AuraEvent)>);

fn observe_vanish_proc(
    world: &mut World,
    _unit: Entity,
    _aura: &crate::auras::AuraInstance,
    event: &crate::auras::procs::AuraEvent,
) {
    let now = crate::spell_cast::now(world);
    world
        .resource_mut::<VanishProcs>()
        .0
        .push((now, event.clone()));
}

fn observe_vanish_removal(
    world: &mut World,
    _unit: Entity,
    _aura: &crate::auras::AuraInstance,
    reason: crate::auras::RemoveReason,
) {
    world.resource_mut::<VanishRemovals>().0.push(reason);
}

fn prepare_vanish_after_opener() -> Run {
    let mut run = prepare_measurement(259, 1).unwrap();
    run.app.init_resource::<VanishRemovals>();
    run.app.init_resource::<VanishProcs>();
    {
        let mut scripts = run
            .app
            .world_mut()
            .resource_mut::<crate::auras::scripts::AuraScripts>();
        let script = scripts.0.entry(11327).or_default();
        script.on_remove = Some(observe_vanish_removal);
        script.on_damage_proc = Some(observe_vanish_proc);
    }
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(703, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..140 {
        run.app.update();
    }
    run
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn rogue_vanish_native_cast_suspends_weapons_expires_to_stealth_and_earns_garrote_window() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_vanish_after_opener();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(!crate::auras::rogue::improved_garrote_active(
        run.app.world(),
        actor,
        703
    ));
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
            }),
        "real weapon swings before Vanish"
    );
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(1856, actor, Some(target)),
    )
    .unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 11327),
        "successful native1856 must grant its actual11327 Vanish stealth"
    );
    assert!(crate::auras::rogue::improved_garrote_active(
        run.app.world(),
        actor,
        703
    ));
    for _ in 0..80 {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
            }),
        "Vanish cancels weapons; natural expiry must not restart them"
    );
    assert!(!crate::auras::has_aura(run.app.world(), actor, 11327));
    assert_eq!(
        run.app.world().resource::<VanishRemovals>().0,
        [crate::auras::RemoveReason::Expired],
        "Vanish must reject its own native no-stealth-break event and expire naturally; actual native procs{:?}",
        run.app.world().resource::<VanishProcs>().0
    );
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1784),
        "natural Vanish expiry grants actual Stealth, rather than breaking concealment"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 392401),
        "natural expiry must preserve concealment without prematurely earning the post-stealth Garrote window"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(703, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 392401),
        "breaking earned stealth grants the native Improved Garrote window"
    );
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
            }),
        "actual harmful re-entry resumes native weapons"
    );
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn rogue_vanish_native_harmful_cast_breaks_active_vanish_without_later_restealth() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_vanish_after_opener();
    let actor = run.actor;
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(), actor,
        literal_rotation::execution::Execution::Spell(1856, actor, Some(target)),
    ).unwrap();
    run.app.update();
    assert!(crate::auras::has_aura(run.app.world(), actor, 11327), "Vanish must survive its own cast event");
    literal_rotation::execution::execute(
        run.app.world_mut(), actor,
        literal_rotation::execution::Execution::Spell(703, target, Some(target)),
    ).unwrap();
    for _ in 0..80 {
        run.app.update();
    }
    assert_eq!(run.app.world().resource::<VanishRemovals>().0, [crate::auras::RemoveReason::Consumed]);
    assert!(!crate::auras::has_aura(run.app.world(), actor, 11327));
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1784), "harmful re-entry must not re-stealth at the old Vanish expiry");
    assert!(crate::auras::has_aura(run.app.world(), actor, 392401), "harmful re-entry earns the real post-stealth Garrote window");
}
