//! Ravage inherits Sabertooth from Ferocious Bite's primary-impact behavior.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_ravage_applies_sabertooth_only_to_primary() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, primary, _) = prepare_class_follower(app, copy, 103, 90);
    let secondary = spawn_level_dummy(&mut app, actor, 90, 1).unwrap();
    assert!(crate::auras::has_aura(app.world(), actor, 202031));
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 441585, Some(actor), 1);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::ComboPoints, 1.0);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
    let spell =
        crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 22568).unwrap();
    crate::spell_cast::creature_cast(app.world_mut(), actor, spell, Some(primary), false).unwrap();
    let stacks = |target: Entity| {
        app.world()
            .get::<crate::auras::ActiveAuras>(target)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|a| a.spell_id == 391722 && a.caster == actor)
                    .map(|a| a.stacks)
            })
    };
    assert_eq!(
        stacks(primary),
        Some(1),
        "one paid point earns one stack once"
    );
    assert_eq!(stacks(secondary), None, "splash does not apply Sabertooth");
}
