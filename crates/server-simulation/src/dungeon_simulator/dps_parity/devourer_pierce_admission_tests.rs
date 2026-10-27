//! Source-selected Pierce the Veil native action admission.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_selected_demonsurge_admits_pierce_in_meta() {
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .is_some_and(|traits| traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == 452402)),
        "profile must select native Demonsurge"
    );
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
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1239525),
        "Hungering Slash must not replace Pierce"
    );
    // SimC76f8118 sc_demon_hunter.cpp5874–5889,11383–11384: selected
    // Demonsurge exposes Pierce the Veil in Devourer Meta.
    let spell = literal_rotation::execution::resolve_action(run.app.world(), actor, 1245483)
        .expect("native Pierce resolution")
        .expect("selected Demonsurge must admit native Pierce");
    assert_eq!(spell, 1245483, "native Pierce without its replacement");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(spell, target, Some(target)),
    )
    .expect("real native Pierce admission");
    for _ in 0..40 {
        if run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(1245483)
                    && event.kind == CombatLogKind::CastSuccess
            })
        {
            break;
        }
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1245483)
                && event.kind == CombatLogKind::CastSuccess),
        "admitted native Pierce must execute"
    );
    for _ in 0..40 {
        run.app.update();
    }
    // SimC76f8118 cpp5830–5856,5874–5889: Pierce inherits Voidblade's damage child.
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1245414)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(target.to_bits())
                && event.amount > 0),
        "real Pierce must deliver native Voidblade child damage"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn reapers_toll_replacement_delivers_native_damage_in_meta() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .is_some_and(|traits| traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == 452402)),
        "selected Demonsurge"
    );
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
        "native Meta active"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 201453),
        "native pacify expired"
    );
    for aura in [1239525, 1245523] {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, aura, Some(actor), 1);
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1239525),
        "Hungering Slash ready"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1245523),
        "native Pierce replacement active"
    );
    let spell = literal_rotation::execution::resolve_action(run.app.world(), actor, 1245483)
        .expect("Pierce resolution")
        .expect("selected Pierce");
    assert_eq!(spell, 1245470, "native Reaper's Toll replacement");
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        shared::components::PowerType::Fury,
        -1000.0,
    )
    .expect("native Fury pool");
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        shared::components::PowerType::Fury,
        60.0,
    )
    .expect("native Fury pool");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(spell, target, Some(target)),
    )
    .expect("real Reaper's Toll admission");
    for _ in 0..80 {
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
                && event.spell_id == Some(1245470)
                && event.kind == CombatLogKind::CastSuccess),
        "real Reaper's Toll executes"
    );
    // SimC76f8118 cpp6697–6783 executes native Hungering Slash damage as Reaper's Toll.
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1239127)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(target.to_bits())
                && event.amount > 0),
        "Reaper's Toll delivers native target damage"
    );
    // SimC76f8118 cpp6742–6744: the source action executes its native Fury child.
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1239507)
                && event.kind == CombatLogKind::Energize
                && event.amount > 0),
        "real Reaper's Toll executes its native Fury child"
    );
    // SimC76f8118 cpp6699–6717,6739–6742: the damage child spawns the parent effect1 count.
    assert!(
        server::soul_fragments::active_count(
            run.app.world(),
            actor,
            server::soul_fragments::FragmentFilter::Lesser
        ) >= 2,
        "real Reaper's Toll spawns its two native lesser fragments"
    );
}
