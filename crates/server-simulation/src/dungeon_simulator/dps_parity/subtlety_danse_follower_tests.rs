//! Danse Macabre behavior for the actual Subtlety follower.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_subtlety_danse_macabre_once_per_distinct_action_per_dance() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 261, 70);
    let mut position = app
        .world()
        .get::<shared::components::Position>(actor)
        .unwrap()
        .clone();
    position.x += 2.0;
    *app.world_mut()
        .get_mut::<shared::components::Position>(target)
        .unwrap() = position;
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    let strikes = |app: &App| {
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.spell_id == Some(1264397)
                    && event.source == Some(actor.to_bits())
                    && event.kind == CombatLogKind::Damage
            })
            .count()
    };
    let cast = |app: &mut App, spell, unit| {
        if unit == target {
            let mut position = app
                .world()
                .get::<shared::components::Position>(actor)
                .unwrap()
                .clone();
            position.x += 2.0;
            *app.world_mut()
                .get_mut::<shared::components::Position>(target)
                .unwrap() = position;
        }
        crate::spell_cast::creature_cast(app.world_mut(), actor, spell, Some(unit), false)
            .unwrap_or_else(|error| {
                panic!(
                    "spell {spell} target {unit:?}: {error:?}, actor={:?}, target={:?}",
                    app.world().get::<shared::components::Position>(actor),
                    app.world().get::<shared::components::Position>(unit)
                )
            });
        for _ in 0..30 {
            app.update();
        }
    };
    cast(&mut app, 200758, target);
    assert_eq!(strikes(&app), 0, "no Lashe Macabre outside Shadow Dance");
    cast(&mut app, 185313, actor);
    cast(&mut app, 185438, target);
    assert_eq!(
        strikes(&app),
        1,
        "first eligible generator earns one strike"
    );
    cast(&mut app, 185438, target);
    assert_eq!(
        strikes(&app),
        1,
        "the same ability cannot trigger twice in one dance"
    );
    cast(&mut app, 196819, target);
    assert_eq!(
        strikes(&app),
        2,
        "a different combo-point spender earns another strike"
    );
    crate::auras::remove_auras(
        app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 185313,
    );
    cast(&mut app, 185313, actor);
    cast(&mut app, 185438, target);
    assert_eq!(strikes(&app), 3, "a new Dance resets the ability tracker");
}
