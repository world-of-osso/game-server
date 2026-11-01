//! Spiteful Reconstitution uses Core-powered Demonbolt's actual proc boundary.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_demonology_spiteful_reconstitution_summons_imp_by_twenty_first_core() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 266, 70);
    app.world_mut()
        .entity_mut(actor)
        .remove::<crate::followers::Follower>();
    app.world_mut()
        .insert_resource(crate::spell_cast::test_harness::ObservedTriggeredCasts::default());
    crate::auras::remove_auras(
        app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| matches!(aura.spell_id, 264173 | 267216),
    );
    assert!(
        crate::auras::has_aura(app.world(), actor, 428394),
        "selected Spiteful Reconstitution"
    );
    let health = app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap()
        .into_inner();
    health.current = 1_000_000_000.0;
    health.max = health.current;
    let summons = |app: &App| {
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
            .0
            .iter()
            .filter(|(caster, spell, _)| *caster == actor && *spell == 279910)
            .count()
    };
    crate::spell_cast::creature_cast(app.world_mut(), actor, 264178, Some(target), false).unwrap();
    for _ in 0..120 {
        app.update();
    }
    assert_eq!(
        summons(&app),
        0,
        "Demonbolt without Core cannot proc Spiteful Reconstitution"
    );
    let before = summons(&app);
    for _ in 0..21 {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 264173, Some(actor), 1);
        crate::spell_cast::creature_cast(app.world_mut(), actor, 264178, Some(target), false)
            .unwrap();
        for _ in 0..40 {
            app.update();
        }
    }
    assert!(
        summons(&app) > before,
        "21 real Core-powered Demonbolts guarantee at least one alternate Imp"
    );
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(264178)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}
