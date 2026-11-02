use super::follower_dps_diagnostic::*;
use super::*;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_culling_cyclone_increases_single_target_bladestorm() {
    fn spin(selected: bool) -> Vec<i64> {
        let _seed = crate::random::seed_scope(70448);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 90);
        for _ in 0..5 {
            app.update();
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
        crate::spell_cast::creature_cast(app.world_mut(), actor, action, Some(actor), false)
            .unwrap();
        for _ in 0..500 {
            app.update();
            if !crate::auras::has_aura(app.world(), actor, action) {
                break;
            }
        }
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(50622)
                    && event.amount > 0
            })
            .map(|event| i64::from(event.amount))
            .collect()
    }
    let base = spin(false);
    let enhanced = spin(true);
    assert!(!base.is_empty());
    assert_eq!(base.len(), enhanced.len());
    let plain: i64 = base.iter().sum();
    let buffed: i64 = enhanced.iter().sum();
    let ratio = buffed as f64 / plain as f64;
    assert!(
        (ratio - 1.2).abs() < 0.002,
        "Bladestorm damage ratio {ratio}"
    );
}
