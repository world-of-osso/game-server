//! Retail-selected Umbilicus on real Blood Plague ticks and Vampiric Blood expiry.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Health;
use shared::protocol::CombatLogKind;

fn retail_traits() -> Vec<inputs::AllocatedTrait> {
    let profiles: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/reference/follower-wcl-candidates.json"
    )))
    .unwrap();
    profiles["profiles"]["death-knight-blood/10-14"]["talent_purchases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| inputs::AllocatedTrait {
            node_id: row[0].as_u64().unwrap() as u32,
            entry_id: row[1].as_u64().unwrap() as u32,
            ranks: row[2].as_u64().unwrap() as u32,
            active_ranks: row[2].as_u64().unwrap() as u32,
        })
        .collect()
}

#[test]
#[ignore = "requires native Blood profile and world database"]
fn umbilicus_banks_owned_plague_including_absorbed_damage_until_vampiric_blood_expires() {
    run_case(false);
    run_case(true);
}

fn run_case(strong_plague: bool) {
    let _seed = crate::random::seed_scope(70448);
    let (profile, _) = read_reference(250).unwrap();
    let identity = inputs::read_actor(&profile).unwrap();
    let traits = retail_traits();
    assert!(traits.iter().any(|entry| entry.entry_id == 96263));
    assert!(traits.iter().any(|entry| entry.entry_id == 96168));
    let mut run = prepare_literal_reference(profile, &traits, 2, identity).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let shielded = run.targets[0];
    let unshielded = run.targets[1];
    for &target in &run.targets {
        let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
        health.max = 1_000_000_000.0;
        health.current = health.max;
    }
    assert!(crate::spell_cast::cast_triggered_with_overrides(
        run.app.world_mut(),
        shielded,
        391527,
        Some(shielded),
        1,
        crate::spell_cast::TriggeredSpellOverrides {
            effect0_amount: Some(1_000_000_000.0),
            ..Default::default()
        },
    ));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 55233, Some(actor), false)
        .unwrap();
    assert!(crate::auras::has_aura(run.app.world(), actor, 391519));
    for target in [shielded, unshielded] {
        assert!(crate::spell_cast::cast_triggered_with_overrides(
            run.app.world_mut(),
            actor,
            55078,
            Some(target),
            1,
            crate::spell_cast::TriggeredSpellOverrides {
                effect0_amount: strong_plague.then_some(1_000_000.0),
                ..Default::default()
            },
        ));
    }
    for _ in 0..500 {
        if !crate::auras::has_aura(run.app.world(), actor, 55233) {
            break;
        }
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 55233),
        "Vampiric Blood must expire"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let plague: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(55078)
                && event.kind == CombatLogKind::Damage
                && event.periodic
        })
        .collect();
    assert!(
        plague.iter().any(|event| event.absorbed > 0),
        "a shielded enemy must absorb an actual tick"
    );
    assert!(
        plague.iter().any(|event| event.amount > 0),
        "an unshielded enemy must take an actual tick"
    );
    let damage: i32 = plague
        .iter()
        .map(|event| event.amount + event.absorbed)
        .sum();
    assert!(damage > 0);
    for _ in 0..10 {
        run.app.update();
        if crate::auras::has_aura(run.app.world(), actor, 391527) {
            break;
        }
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 391527),
        "Umbilicus shield after natural expiry"
    );
    let target_multiplier = run
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(actor)
        .unwrap()
        .absorb_taken_mult;
    let max_after_vampiric = run.app.world().get::<Health>(actor).unwrap().max;
    assert_eq!(
        damage as f32 * 6.0 >= max_after_vampiric,
        strong_plague,
        "exercise the selected cap case"
    );
    let expected = (damage as f32 * 6.0).min(max_after_vampiric) * target_multiplier;
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| {
            aura.spell_id != 391527
                && aura
                    .effects
                    .iter()
                    .any(|effect| effect.aura_type == crate::auras::handlers::SCHOOL_ABSORB)
        },
    );
    let (_, actual) = crate::auras::absorb_damage(
        run.app.world_mut(),
        actor,
        Some(unshielded),
        4,
        (expected + 1_000.0).ceil(),
    );
    assert!(
        (actual - expected).abs() <= 1.0,
        "actual={actual} expected={expected} damage={damage} cap={max_after_vampiric} multiplier={target_multiplier}"
    );
}
