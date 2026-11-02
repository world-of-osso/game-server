//! Adrenaline Rush shortens an affected Outlaw follower's actual GCD.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;

fn sinister_strike_interval(adrenaline: bool) -> f64 {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 260, 70);
    for _ in 0..5 { app.update(); }
    if adrenaline {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 13750, Some(actor), 0);
        app.update();
    }
    let cast = |app: &mut bevy::prelude::App| {
        let mut pools = app.world_mut().get_mut::<crate::spell_cast::powers::PowerPools>(actor).unwrap();
        let energy = pools.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
        crate::spell_cast::creature_cast(app.world_mut(), actor, 193315, Some(target), false)
    };
    cast(&mut app).unwrap();
    let start = app.world().resource::<bevy::prelude::Time>().elapsed_secs_f64();
    for _ in 0..30 {
        app.update();
        if cast(&mut app).is_ok() {
            return app.world().resource::<bevy::prelude::Time>().elapsed_secs_f64() - start;
        }
    }
    panic!("second Sinister Strike did not become ready");
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn follower_adrenaline_rush_shortens_sinister_strike_gcd() {
    let _seed = crate::random::seed_scope(70448);
    let normal = sinister_strike_interval(false);
    let improved = sinister_strike_interval(true);
    assert!(normal >= 0.99, "normal interval: {normal}");
    assert!(improved < normal - 0.05, "normal {normal}, Adrenaline Rush {improved}");
    assert!(improved >= 0.79, "Adrenaline Rush interval {improved}");
}
