use super::*;
use super::follower_dps_diagnostic::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_wild_strikes_requires_critical_auto_attack() {
    fn swing(critical: bool) -> (bool, bool) {
        let _seed = crate::random::seed_scope(70448);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2515).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 70);
        for _ in 0..5 {
            app.update();
        }
        assert!(crate::auras::has_aura(app.world(), actor, 382946));
        face_actor_toward_primary(app.world_mut(), actor, target);
        crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
        crate::combat::start_attack_now(app.world_mut(), actor, crate::combat::AutoAttack::new(target));
        for _ in 0..300 {
            app.world_mut().get_mut::<crate::stats::CharacterStats>(actor).unwrap().crit_pct = if critical { 100.0 } else { 0.0 };
            app.update();
            let events = &app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
            if let Some(hit) = events.iter().find(|event| event.source == Some(actor.to_bits()) && event.target == Some(target.to_bits()) && event.spell_id.is_none() && event.amount > 0) {
                return (hit.crit, crate::auras::has_aura(app.world(), actor, 392778));
            }
        }
        panic!("actual auto attack did not land");
    }
    let (normal_crit, normal_buff) = swing(false);
    assert!(!normal_crit);
    assert!(!normal_buff, "normal auto attack must not grant Wild Strikes");
    let (critical_crit, critical_buff) = swing(true);
    assert!(critical_crit);
    assert!(critical_buff, "critical auto attack must grant Wild Strikes");
}
