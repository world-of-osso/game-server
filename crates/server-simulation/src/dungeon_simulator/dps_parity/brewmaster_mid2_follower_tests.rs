use super::*;
use crate::auras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::spell_info::SpellStore;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires equipped MID2 Brewmaster profile and world data"]
fn dps_parity_brewmaster_ignited_keg_marks_target_for_physical_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(268, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::item_proc_data::has_equipped_bonus(run.app.world(), actor, 1296617).unwrap());
    assert!(crate::item_proc_data::has_equipped_bonus(run.app.world(), actor, 1296618).unwrap());
    engage(&mut run);
    let tiger = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(100780)
        .unwrap()
        .clone();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115181, Some(target), false)
        .unwrap();
    assert!(
        auras::has_aura(run.app.world(), actor, 1301477),
        "Breath of Fire primes set"
    );
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(!auras::has_aura(run.app.world(), actor, 1301477));
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let hit = |spell| {
        events.iter().any(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
    };
    assert!(hit(1301619), "ignited Keg lands Fiery Shrapnel");
    assert!(
        auras::has_aura(run.app.world(), target, 1301410),
        "shrapnel applies Scorched"
    );
    let after = auras::damage_taken_multiplier(run.app.world(), target, actor, &tiger);
    auras::remove_auras(
        run.app.world_mut(),
        target,
        auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1301410 && aura.caster == actor,
    );
    let without = auras::damage_taken_multiplier(run.app.world(), target, actor, &tiger);
    assert!(
        (after / without - 1.08).abs() < 0.001,
        "Tiger Palm target multiplier {without} -> {after}"
    );
}
