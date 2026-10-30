//! Native Elemental Lightning Rod spender, copy, and Earthquake spread boundaries.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn rod(run: &Run, target: Entity) -> Option<f64> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.spell_id == 197209 && aura.caster == run.actor)?
        .expires_at
}

fn select_rod(run: &mut Run) {
    let mut spells: Vec<u32> = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap()
        .iter()
        .collect();
    spells.push(210689);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::class_progression::KnownSpells::fixture(spells));
}

fn trigger(run: &mut Run, spell: u32, target: Entity) {
    assert!(
        crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(target),
            0
        ),
        "native spell {spell}"
    );
    for _ in 0..4 {
        run.app.update();
    }
}

#[test]
#[ignore = "native262 reference inputs and private world database"]
fn elemental_spenders_apply_rod_and_lightning_copies_resolved_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(262, 1).unwrap();
    select_rod(&mut run);
    engage(&mut run);
    let target = run.targets[0];
    trigger(&mut run, 188196, target);
    assert!(
        rod(&run, target).is_none(),
        "Lightning Bolt does not apply Elemental Rod"
    );
    assert!(
        !run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.spell_id == Some(197568))
    );
    trigger(&mut run, 8042, target);
    assert!(rod(&run, target).is_some(), "Earth Shock applies Rod");
    let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    trigger(&mut run, 188196, target);
    let world = run.app.world();
    let events = &world.resource::<ObservedCombatEvents>().0[before..];
    let bolt = events
        .iter()
        .find(|e| {
            e.source == Some(run.actor.to_bits())
                && e.spell_id == Some(188196)
                && e.kind == CombatLogKind::Damage
        })
        .expect("real Lightning Bolt hit");
    let copy = events
        .iter()
        .find(|e| {
            e.source == Some(run.actor.to_bits())
                && e.spell_id == Some(197568)
                && e.kind == CombatLogKind::Damage
        })
        .expect("existing Rod copies Bolt");
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(197568)
        .unwrap();
    let taken = crate::auras::damage_taken_multiplier(world, target, run.actor, spell);
    assert!((copy.amount as f32 - bolt.amount as f32 * 0.1 * taken).abs() <= 2.0);
    assert_eq!(copy.target, Some(target.to_bits()));
    assert!(!copy.crit);
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 197209,
    );
    trigger(&mut run, 117014, target);
    assert!(rod(&run, target).is_some(), "Elemental Blast applies Rod");
}

#[test]
#[ignore = "native262 reference inputs and private world database"]
fn elemental_earthquake_spreads_to_unmarked_targets_without_refresh() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(262, 3).unwrap();
    select_rod(&mut run);
    engage(&mut run);
    let primary = run.targets[0];
    trigger(&mut run, 462620, primary);
    let expiry = rod(&run, primary).expect("first Earthquake marks its primary");
    trigger(&mut run, 462620, primary);
    assert_eq!(
        rod(&run, primary),
        Some(expiry),
        "Earthquake must not refresh an existing Rod"
    );
    assert_eq!(
        run.targets
            .iter()
            .filter(|&&target| rod(&run, target).is_some())
            .count(),
        2
    );
    trigger(&mut run, 462620, primary);
    assert_eq!(
        run.targets
            .iter()
            .filter(|&&target| rod(&run, target).is_some())
            .count(),
        3
    );
    trigger(&mut run, 462620, primary);
    assert_eq!(rod(&run, primary), Some(expiry));
}
