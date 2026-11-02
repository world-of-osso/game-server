use super::follower_dps_diagnostic::*;
use super::*;

fn spin(selected: bool, count: usize) -> Vec<i64> {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 90);
    let mut enemies = vec![target];
    for index in 1..count {
        enemies.push(spawn_level_dummy(&mut app, actor, 90, index).unwrap());
    }
    for _ in 0..5 {
        app.update();
    }
    for &enemy in &enemies {
        crate::combat_refs::set_in_combat_with(app.world_mut(), actor, enemy);
    }
    assert!(crate::auras::has_aura(app.world(), actor, 444778));
    if !selected {
        crate::auras::remove_auras(
            app.world_mut(),
            actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 444778,
        );
    }
    let action =
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 227847).unwrap();
    crate::spell_cast::creature_cast(app.world_mut(), actor, action, Some(actor), false).unwrap();
    for _ in 0..500 {
        app.update();
        if !crate::auras::has_aura(app.world(), actor, action) {
            break;
        }
    }
    let events = &app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    let hits: Vec<_> = events.iter().filter(|event| {
        event.source == Some(actor.to_bits())
            && enemies.iter().any(|enemy| event.target == Some(enemy.to_bits()))
            && event.spell_id == Some(50622)
            && event.amount > 0
    }).collect();
    for enemy in &enemies {
        assert!(hits.iter().any(|event| event.target == Some(enemy.to_bits())));
    }
    hits.iter().map(|event| i64::from(event.amount)).collect()
}

fn assert_multiplier(count: usize, expected: f64) {
    let base = spin(false, count);
    let enhanced = spin(true, count);
    assert!(!base.is_empty());
    assert_eq!(base.len(), enhanced.len());
    let plain: i64 = base.iter().sum();
    let buffed: i64 = enhanced.iter().sum();
    let ratio = buffed as f64 / plain as f64;
    assert!((ratio - expected).abs() < 0.002, "Bladestorm damage ratio {ratio}");
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_culling_cyclone_increases_single_target_bladestorm() {
    assert_multiplier(1, 1.2);
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_culling_cyclone_divides_bonus_between_two_targets() {
    assert_multiplier(2, 1.1);
}
