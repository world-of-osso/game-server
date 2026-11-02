//! WCL-observed Infusion Pool must not aggro or melee nearby players.
use super::*;

#[test]
fn infusion_pool_does_not_attack_nearby_player() {
    let source = placed(spawn_of(INFANTRY), 189162, 0.0);
    let mut app = fight_app(vec![source.clone()]).expect("native faction data");
    app.init_resource::<crate::spell_cast::script_registry::SpellScriptRegistry>();
    let pool = spawned(&app, 0);
    crate::auras::ensure_aura_components(app.world_mut(), pool);
    let player = spawn_player_near(&mut app, &source, 1.0);
    let initial_health = health(&app, player).0;
    assert!(
        app.world()
            .get::<crate::creature_aggro::AggroRange>(pool)
            .is_none()
    );
    run(&mut app, NOTIFY_TICKS * 8);
    assert_eq!(health(&app, player).0, initial_health);
    assert!(app.world().get::<AutoAttack>(pool).is_none());
}
