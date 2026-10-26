use super::*;
use shared::components::PowerType;

fn pet_focus(run: &Run, pet: Entity) -> f32 {
    crate::spell_cast::powers::power_value(run.app.world(), pet, PowerType::Focus)
        .unwrap()
        .0
}

fn event_cursor(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len()
}

fn landed_basic_since(run: &Run, pet: Entity, cursor: usize) -> Option<f32> {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..]
        .iter()
        .find(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(49966)
                && event.kind == shared::protocol::CombatLogKind::Damage
        })
        .map(|event| event.amount as f32 / if event.crit { 2.0 } else { 1.0 })
}

fn try_cast_paid_basic(run: &mut Run, pet: Entity) -> Option<f32> {
    use crate::spell_cast::CastFailure;
    let cursor = event_cursor(run);
    let target = run.targets[0];
    match crate::spell_cast::creature_cast(run.app.world_mut(), pet, 49966, Some(target), false) {
        Ok(()) => landed_basic_since(run, pet, cursor),
        Err(CastFailure::OnCooldown | CastFailure::OnGlobalCooldown) => None,
        Err(error) => panic!("native paid basic refused unexpectedly: {error:?}"),
    }
}

fn sample_paid_high_focus_basic(run: &mut Run, pet: Entity, threshold: f32) -> f32 {
    // Ordinary native attacks can dodge. Wait for actual regen/cooldown and
    // sample a real landed paid attack, never alter Focus or force an outcome.
    for _ in 0..1200 {
        run.app.update();
        if pet_focus(run, pet) > threshold
            && let Some(damage) = try_cast_paid_basic(run, pet)
        {
            return damage;
        }
    }
    panic!("no real high-Focus paid basic landed during the bounded native wait")
}

fn sample_background_basic(run: &mut Run, pet: Entity) -> f32 {
    for _ in 0..32 {
        let cursor = event_cursor(run);
        assert!(
            crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                pet,
                49966,
                Some(run.targets[0]),
                1
            ),
            "actual background basic must launch"
        );
        if let Some(damage) = landed_basic_since(run, pet, cursor) {
            return damage;
        }
    }
    panic!("no actual native background basic landed in32 attempts")
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_hunter_wild_hunt_snapshots_high_focus_damage_and_cost_before_payment() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    let store = run.app.world().resource::<crate::spell_info::SpellStore>();
    let basic = store.get(49966).expect("native main-pet Smack").clone();
    let wild_hunt = store.get(62762).expect("native Wild Hunt values");
    let threshold = wild_hunt.effects[2].base_points;
    let cost_multiplier = 1.0 + wild_hunt.effects[1].base_points / 100.0;
    let damage_multiplier = 1.0 + wild_hunt.effects[0].base_points / 100.0;
    let ordinary_cost = wild_hunt.effects[3].base_points;
    let before = pet_focus(&run, pet);
    assert!(
        before > threshold,
        "native initial Focus must enable Wild Hunt"
    );
    let cursor = event_cursor(&run);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        pet,
        basic.id,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary native main-pet basic");
    let first_hit = landed_basic_since(&run, pet, cursor);
    let paid = before - pet_focus(&run, pet);
    // SimC76f8118 sc_hunter.cpp3046–3079: the same strict high-Focus
    // condition snapshots damage before the real doubled payment.
    assert!(
        (paid - ordinary_cost * cost_multiplier).abs() < 0.001,
        "high-Focus basic must pay native Wild Hunt cost: before={before}, paid={paid}, ordinary={ordinary_cost}, multiplier={cost_multiplier}"
    );
    assert!(
        pet_focus(&run, pet) <= threshold,
        "real boosted payment must close Wild Hunt"
    );
    let costs = crate::spell_cast::applicable_costs(run.app.world(), pet, &basic).unwrap();
    assert!(
        (costs[0].1 - ordinary_cost).abs() < 0.001,
        "at or below the threshold the next ordinary basic costs25, not50"
    );
    let boosted = match first_hit {
        Some(damage) => damage,
        None => sample_paid_high_focus_basic(&mut run, pet, threshold),
    };
    let ordinary = sample_background_basic(&mut run, pet);
    let ratio = boosted / ordinary;
    assert!(
        (ratio - damage_multiplier).abs() < damage_multiplier * 0.15,
        "boost must survive payment; background basic stays ordinary: boosted={boosted}, ordinary={ordinary}, ratio={ratio}, expected={damage_multiplier}"
    );
}
