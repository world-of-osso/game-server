//! Native468695 chance and actual Austin Auto Shot/free Cobra provenance.
//! SimC76f8118 sc_hunter.cpp3988-4007/5041-5047: roll on execute, free background Cobra.
use super::*;
use crate::auras::RemoveReason;
use crate::spell_cast::test_harness::{ObservedCombatEvents, ObservedTriggeredCasts};
use shared::protocol::CombatLogKind;

const SNAKESKIN_QUIVER: u32 = 468695;
const AUTO_SHOT: u32 = 75;
const COBRA_SHOT: u32 = 193455;

struct CobraSources {
    auto_shots: usize,
    paid_cobras: usize,
    free_targets: Vec<Option<Entity>>,
}

fn prepare_austin() -> Run {
    let (profile, traits) = read_reference(253).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    assert!(crate::auras::has_aura(
        run.app.world(),
        run.actor,
        SNAKESKIN_QUIVER
    ));
    let native_chance = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SNAKESKIN_QUIVER)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert_eq!(
        native_chance, 15.0,
        "native70448 learned Snakeskin chance, not a forced proc"
    );
    run.app.init_resource::<ObservedTriggeredCasts>();
    run
}

fn count_casts(events: &[shared::protocol::CombatLogEvent], actor: Entity, spell: u32) -> usize {
    events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::CastSuccess
                && event.source == Some(actor.to_bits())
                && event.spell_id == Some(spell)
        })
        .count()
}

fn run_and_count_cobras(run: &mut Run) -> CobraSources {
    engage(run);
    let (seconds, _, _) = advance_combat(run).unwrap();
    assert!((seconds - 300.0).abs() < 0.001);
    let world = run.app.world();
    let events = &world.resource::<ObservedCombatEvents>().0;
    let free_targets: Vec<_> = world
        .resource::<ObservedTriggeredCasts>()
        .0
        .iter()
        .filter(|(caster, spell, _)| *caster == run.actor && *spell == COBRA_SHOT)
        .map(|(_, _, target)| *target)
        .collect();
    let sources = CobraSources {
        auto_shots: count_casts(events, run.actor, AUTO_SHOT),
        paid_cobras: count_casts(events, run.actor, COBRA_SHOT)
            .checked_sub(free_targets.len())
            .expect("actual triggered Cobra launches must also log CastSuccess"),
        free_targets,
    };
    eprintln!(
        "NATIVE_SNAKESKIN actor={} target={} shots={} paid_cobras={} free_cobras={}",
        run.actor.to_bits(),
        run.targets[0].to_bits(),
        sources.auto_shots,
        sources.paid_cobras,
        sources.free_targets.len()
    );
    assert!(
        sources.auto_shots > 0 && sources.paid_cobras > 0,
        "actual kit must exercise native ranged timing and normal paid Cobra"
    );
    sources
}

#[test]
#[ignore = "native Snakeskin execution; requires prepared253 gear/traits/current canonical"]
fn dps_parity_snakeskin_native_austin_autoshots_launch_free_cobra_at_actual_victim() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_austin();
    let sources = run_and_count_cobras(&mut run);
    assert!(
        !sources.free_targets.is_empty(),
        "native fifteen-percent talent over actual Auto Shot opportunities must launch free Cobra"
    );
    assert!(
        sources.free_targets.len() < sources.auto_shots,
        "native chance must not become a guaranteed shot"
    );
    assert!(
        sources
            .free_targets
            .iter()
            .all(|target| *target == Some(run.targets[0])),
        "free Cobra retains the actual successful Auto Shot victim"
    );
}

#[test]
#[ignore = "native inactive Snakeskin control; requires prepared253 gear/traits/current canonical"]
fn dps_parity_snakeskin_native_removed_passive_keeps_paid_cobra_without_free_shots() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_austin();
    let actor = run.actor;
    let removed = crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == SNAKESKIN_QUIVER,
    );
    assert_eq!(removed.len(), 1);
    let sources = run_and_count_cobras(&mut run);
    assert!(!crate::auras::has_aura(
        run.app.world(),
        actor,
        SNAKESKIN_QUIVER
    ));
    assert!(
        sources.free_targets.is_empty(),
        "no talent means no free Cobra execution"
    );
}
