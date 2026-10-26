//! Actual selected-apex production through the frozen literal player profile.
use super::*;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_echo_native_apex_values_include_real_gear_and_talent_labels() {
    let _seed = crate::random::seed_scope(70448);
    let run = prepare_literal(267, 1).unwrap();
    let world = run.app.world();
    let gear = world.get::<crate::auras::ActiveAuras>(run.actor).unwrap();
    assert!(gear.iter().any(|aura| aura.spell_id == 1296571
        && matches!(aura.source, crate::auras::AuraSource::ItemSet(_))));
    let store = world.resource::<crate::spell_info::SpellStore>();
    let base = |spell, index| {
        store
            .get(spell)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .unwrap()
            .base_points
    };
    let chance = crate::spell_cast::scripts::script_effect_value(world, run.actor, 1265770, 0);
    assert_eq!(
        chance,
        base(1265770, 0) + base(1296571, 1),
        "native2pc adds its actual label-bound chance"
    );
    for index in [0, 1] {
        let vision =
            crate::spell_cast::scripts::script_effect_value(world, run.actor, 1265939, index);
        assert_eq!(
            vision,
            base(1265772, index),
            "native EmbersII binds Vision effect{index}"
        );
    }
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_echo_native_payload_hits_primary_once_and_each_secondary_once() {
    // Payload prerequisite only, not producer qualification: native1265884
    // has separate primary/secondary effects, matching actions5140–5163.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 3).unwrap();
    engage(&mut run);
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1265884)
        .unwrap()
        .clone();
    assert!(crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        run.actor,
        &spell,
        Some(run.targets[0]),
        1,
    ));
    for _ in 0..60 {
        run.app.update();
    }
    for &recipient in &run.targets {
        let expected = (
            Some(run.actor.to_bits()),
            Some(recipient.to_bits()),
            Some(1265884),
        );
        let hits = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                let identity = (event.source, event.target, event.spell_id);
                let damage = event.kind == CombatLogKind::Damage;
                identity == expected && damage && event.amount > 0
            })
            .count();
        assert_eq!(
            hits, 1,
            "native Echo must hit each actual recipient once: {recipient:?}"
        );
    }
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_echo_literal_player_earns_actual_pack_damage_and_native_vision() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(267, 3).unwrap();
    let traits = run.app.world().get::<TraitLoadout>(run.actor).unwrap();
    for spell in [1265770, 1265772, 1265774] {
        assert!(
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == spell),
            "frozen native profile must actually select apex{spell}"
        );
    }
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    let mut earned_vision = false;
    // Ordinary native casts, costs, pet orders and clocks; no forced child,
    // proc buff, shard grant, resource reset or surrogate caster.
    for _ in 0..1800 {
        drive_player(&mut run).unwrap();
        run.app.update();
        earned_vision |= crate::auras::has_aura(run.app.world(), run.actor, 1265939);
    }
    for &recipient in &run.targets {
        let expected = (
            Some(run.actor.to_bits()),
            Some(recipient.to_bits()),
            Some(1265884),
        );
        let landed = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                let identity = (event.source, event.target, event.spell_id);
                let damage = event.kind == CombatLogKind::Damage;
                identity == expected && damage && event.amount > 0
            });
        assert!(
            landed,
            "selected native apex must earn actual1265884 from literal player events at pack recipient{recipient:?}"
        );
    }
    assert!(
        earned_vision,
        "actual Echo must earn the native1265939 Vision aura"
    );
}
