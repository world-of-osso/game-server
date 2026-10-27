//! Native Reap and Cull fragment consumption and delayed damage boundaries.
use super::*;
use server::soul_fragments::{self, FragmentFilter, FragmentKind};
use shared::protocol::CombatLogKind;

fn spawn_active_fragments(run: &mut Run) -> usize {
    let actor = run.actor;
    for _ in 0..2 {
        soul_fragments::spawn(run.app.world_mut(), actor, FragmentKind::Lesser, false)
            .expect("native lesser fragment");
    }
    for _ in 0..40 {
        if soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser) > 0 {
            break;
        }
        run.app.update();
    }
    let count = soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser);
    assert!(
        count > 0,
        "real lesser fragments must be active before the cast"
    );
    count
}

fn observed(run: &Run, spell: u32, kind: CombatLogKind, positive: bool) -> bool {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == kind
                && (!positive || event.amount > 0)
        })
}

fn assert_fragment_damage_boundary(run: &mut Run, before: usize, root: u32, damage: u32) {
    assert!(
        observed(run, root, CombatLogKind::CastSuccess, false),
        "native action must execute before checking consumption"
    );
    let after = soul_fragments::active_count(run.app.world(), run.actor, FragmentFilter::Lesser);
    assert!(
        after < before,
        "native action must consume actual lesser fragments: {before} -> {after}"
    );
    for _ in 0..40 {
        run.app.update();
        if observed(run, damage, CombatLogKind::Damage, true) {
            break;
        }
    }
    assert!(
        observed(run, damage, CombatLogKind::Damage, true),
        "native delayed damage must land"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn batch3_reap_damage_hits_produce_shattered_souls() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    // Keep Entropy from independently producing a fragment at combat entry.
    let spells: Vec<_> = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(actor)
        .unwrap()
        .iter()
        .filter(|&spell| spell != 1261684)
        .collect();
    run.app
        .world_mut()
        .entity_mut(actor)
        .insert(crate::class_progression::KnownSpells::from_baseline(spells));
    engage(&mut run);
    for _ in 0..100 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1225823, Some(target), 1);
    }
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        observed(&run, 1225823, CombatLogKind::Damage, true),
        "real Reap damage hits"
    );
    assert!(
        soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser) > 0,
        "seeded Reap hits produce lesser fragments"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn batch7_reap_consumes_moment_of_craving() {
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    spawn_active_fragments(&mut run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1238495, Some(actor), 1);
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1238495),
        "native Moment active"
    );
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1226019,
        Some(target),
    )
    .expect("native Reap");
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1238495),
        "Reap consumes Moment"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_reap_consumes_active_lesser_fragments_and_hits() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    let before = spawn_active_fragments(&mut run);
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1226019,
        Some(target),
    )
    .expect("real native Reap admission");
    // SimC76f8118 sc_demon_hunter.cpp5981–6018,6081–6107: Reap consumes
    // active lesser fragments on execution and deals delayed native1225823 damage.
    assert_fragment_damage_boundary(&mut run, before, 1226019, 1225823);
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_cull_consumes_active_lesser_fragments_and_hits() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..30 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607)
            && !crate::auras::has_aura(run.app.world(), actor, 201453)
        {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "native Meta must be active"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 201453),
        "native Meta pacify must expire"
    );
    let spell = literal_rotation::execution::resolve_action(run.app.world(), actor, 1226019)
        .expect("native Reap resolution")
        .expect("native Reap known");
    assert_eq!(spell, 1245453, "native Meta must replace Reap with Cull");
    let before = spawn_active_fragments(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(spell, target, Some(target)),
    )
    .expect("real native Cull admission");
    // SimC76f8118 sc_demon_hunter.cpp6081–6107: Cull inherits the Reap
    // fragment boundary and uses the native1245455 damage child.
    assert_fragment_damage_boundary(&mut run, before, 1245453, 1245455);
}
