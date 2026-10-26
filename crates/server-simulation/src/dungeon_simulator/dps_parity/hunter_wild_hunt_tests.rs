use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_hunter_wild_hunt_snapshots_high_focus_damage_and_cost_before_payment() {
    use shared::components::PowerType;
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    let focus = |run: &Run| {
        crate::spell_cast::powers::power_value(run.app.world(), pet, PowerType::Focus)
            .unwrap()
            .0
    };
    let store = run.app.world().resource::<crate::spell_info::SpellStore>();
    let basic = store.get(49966).expect("native main-pet Smack").clone();
    let wild_hunt = store.get(62762).expect("native Wild Hunt values");
    let threshold = wild_hunt.effects[2].base_points;
    let cost_multiplier = 1.0 + wild_hunt.effects[1].base_points / 100.0;
    let damage_multiplier = 1.0 + wild_hunt.effects[0].base_points / 100.0;
    let ordinary_cost = wild_hunt.effects[3].base_points;
    let before = focus(&run);
    assert!(
        before > threshold,
        "native initial Focus must enable Wild Hunt"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        pet,
        basic.id,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary native main-pet basic");
    let paid = before - focus(&run);
    // SimC76f8118 sc_hunter.cpp3046–3079: high Focus is snapshotted before
    // payment for both damage and cost, with a strict greater-than threshold.
    assert!(
        (paid - ordinary_cost * cost_multiplier).abs() < 0.001,
        "high-Focus basic must pay the native Wild Hunt cost: before={before}, paid={paid}, ordinary={ordinary_cost}, multiplier={cost_multiplier}"
    );
    assert!(
        focus(&run) <= threshold,
        "real boosted payment must close Wild Hunt"
    );
    let costs = crate::spell_cast::applicable_costs(run.app.world(), pet, &basic).unwrap();
    assert!(
        (costs[0].1 - ordinary_cost).abs() < 0.001,
        "at or below the native threshold the next ordinary basic costs25, not50"
    );
    assert!(
        crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            pet,
            basic.id,
            Some(run.targets[0]),
            1
        ),
        "background basic does not inherit main-action Wild Hunt"
    );
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(basic.id)
                && event.kind == shared::protocol::CombatLogKind::Damage
        })
        .map(|event| event.amount as f32 / if event.crit { 2.0 } else { 1.0 })
        .collect();
    assert_eq!(hits.len(), 2, "both actual native basics must land");
    let ratio = hits[0] / hits[1];
    assert!(
        (ratio - damage_multiplier).abs() < damage_multiplier * 0.15,
        "boost must survive native payment, while background basic stays ordinary: hits={hits:?}, ratio={ratio}, expected={damage_multiplier}"
    );
}
