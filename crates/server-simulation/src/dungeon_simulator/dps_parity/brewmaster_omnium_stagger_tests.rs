//! Omnium Fire treats a proc on self-Stagger as a heal.
use super::*;
use server::test_support::healer_intake::{Hit, HitKind};
use shared::{components::Health, protocol::CombatLogKind};

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn actual_stagger_ticks_can_trigger_omnium_healing() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let attacker = run.targets[0];
    let hit = Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 100_000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    for _ in 0..20 {
        {
            let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
            health.current = health.max * 0.5;
        }
        crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
        for _ in 0..500 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if events.iter().any(|event| {
            event.spell_id == Some(1263002)
                && event.kind == CombatLogKind::Heal
                && event.target == Some(actor.to_bits())
                && event.amount > 0
        }) {
            assert!(
                events
                    .iter()
                    .any(|event| event.spell_id == Some(124255)
                        && event.kind == CombatLogKind::Damage)
            );
            return;
        }
    }
    panic!("actual self-Stagger ticks must be able to trigger Omnium healing");
}
