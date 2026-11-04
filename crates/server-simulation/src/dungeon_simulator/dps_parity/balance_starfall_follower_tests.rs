use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn cast_starfall(run: &mut Run) {
    let (_, maximum) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::LunarPower)
            .unwrap();
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        run.actor,
        PowerType::LunarPower,
        maximum,
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        191034,
        Some(run.actor),
        false,
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
}

fn owned_aura(run: &Run, spell: u32) -> Option<crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
        .cloned()
}

#[test]
#[ignore = "requires native Balance102 WCL profile and frozen world data"]
fn balance_starfall_periodic_hits_owned_combat_targets_and_expires_independent_stacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(102, 3).unwrap();
    let untouched = run.targets.pop().unwrap();
    engage(&mut run);
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1296603,
    );
    crate::spell_cast::cast_triggered_with_overrides(
        run.app.world_mut(),
        run.actor,
        1263382,
        Some(run.actor),
        1,
        crate::spell_cast::TriggeredSpellOverrides {
            aura_stacks: Some(3),
            ..Default::default()
        },
    );
    cast_starfall(&mut run);
    assert_eq!(owned_aura(&run, 1263382).unwrap().stacks, 2);
    assert_eq!(
        owned_aura(&run, 1263401)
            .expect("consumed Ascendant Stars earns Starfall budget")
            .stacks,
        1
    );
    let first_expiry = owned_aura(&run, 191034).unwrap().expires_at.unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(191037))
        .collect();
    assert!(hits.len() >= 2, "real periodic Starfall damage must land");
    assert!(
        hits.iter()
            .all(|event| event.amount > 0 && event.target != Some(untouched.to_bits()))
    );
    for target in &run.targets {
        assert!(
            hits.iter()
                .any(|event| event.target == Some(target.to_bits()))
        );
    }
    cast_starfall(&mut run);
    let second = owned_aura(&run, 191034).unwrap();
    assert_eq!(second.stacks, 2);
    let final_expiry = second.expires_at.unwrap();
    while crate::spell_cast::now(run.app.world()) <= first_expiry + 0.1 {
        run.app.update();
    }
    assert_eq!(
        owned_aura(&run, 191034).unwrap().stacks,
        1,
        "first application expires independently"
    );
    while crate::spell_cast::now(run.app.world()) <= final_expiry + 1.1 {
        run.app.update();
    }
    assert!(owned_aura(&run, 191034).is_none());
    let event_count = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(191037))
        .count();
    for _ in 0..40 {
        run.app.update();
    }
    let final_count = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(191037))
        .count();
    assert_eq!(
        event_count, final_count,
        "expired Starfall cannot keep scheduling damage"
    );
}
