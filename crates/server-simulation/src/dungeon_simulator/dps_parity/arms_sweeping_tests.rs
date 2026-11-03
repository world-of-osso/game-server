use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::spell_cast::powers::PowerPools;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_sweeping_search_handles_scaled_and_missing_model_reach() {
    let mut run = prepare_measurement(71, 3).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    let missing = run.targets[1];
    let large = run.targets[2];
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(260708, actor, Some(actor)),
    )
    .unwrap();
    let world = run.app.world_mut();
    world
        .entity_mut(missing)
        .remove::<shared::components::ModelDisplay>();
    let mut position = *world.get::<shared::components::Position>(large).unwrap();
    position.x = 200.0;
    world
        .entity_mut(large)
        .insert((position, shared::components::UnitScale(100.0)));
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(12294)
        .unwrap()
        .clone();
    let recipients = crate::spell_cast::targets::chain_targets(
        world,
        actor,
        &spell,
        &spell.effects[0],
        primary,
        crate::spell_cast::targets::Relation::Enemy,
    );
    assert_eq!(
        recipients,
        vec![large],
        "scaled target is found and missing geometry is excluded"
    );
    world
        .entity_mut(primary)
        .remove::<shared::components::ModelDisplay>();
    assert!(
        crate::spell_cast::targets::chain_targets(
            world,
            actor,
            &spell,
            &spell.effects[0],
            primary,
            crate::spell_cast::targets::Relation::Enemy,
        )
        .is_empty(),
        "missing primary geometry fails the chain without a panic"
    );
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_sweeping_strikes_missing_aura_does_not_panic_or_reapply() {
    let mut run = prepare_measurement(71, 1).unwrap();
    let actor = run.actor;
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(260708)
        .unwrap()
        .clone();
    assert!(!crate::auras::has_aura(run.app.world(), actor, 260708));
    crate::spell_cast::script_registry::after_foreground_special(
        run.app.world_mut(),
        actor,
        &spell,
    );
    assert!(!crate::auras::has_aura(run.app.world(), actor, 260708));
}

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_sweeping_strikes_grants_native_uses_and_one_secondary_mortal_strike() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(71, 3).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    run.app
        .world_mut()
        .get_mut::<PowerPools>(actor)
        .unwrap()
        .get_mut(PowerType::Rage)
        .unwrap()
        .current = 400.0;
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(260708, actor, Some(actor)),
    )
    .unwrap();
    let uses = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 260708 && aura.caster == actor)
            .map_or(0, |aura| aura.stacks)
    };
    // SimC76f8118 sc_warrior.cpp5291–5294: current-version effect4
    // supplies initial uses, rather than one default aura stack or max18.
    assert_eq!(
        uses(&run),
        12,
        "native70448 Sweeping Strikes grants its authored12 uses"
    );
    for _ in 0..60 {
        run.app.update();
    }
    // Within native melee reach, but beyond five yards between centers.
    let mut position = *run
        .app
        .world()
        .get::<shared::components::Position>(actor)
        .unwrap();
    position.x = 6.6;
    position.z = 0.0;
    run.app.world_mut().entity_mut(actor).insert(position);
    execute(
        run.app.world_mut(),
        actor,
        Execution::Spell(12294, primary, Some(primary)),
    )
    .unwrap();
    run.app.update();
    let targets: std::collections::BTreeSet<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(12294)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .filter_map(|event| event.target)
        .collect();
    assert!(targets.contains(&primary.to_bits()));
    assert_eq!(
        targets.len(),
        2,
        "one native extra target, not all three enemies"
    );
    assert_eq!(
        uses(&run),
        11,
        "one use is consumed only after the native strike hits two targets"
    );
}
