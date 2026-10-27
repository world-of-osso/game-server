//! SimC76f8118 sc_shaman.cpp4070–4139/4248–4315/8638–8640/8752–8755.
use super::elemental_mechanics_tests::{fixture, own_aura, settle};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::{
    components::{Npc, UnitSummonedBy},
    protocol::CombatLogKind,
};

fn assert_native_elemental(root: u32, talent: u32, entry: u32, attacks: &[u32]) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(
        own_aura(&run, talent).is_some(),
        "selected native elemental-summon talent"
    );
    assert!(
        own_aura(&run, 117013).is_some(),
        "selected native Primal Elementalist"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        root,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    settle(&mut run);
    let elemental = run
        .app
        .world_mut()
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(run.app.world())
        .find(|(_, npc, owner)| npc.template_id == entry && owner.0 == run.actor.to_bits())
        .map(|(entity, _, _)| entity)
        .expect("ordinary native cooldown summons its actual owned Primal Elemental");
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(elemental.to_bits())
                && event.spell_id.is_some_and(|spell| attacks.contains(&spell))
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "actual elemental deals native owned damage"
    );
}

fn owned_actors(run: &mut Run, entry: u32) -> Vec<(Entity, f64)> {
    let owner = run.actor;
    run.app
        .world_mut()
        .query::<(Entity, &Npc, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .filter(|(_, npc, summon)| npc.template_id == entry && summon.owner == owner)
        .map(|(entity, _, summon)| (entity, summon.expires_at.unwrap()))
        .collect()
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_primal_storm_resummon_extends_and_resets_actions() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    // SimC76f8118 sc_shaman.cpp11555–11564: an active elemental is extended by
    // the new duration and every action cooldown resets; no second actor.
    let stormkeeper = |run: &mut Run| {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            191634,
            Some(run.targets[0]),
            1
        ));
    };
    stormkeeper(&mut run);
    for _ in 0..100 {
        run.app.update();
    }
    let first = owned_actors(&mut run, 77942);
    assert_eq!(
        first.len(),
        1,
        "one actual Primal Storm Elemental: {first:?}"
    );
    let (actor, expiry) = first[0];
    let before = actor_casts(&run, actor, &[157348]).len();
    assert_eq!(
        before, 1,
        "Call Lightning cast once, then on its 20s cooldown"
    );
    stormkeeper(&mut run);
    for _ in 0..60 {
        run.app.update();
    }
    let second = owned_actors(&mut run, 77942);
    assert_eq!(
        second.len(),
        1,
        "re-summon keeps the active actor: {second:?}"
    );
    assert_eq!(second[0].0, actor);
    assert!(
        (second[0].1 - (expiry + 10.0)).abs() < 1e-6,
        "active elemental extended by native10s: {expiry} -> {}",
        second[0].1
    );
    assert_eq!(
        actor_casts(&run, actor, &[157348]).len(),
        2,
        "re-summon resets Call Lightning's native cooldown"
    );
}

fn bound_native_guardian(run: &mut Run, entry: u32, summon: u32, spells: &[u32]) -> Entity {
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::guardian::GuardianSpells>()
        .spells
        .insert(entry, spells.to_vec());
    assert!(
        crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            summon,
            Some(run.actor),
            1
        ),
        "native summon must execute"
    );
    for _ in 0..40 {
        run.app.update();
    }
    let creatures: Vec<_> = run
        .app
        .world_mut()
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(run.app.world())
        .map(|(entity, npc, owner)| (entity, npc.template_id, owner.0))
        .collect();
    creatures
        .iter()
        .find(|(_, template, owner)| *template == entry && *owner == run.actor.to_bits())
        .map(|(entity, _, _)| *entity)
        .unwrap_or_else(|| {
            let world = run.app.world();
            let template_exists = world.resource::<crate::world_data::WorldData>().templates.contains_key(&entry);
            let spell = world.resource::<crate::spell_info::SpellStore>().get(summon).unwrap();
            let effects: Vec<_> = spell.effects.iter().map(|e| (e.index, e.effect, e.implicit_targets, e.misc_values)).collect();
            let events: Vec<_> = world.resource::<ObservedCombatEvents>().0.iter().filter(|e| e.source == Some(run.actor.to_bits())).map(|e| (e.kind, e.spell_id)).collect();
            panic!("actual guardian {entry}; template={template_exists} effects={effects:?} owner events={events:?}; creatures={creatures:?}")
        })
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_primal_storm_action_requires_call_lightning() {
    let mut run = fixture();
    // SimC76f8118 sc_shaman.cpp4275–4285: Stormfury is ineligible until
    // Call Lightning has at least ten seconds remaining.
    let actor = bound_native_guardian(&mut run, 77942, 157319, &[157375, 157348, 157331]);
    for _ in 0..80 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let actions: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event
                    .spell_id
                    .is_some_and(|id| [157375, 157348, 157331].contains(&id))
        })
        .map(|event| event.spell_id.unwrap())
        .collect();
    assert_eq!(
        actions.first(),
        Some(&157348),
        "Call Lightning must precede Stormfury: {actions:?}"
    );
}

fn actor_casts(run: &Run, actor: Entity, spells: &[u32]) -> Vec<u32> {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id.is_some_and(|id| spells.contains(&id))
        })
        .map(|event| event.spell_id.unwrap())
        .collect()
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_primal_fire_pays_native_mana_for_its_actions() {
    let mut run = fixture();
    // Native61029 is unit_class2/ManaModifier41; Immolate costs10 and
    // Fire Blast4 native mana. TC Guardian::InitStatsForLevel supplies its pool.
    let actor = bound_native_guardian(&mut run, 61029, 118291, &[118297, 57984]);
    for _ in 0..200 {
        run.app.update();
    }
    let casts = actor_casts(&run, actor, &[118297, 57984]);
    assert!(
        casts.contains(&118297) && casts.contains(&57984),
        "real Primal Fire Elemental casts native mana actions: {casts:?}"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_primal_fire_immolate_requires_no_current_dot() {
    let mut run = fixture();
    // SimC76f8118 sc_shaman.cpp4109–4117: do not reapply a ticking Immolate.
    let actor = bound_native_guardian(&mut run, 61029, 118291, &[118297, 57984]);
    // Native Immolate has a 10s cooldown and a 21s duration; cross the cooldown
    // boundary while the first application's damage-over-time effect is active.
    for _ in 0..300 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let casts: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event
                    .spell_id
                    .is_some_and(|id| [118297, 57984].contains(&id))
        })
        .map(|event| event.spell_id.unwrap())
        .collect();
    if casts.is_empty() {
        let result = crate::spell_cast::creature_cast(
            run.app.world_mut(),
            actor,
            118297,
            Some(run.targets[0]),
            false,
        );
        panic!("fire elemental did not cast; direct native Immolate admission={result:?}");
    }
    assert!(
        casts.starts_with(&[118297, 57984]),
        "fire elemental casts Immolate then Fire Blast: {casts:?}"
    );
    assert_eq!(
        casts.iter().filter(|id| **id == 118297).count(),
        1,
        "a ticking Immolate yields to Fire Blast after its cooldown: {casts:?}"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_primal_fire_ascendance_summons_and_attacks() {
    assert_native_elemental(1219480, 378255, 61029, &[117588, 57984, 118297]);
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_primal_storm_stormkeeper_summons_and_attacks() {
    assert_native_elemental(191634, 191717, 77942, &[157331, 157348, 269005]);
}
