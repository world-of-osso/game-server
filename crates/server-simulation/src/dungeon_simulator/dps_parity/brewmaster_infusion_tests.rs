//! Native Celestial Infusion shield capacity and damage absorption.
use super::*;
use crate::stats::CharacterStats;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn infusion_absorbs_only_its_fraction_of_each_hit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
        .unwrap();
    let world = run.app.world();
    let ap = world.get::<CharacterStats>(actor).unwrap().attack_power;
    let aura = world
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 1241059)
        .unwrap();
    let capacity = aura.effects.iter().find(|e| e.index == 0).unwrap().amount;
    println!(
        "Infusion native AP={ap} shield={capacity} shield/AP={}",
        capacity / ap
    );
    assert!(capacity > 6000.0);
    let (remaining, absorbed) =
        crate::auras::absorb_damage(run.app.world_mut(), actor, None, 1, 10_000.0);
    assert!(
        (remaining - 7000.0).abs() < 1.0 && (absorbed - 3000.0).abs() < 1.0,
        "30% of incoming damage must be absorbed, got {absorbed} absorbed and {remaining} through"
    );
    let (remaining, absorbed) =
        crate::auras::absorb_damage(run.app.world_mut(), actor, None, 4, 10_000.0);
    assert!(
        (remaining - 7000.0).abs() < 1.0 && (absorbed - 3000.0).abs() < 1.0,
        "the remaining shield must absorb 30% of a second school"
    );
}
