//! Actual ranged-weapon casts must not acquire the Rogue melee-start behavior.
use super::*;
use shared::{components::WeaponType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_ranged_weapon_flagged_hit_does_not_begin_weapon_swings() {
    let (profile, traits) = read_reference(254).unwrap();
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    engage(&mut run);
    assert_eq!(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .main_hand
            .kind,
        WeaponType::Ranged,
        "fixture really carries the reference ranged weapon"
    );
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(185358, target, Some(target)),
    )
    .unwrap();
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(185358)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "the actual flagged Arcane Shot reaches the real enemy"
    );
    let swings: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id.is_none()
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .collect();
    assert!(
        swings.is_empty(),
        "ranged weapon cast must not begin melee: {swings:?}"
    );
}
