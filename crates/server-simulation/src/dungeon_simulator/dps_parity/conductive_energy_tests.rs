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
