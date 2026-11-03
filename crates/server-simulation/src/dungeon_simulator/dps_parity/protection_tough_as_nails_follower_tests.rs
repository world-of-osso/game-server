use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires MID2 Protection profile and native combat catalogs"]
fn dps_parity_protection_tough_as_nails_retaliates_only_to_hostile_blocks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(73, 1).unwrap();
    let actor = run.actor;
    let attacker = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 385888));
    engage(&mut run);
    let hit = |outcome: &str| super::super::healer_intake::Hit {
        tick: 1,
        source_guid: 0,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 10_000.0,
        level_scaling_factor: 1.0,
        kind: super::super::healer_intake::HitKind::Melee {
            hand: 0,
            outcome: outcome.into(),
            level_difference: -2,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit("Hit")).unwrap();
    run.app.update();
    let count = |run: &Run| {
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(attacker.to_bits())
                    && event.spell_id == Some(385890)
                    && event.kind == CombatLogKind::Damage
            })
            .count()
    };
    assert_eq!(count(&run), 0, "unblocked attack must not retaliate");
    crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit("Block")).unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    assert_eq!(
        count(&run),
        1,
        "a real hostile block triggers one native retaliation"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let damage = events
        .iter()
        .find(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(attacker.to_bits())
                && event.spell_id == Some(385890)
                && event.kind == CombatLogKind::Damage
        })
        .unwrap();
    assert!(damage.amount > 0);
    assert!(!damage.crit, "SimC retaliation cannot crit");
}
