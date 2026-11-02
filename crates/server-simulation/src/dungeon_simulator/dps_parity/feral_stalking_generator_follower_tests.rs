//! Stalking Predator's guaranteed Unseen uses belong to combo generators.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with class follower data"]
fn feral_follower_stalking_uses_are_not_spent_by_rip() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("measurement snapshot");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 103, 90);
    assert!(
        crate::auras::has_aura(app.world(), actor, 1263660),
        "Stalking selected"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 768, Some(actor), 1);
    crate::spell_cast::creature_cast(app.world_mut(), actor, 5217, Some(actor), false).unwrap();
    let stacks = |app: &bevy::prelude::App| {
        app.world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 1263962 && a.caster == actor)
            .map(|a| a.stacks)
    };
    assert_eq!(stacks(&app), Some(2));
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::spell_cast::SpellHistory>();
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::ComboPoints, 5.0);
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
    crate::spell_cast::creature_cast(app.world_mut(), actor, 1079, Some(target), false).unwrap();
    assert_eq!(
        stacks(&app),
        Some(2),
        "Rip is a finisher and cannot spend a Stalking use"
    );
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::spell_cast::SpellHistory>();
    crate::spell_cast::powers::change_power(app.world_mut(), actor, PowerType::Energy, 100.0);
    crate::spell_cast::creature_cast(app.world_mut(), actor, 5221, Some(target), false).unwrap();
    assert_eq!(
        stacks(&app),
        Some(1),
        "the real Shred generator spends exactly one use"
    );
}
