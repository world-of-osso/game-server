//! Selected Familiar talents act on ordinary native Arcane Assault executions.
use super::*;
use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Mana;
use shared::protocol::CombatLogKind;

fn selected(run: &Run, spell: u32) -> bool {
    run.app
        .world()
        .get::<TraitLoadout>(run.actor)
        .unwrap()
        .active_talent_spells()
        .any(|talent| talent.spell_id == spell)
}

fn observed_hits(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "requires private native70448 Arcane player profile"]
fn arcane_familiar_energized_native_surge_produces_authored_extra_assaults() {
    // SimC76f8118 sc_mage.cpp6368–6374: each hasted Familiar tick during
    // real Surge executes native452997 effect1 total assaults,75ms apart.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(62, 1).unwrap();
    assert!(
        selected(&run, 452997),
        "actual player profile selects Energized Familiar"
    );
    engage(&mut run);
    for _ in 0..40 {
        run.app.update();
    }
    super::arcane_tests::start_cast(&mut run, 365350);
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(run.actor)
            .unwrap()
            .has_spell(365362),
        "actual native Surge aura"
    );
    let count =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 452997, 0)
            as usize;
    assert!(
        count > 1,
        "authored selected Energized Familiar extra attacks"
    );
    let before = observed_hits(&run, 225119);
    for _ in 0..140 {
        run.app.update();
    }
    let gained = observed_hits(&run, 225119) - before;
    assert!(
        gained >= count * 2,
        "two hasted Familiar ticks during real Surge must produce {count} assaults each; observed{gained}"
    );
}

fn sample_native_assault_cohort() -> (f32, usize) {
    // SimC76f8118 sc_mage.cpp3052–3065: actual Assault rolls native
    // Energized452997 effect2 for native454020 Mana and Attuned1261106
    // effect1 for an owned Splinter. Native chances and fixed seed unchanged.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(62, 1).unwrap();
    assert!(
        selected(&run, 452997) && selected(&run, 1261106),
        "actual profile selects both Familiar talents"
    );
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    run.app.world_mut().get_mut::<Mana>(actor).unwrap().current = 0.0;
    let splinters_before = observed_hits(&run, 443763);
    for _ in 0..80 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 225119, Some(target), 1);
    }
    let mana = run.app.world().get::<Mana>(actor).unwrap().current;
    for _ in 0..100 {
        run.app.update();
    }
    let splinters = observed_hits(&run, 443763) - splinters_before;
    (mana, splinters)
}

#[test]
#[ignore = "requires private native70448 Arcane player profile"]
fn arcane_familiar_native_assault_earns_selected_energized_mana() {
    let (mana, _) = sample_native_assault_cohort();
    assert!(
        mana > 0.0,
        "native selected Energized Familiar must grant actual Mana on the seeded Assault cohort; got{mana}"
    );
}

fn sample_attuned_splinters(include_attuned: bool) -> usize {
    let _seed = crate::random::seed_scope(70448);
    let (profile, mut traits) = read_reference(62).unwrap();
    if !include_attuned {
        // Native active trait entry135920 maps to Attuned Familiar1261106.
        traits.retain(|trait_entry| trait_entry.entry_id != 135920);
    }
    let actor_kind = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_literal_reference(profile, &traits, 1, actor_kind).unwrap();
    assert_eq!(selected(&run, 1261106), include_attuned);
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let before = observed_hits(&run, 443763);
    for _ in 0..80 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 225119, Some(target), 1);
    }
    for _ in 0..100 {
        run.app.update();
    }
    observed_hits(&run, 443763) - before
}

#[test]
#[ignore = "requires private native70448 Arcane player profile"]
fn arcane_familiar_native_attuned_selection_adds_owned_splinters() {
    // A positive-only Splinter assertion passed without an Attuned producer;
    // compare real profile allocations with the same seeded native cohort.
    let without = sample_attuned_splinters(false);
    let with = sample_attuned_splinters(true);
    assert!(
        with > without,
        "native Attuned selection must add owned Splinters on actual Assaults: selected{with}, unselected{without}"
    );
}
