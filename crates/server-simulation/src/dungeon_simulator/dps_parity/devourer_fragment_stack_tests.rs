//! Native fragment heal impact and Devourer stack production.
use super::*;
use server::soul_fragments::{self, FragmentFilter, FragmentKind};
use shared::{components::Health, protocol::CombatLogKind};

fn stacks(world: &World, actor: Entity, spell: u32) -> u8 {
    world
        .get::<crate::auras::ActiveAuras>(actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == actor)
                .map(|aura| aura.stacks)
        })
        .unwrap_or(0)
}

fn consume_fragment_and_observe_heal(run: &mut Run) {
    let actor = run.actor;
    {
        let mut health = run
            .app
            .world_mut()
            .get_mut::<Health>(actor)
            .expect("native health");
        health.current = health.max * 0.5;
    }
    soul_fragments::spawn(run.app.world_mut(), actor, FragmentKind::Lesser, false)
        .expect("native lesser fragment");
    for _ in 0..40 {
        if soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser) > 0 {
            break;
        }
        run.app.update();
    }
    assert!(
        soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser) > 0,
        "native fragment must become active"
    );
    assert_eq!(
        soul_fragments::consume_active(
            run.app.world_mut(),
            actor,
            FragmentFilter::Lesser,
            1,
            false
        )
        .expect("consume native fragment"),
        1
    );
    for _ in 0..200 {
        run.app.update();
        if has_fragment_heal(run) {
            break;
        }
    }
    assert!(
        has_fragment_heal(run),
        "native fragment heal must land before checking its side effect"
    );
}

fn has_fragment_heal(run: &Run) -> bool {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1266301)
                && event.kind == CombatLogKind::Heal
                && event.amount > 0
        })
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn batch1_fragment_heal_grants_feast_of_souls() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    assert!(run.app.world().get::<crate::class_progression::KnownSpells>(actor).is_some_and(|known| known.contains(1237270)), "selected Feast");
    engage(&mut run);
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1232310));
    consume_fragment_and_observe_heal(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), actor, 1232310), "real fragment heal grants Feast");
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn batch1_real_star_hit_grants_delayed_impending_apocalypse() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    assert!(run.app.world().get::<crate::class_progression::KnownSpells>(actor).is_some_and(|known| known.contains(1227707)), "selected Impending Apocalypse");
    engage(&mut run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..30 { run.app.update(); }
    for _ in 0..30 { crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1227702, Some(actor), 1); }
    assert!(crate::auras::has_aura(run.app.world(), actor, 1221171), "star ready");
    crate::spell_cast::cast_for_player_with_target(run.app.world_mut(), actor, 1221150, Some(target)).expect("real Star admission");
    for _ in 0..100 { run.app.update(); }
    assert!(run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter().any(|e| e.spell_id == Some(1221162) && e.target == Some(target.to_bits()) && e.kind == CombatLogKind::Damage && e.amount > 0), "primary Star hit landed");
    assert!(crate::auras::has_aura(run.app.world(), actor, 1227338), "Star hit grants delayed Impending Apocalypse");
}

fn batch1_voidblade_hit(meta: bool, mastery: f32) -> (i32, bool) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    if meta { crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1); }
    for _ in 0..30 { run.app.update(); }
    assert_eq!(crate::auras::has_aura(run.app.world(), actor, 1217607), meta, "native Meta state");
    let mut stats = run.app.world_mut().get_mut::<crate::stats::CharacterStats>(actor).unwrap();
    stats.mastery_points = mastery;
    stats.mastery_pct = mastery;
    let _hit_seed = crate::random::seed_scope(91);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1245414, Some(target), 1);
    let hit = run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter().find(|e| e.spell_id == Some(1245414) && e.target == Some(target.to_bits()) && e.kind == CombatLogKind::Damage && e.amount > 0).expect("real Voidblade child hit");
    (hit.amount, hit.crit)
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn batch1_meta_scales_devourer_damage_with_mastery() {
    let (zero, zero_crit) = batch1_voidblade_hit(true, 0.0);
    let (forty, forty_crit) = batch1_voidblade_hit(true, 40.0);
    assert_eq!(zero_crit, forty_crit, "compare same hit outcome");
    // SimC76f8118 cpp2169–2185 pairs Meta effect4 with Monster Within mastery.
    let expected = f64::from(zero) * 1.16;
    assert!((f64::from(forty)-expected).abs() < expected * 0.02, "Meta/mastery expected {expected}, actual {forty}, zero-mastery hit {zero}");
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_fragment_heal_increments_meta_stacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1217607),
        "out-of-Meta fixture"
    );
    let initial = stacks(run.app.world(), actor, 1225789);
    consume_fragment_and_observe_heal(&mut run);
    // SimC76f8118 sc_demon_hunter.cpp3678–3692 triggers the stack at heal impact.
    assert_eq!(
        stacks(run.app.world(), actor, 1225789),
        initial + 1,
        "one consumed fragment must add one Meta stack"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_collapsing_star_threshold_grants_ready_buff() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1221167)),
        "selected Collapsing Star talent"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "native Meta active"
    );
    let threshold =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 1221167, 0) as u8;
    assert_eq!(threshold, 30, "native selected talent threshold");
    for _ in 0..threshold - 1 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1227702, Some(actor), 1);
    }
    for _ in 0..20 {
        if stacks(run.app.world(), actor, 1227702) == threshold - 1 {
            break;
        }
        run.app.update();
    }
    assert_eq!(
        stacks(run.app.world(), actor, 1227702),
        threshold - 1,
        "native stacks prepared below threshold"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1221171),
        "ready buff absent below threshold"
    );
    consume_fragment_and_observe_heal(&mut run);
    assert_eq!(
        stacks(run.app.world(), actor, 1227702),
        threshold,
        "real fragment heal crosses threshold"
    );
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1221171) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1221171),
        "crossing threshold grants ready buff"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_meta_fragment_heal_increments_collapsing_star() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1221167)),
        "selected native Collapsing Star talent"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "native Meta must be active"
    );
    let initial_star = stacks(run.app.world(), actor, 1227702);
    let initial_meta = stacks(run.app.world(), actor, 1225789);
    consume_fragment_and_observe_heal(&mut run);
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "Meta must remain active through the heal"
    );
    // SimC76f8118 sc_demon_hunter.cpp3691–3698 uses the selected Collapsing Star stack in Meta.
    assert_eq!(
        stacks(run.app.world(), actor, 1227702),
        initial_star + 1,
        "one consumed fragment must add one Collapsing Star stack"
    );
    assert_eq!(
        stacks(run.app.world(), actor, 1225789),
        initial_meta,
        "Meta fragments must not add out-of-Meta stacks"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_full_soul_stacks_admit_real_metamorphosis() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1217607),
        "outside Meta"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1225789, Some(actor), 1);
    let cap = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 1225789 && aura.caster == actor)
        })
        .expect("native soul stack aura")
        .max_stacks;
    assert_eq!(cap, 35, "selected Soul Glutton modifies the native cap");
    for _ in 1..cap - 1 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1225789, Some(actor), 1);
    }
    for _ in 0..20 {
        if stacks(run.app.world(), actor, 1225789) == cap - 1 {
            break;
        }
        run.app.update();
    }
    assert_eq!(
        stacks(run.app.world(), actor, 1225789),
        cap - 1,
        "native stacks prepared below effective cap"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1213809),
        "Meta not ready before cap"
    );
    consume_fragment_and_observe_heal(&mut run);
    assert_eq!(
        stacks(run.app.world(), actor, 1225789),
        cap,
        "real fragment heal reaches native cap"
    );
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1213809) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1213809),
        "full native soul stacks grant Meta admission aura"
    );
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1217605,
        Some(actor),
    )
    .expect("full stacks admit ordinary Meta cast");
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "ordinary Meta starts native aura"
    );
    // SimC76f8118 sc_demon_hunter.cpp9192–9195 expires soul stacks on Meta entry.
    assert_eq!(
        stacks(run.app.world(), actor, 1225789),
        0,
        "Meta entry consumes its soul stacks"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1213809),
        "Meta entry removes its ready aura"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn real_collapsing_star_spends_stacks_and_ready_buff() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 3).expect("native Devourer profile");
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
        "real native Meta active"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 201453),
        "pacify expired"
    );
    for _ in 0..35 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1227702, Some(actor), 1);
    }
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1221171) {
            break;
        }
        run.app.update();
    }
    assert_eq!(
        stacks(run.app.world(), actor, 1227702),
        35,
        "native 35 Collapsing Star stacks"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1221171),
        "native Collapsing Star ready"
    );
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1221150,
        Some(target),
    )
    .expect("real Collapsing Star admission");
    for _ in 0..100 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1221150)
                && event.kind == CombatLogKind::CastSuccess),
        "real Collapsing Star executed"
    );
    // SimC76f8118 cpp6529–6553 expires ready and decrements 30 selected souls on execute.
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1221171),
        "Collapsing Star consumes its ready buff"
    );
    assert_eq!(
        stacks(run.app.world(), actor, 1227702),
        5,
        "Collapsing Star spends 30 souls and preserves remainder"
    );
    // SimC76f8118 cpp6464–6474,6532–6533 executes native Collapsing Star damage.
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1221162)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(target.to_bits())
                && event.amount > 0),
        "real Collapsing Star delivers native damage to its target"
    );
    let hits: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(1221162)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect();
    let primary = hits
        .iter()
        .find(|event| event.target == Some(target.to_bits()))
        .expect("primary hit");
    assert!(
        !primary.crit,
        "seeded primary must be noncritical for the cleave comparison"
    );
    let secondary = hits
        .iter()
        .find(|event| event.target != Some(target.to_bits()) && !event.crit)
        .expect("seeded noncritical secondary hit");
    // SimC76f8118 cpp6488–6495: secondary damage is native parent effect2's 50%.
    let ratio = secondary.amount as f64 / primary.amount as f64;
    assert!(
        (ratio - 0.5).abs() < 0.02,
        "secondary/primary damage ratio: {ratio}, hits={hits:?}"
    );
}
