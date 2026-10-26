//! Conductive Energy must use actual native casts, caster-owned rods and resolved damage.
use super::enhancement_generation_tests::cast;
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn conductive_energy_real_chain_applies_one_rod_then_mirrors_the_next_complete_cast() {
    // SimC76f8118 sc_shaman.cpp3423–3486/6546–6564/6698/11478: first application is deferred after damage.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 3).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(455123)
    );
    engage(&mut run);
    let target = run.targets[0];
    cast(&mut run, 188443, target);
    for _ in 0..4 {
        run.app.update();
    }
    let world = run.app.world();
    let chain_hits = world
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(188443)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        chain_hits, 3,
        "native Chain Lightning must hit all three attackable dummies before Rod application"
    );
    let rod = world
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 197209 && aura.caster == run.actor)
        .expect("actual Chain Lightning must apply this caster's native Lightning Rod");
    let amount = rod
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let coefficient = world
        .resource::<crate::spell_info::SpellStore>()
        .get(210689)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points;
    assert_eq!(
        amount, coefficient,
        "Rod value uses native210689 effect1, not the unrelated tooltip default"
    );
    assert_eq!(
        rod_damage(&run).len(),
        0,
        "the creating cast cannot mirror into its newly applied Rod"
    );
    cast(&mut run, 188443, target);
    for _ in 0..4 {
        run.app.update();
    }
    let damage = rod_damage(&run);
    assert_eq!(
        damage.len(),
        1,
        "one complete three-target cast emits one accumulated Rod damage event"
    );
    assert_eq!(damage[0].target, Some(target.to_bits()));
    assert!(
        !damage[0].crit,
        "Lightning Rod cannot crit its already resolved source damage again"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn conductive_energy_mirrors_resolved_chain_damage_without_reapplying_caster_spell_mods() {
    // SimC76f8118 sc_shaman.cpp4471–4485: Lightning Rod retains only target multipliers.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 3).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    cast(&mut run, 188443, target);
    for _ in 0..4 {
        run.app.update();
    }
    let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    cast(&mut run, 188443, target);
    for _ in 0..4 {
        run.app.update();
    }
    let world = run.app.world();
    let events = &world.resource::<ObservedCombatEvents>().0[before..];
    let chain_damage: i64 = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(188443)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| i64::from(event.amount))
        .sum();
    assert!(chain_damage > 0, "the real second chain must deal damage");
    let mirrored: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(197568)
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert_eq!(
        mirrored.len(),
        1,
        "exactly one existing Rod receives the complete chain"
    );
    let store = world.resource::<crate::spell_info::SpellStore>();
    let coefficient = store
        .get(210689)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points;
    let target_multiplier =
        crate::auras::damage_taken_multiplier(world, target, run.actor, store.get(197568).unwrap());
    let expected = chain_damage as f32 * coefficient / 100.0 * target_multiplier;
    assert!(
        (mirrored[0].amount as f32 - expected).abs() <= 2.0,
        "Rod must reflect the resolved native amount once, not run caster spell modifiers again: source={chain_damage}, coefficient={coefficient}, target_multiplier={target_multiplier}, expected={expected}, actual={}",
        mirrored[0].amount
    );
    assert!(!mirrored[0].crit);
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn conductive_energy_real_burning_core_does_not_amplify_the_next_rod_copy() {
    // SimC76f8118 sc_shaman.cpp7010–7028 scopes Burning Core to Fire Nova;4471–4485 keeps valid Rod target modifiers.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 3).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    cast(&mut run, 188443, target);
    for _ in 0..4 {
        run.app.update();
    }
    let baseline = {
        let world = run.app.world();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(197568)
            .unwrap();
        crate::auras::damage_taken_multiplier(world, target, run.actor, spell)
    };
    cast(&mut run, 470057, target);
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(target)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 1299975 && aura.caster == run.actor),
        "actual Voltaic Blaze must establish this caster's Burning Core"
    );
    let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    cast(&mut run, 188443, target);
    for _ in 0..4 {
        run.app.update();
    }
    let world = run.app.world();
    let events = &world.resource::<ObservedCombatEvents>().0[before..];
    let source: i64 = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(188443)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| i64::from(event.amount))
        .sum();
    assert!(source > 0);
    let mirrored = rod_damage(&run);
    assert_eq!(
        mirrored.len(),
        1,
        "one existing Rod must receive the second actual chain"
    );
    let store = world.resource::<crate::spell_info::SpellStore>();
    let coefficient = store
        .get(210689)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points;
    let expected = source as f32 * coefficient / 100.0 * baseline;
    assert!(
        (mirrored[0].amount as f32 - expected).abs() <= 2.0,
        "Burning Core must not amplify the native Rod copy: source={source}, coefficient={coefficient}, baseline_target={baseline}, expected={expected}, actual={}",
        mirrored[0].amount
    );
}

fn rod_damage(run: &Run) -> Vec<&shared::protocol::CombatLogEvent> {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            let routed =
                event.source == Some(run.actor.to_bits()) && event.spell_id == Some(197568);
            let dealt_damage = event.kind == CombatLogKind::Damage && event.amount > 0;
            routed && dealt_damage
        })
        .collect()
}
