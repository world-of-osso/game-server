//! Improved Adrenaline Rush fills the selected Outlaw follower's combo-point pool.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_improved_adrenaline_rush_fills_combo_points() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, _, _) = prepare_class_follower(app, copy, 260, 70);
    for _ in 0..3 {
        app.update();
    }
    assert!(crate::auras::has_aura(app.world(), actor, 395422));
    {
        let mut pools = app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap();
        pools.get_mut(PowerType::ComboPoints).unwrap().current = 0.0;
    }
    crate::spell_cast::creature_cast(app.world_mut(), actor, 13750, Some(actor), false).unwrap();
    let (current, maximum) =
        crate::spell_cast::powers::power_value(app.world(), actor, PowerType::ComboPoints).unwrap();
    assert_eq!(
        current, maximum,
        "the real Adrenaline Rush cast fills combo points"
    );
}
