//! Real Barkskin uses the selected Matted Fur rank and one recipient multiplier.
use super::*;
use crate::auras::{ActiveAuras, AuraStatMods};
use crate::stats::{CharacterStats, StatModifiers, WeaponAttackType};

#[test]
#[ignore = "requires Guardian rank-two Matted Fur profile and native world data"]
fn guardian_matted_fur_uses_selected_rank_and_single_absorb_multiplier() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let world = run.app.world();
    let coefficient = crate::spell_cast::scripts::script_effect_value(world, actor, 385786, 0);
    assert_eq!(coefficient, 3750.0);
    let stats = world.get::<CharacterStats>(actor).unwrap();
    let recipient = world.get::<AuraStatMods>(actor).unwrap().absorb_taken_mult;
    let attack_power = stats.total_attack_power(
        WeaponAttackType::MainHand,
        world.get::<StatModifiers>(actor),
    );
    assert!(attack_power > stats.attack_power, "the equipped weapon contributes attack power");
    let expected = (attack_power * coefficient / 100.0
        * (1.0 + stats.versatility_pct / 100.0)
        * recipient)
        .round();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 22812, Some(actor), false)
        .unwrap();
    let shield = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 385787 && a.caster == actor)
        .unwrap();
    let amount = shield.effects.iter().find(|e| e.index == 0).unwrap().amount;
    assert!(
        (amount - expected).abs() <= 1.0,
        "rank-scaled shield {amount} vs {expected}"
    );
    assert_eq!(shield.duration_ms, 15000);
}
