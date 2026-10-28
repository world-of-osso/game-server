//! Actual controlled main-Imp power inputs, without surrogate guardian actors.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Npc, UnitIsPet};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private267 native player profile"]
fn destruction_imp_native_firebolt_inherits_validated_live_owner_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    let owner = run.actor;
    // Profile starts at0 shards. Reuse native OOC readiness, never grant or reset.
    preparation::start_configured_native_pet(&mut run).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let imp = crate::hunter_pet::summoned_pet(run.app.world(), owner).unwrap();
    assert_eq!(run.app.world().get::<Npc>(imp).unwrap().template_id, 416);
    assert!(run.app.world().get::<UnitIsPet>(imp).unwrap().0);
    let target = run.targets[0];
    // Read-only native27 planning also covers controlled pets. Do not cast a
    // different pet's energize or trust the supplied enemy as the master.
    let master_spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(264365)
        .unwrap()
        .clone();
    let recipients = crate::spell_cast::targets::select_targets(
        run.app.world_mut(),
        imp,
        Some(target),
        &master_spell,
        &master_spell.effects[0],
        0,
        false,
    )
    .unwrap();
    assert_eq!(
        recipients,
        vec![owner],
        "native27 must resolve the actual controlled owner"
    );
    engage(&mut run);
    // A real pet order isolates power from literal-controller engagement.
    crate::pet_control::order_companion_pet(run.app.world_mut(), owner, Some(target));
    run.app
        .insert_resource(FactorCapture::new(imp, target, 3110, "firebolt", 1));
    for _ in 0..200 {
        let owner_power = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(owner)
            .unwrap()
            .spell_power;
        run.app.update();
        let expected_identity = (Some(imp.to_bits()), Some(target.to_bits()), Some(3110));
        let landed = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                let identity = (event.source, event.target, event.spell_id);
                let is_damage = event.kind == CombatLogKind::Damage;
                identity == expected_identity && is_damage && event.amount > 0
            });
        if !landed {
            continue;
        }
        let capture = run.app.world().resource::<FactorCapture>().export();
        let calculation = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 3110)
            .expect("actual Firebolt must have its native input calculation");
        let spell_power = calculation["factors"]["s_power"].as_f64().unwrap();
        let attack_power = calculation["factors"]["a_power"].as_f64().unwrap();
        // SimC76f8118 pets.cpp372–375: both powers inherit from live owner SP.
        let expected_spell = f64::from(owner_power) * 5.625;
        let expected_attack = f64::from(owner_power) * 2.8125;
        assert!(
            (spell_power - expected_spell).abs() < 1.0,
            "actual main Imp SP: inherited={spell_power}, owner={owner_power}, expected={expected_spell}"
        );
        assert!(
            (attack_power - expected_attack).abs() < 1.0,
            "actual main Imp AP: inherited={attack_power}, ownerSP={owner_power}, expected={expected_attack}"
        );
        return;
    }
    panic!("explicitly ordered native Imp must land a real3110 before power is qualified");
}

#[test]
#[ignore = "requires private267 native player profile"]
fn destruction_imp_firebolt_inherits_owner_critical_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    let owner = run.actor;
    preparation::start_configured_native_pet(&mut run).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let imp = crate::hunter_pet::summoned_pet(run.app.world(), owner).unwrap();
    let target = run.targets[0];
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(3110)
        .unwrap();
    let owner_critical = run
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(owner)
        .unwrap()
        .critical_damage(spell.school_mask());
    assert!(
        owner_critical > 1.0,
        "the actual owner must wear its critical-damage enchants"
    );
    engage(&mut run);
    crate::pet_control::order_companion_pet(run.app.world_mut(), owner, Some(target));
    run.app
        .insert_resource(FactorCapture::new(imp, target, 3110, "firebolt", 30));
    for _ in 0..600 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let events = capture["events"].as_array().unwrap();
    let position = events
        .iter()
        .position(|event| {
            event["event"] == "result"
                && event["spell_id"] == 3110
                && event["factors"]["result"] == "crit"
        })
        .expect("actual Firebolt must critically hit");
    let result = &events[position];
    let direct = events[..position]
        .iter()
        .rev()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 3110)
        .unwrap();
    let uncritical = direct["factors"]["amount"].as_f64().unwrap();
    let actual = result["factors"]["actual_amount"].as_f64().unwrap();
    // SimC76f8118 pet.cpp488–494 inherits the owner's critical-damage
    // multiplier; its equipment aura applies to the whole critical hit.
    let expected = uncritical * 2.0 * f64::from(owner_critical);
    assert!(
        (actual - expected).abs() < 2.0,
        "critical Firebolt {actual}, expected {expected}"
    );
}
