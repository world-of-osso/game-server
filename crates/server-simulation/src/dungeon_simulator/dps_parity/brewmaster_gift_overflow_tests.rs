//! Gift of the Ox consumes the oldest orb rather than retaining overflow.
use super::*;
use server::test_support::healer_intake::{Hit, HitKind};
use shared::{components::Health, protocol::CombatLogKind};

fn orb_heals(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.spell_id == Some(124507) && event.kind == CombatLogKind::Heal)
        .count()
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn overflow_consumes_oldest_without_retaining_new_orb() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let attacker = run.targets[0];
    let maximum = run.app.world().get::<Health>(actor).unwrap().max;
    let hit = Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: maximum,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    for _ in 0..6 {
        run.app
            .world_mut()
            .get_mut::<Health>(actor)
            .unwrap()
            .current = maximum;
        crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
    }
    assert_eq!(orb_heals(&run), 1, "sixth orb triggers the oldest");
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 322101, Some(actor), false)
        .unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    assert_eq!(orb_heals(&run), 5, "four orbs remain for Expel Harm");
}
