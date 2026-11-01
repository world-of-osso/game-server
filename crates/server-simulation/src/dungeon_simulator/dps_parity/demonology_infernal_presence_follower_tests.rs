//! Behavioral tests with the actual level-70 follower.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_demonology_charhound_deals_infernal_presence_until_despawn() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 266, 70);
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::networking::CombatTarget(Some(target)))
        .remove::<crate::followers::Follower>();
    let mut position = app
        .world()
        .get::<shared::components::Position>(actor)
        .unwrap()
        .clone();
    position.x += 2.0;
    *app.world_mut()
        .get_mut::<shared::components::Position>(target)
        .unwrap() = position;
    assert!(
        crate::auras::has_aura(app.world(), actor, 455450),
        "selected Mark of F'harg"
    );
    crate::spell_cast::cast_triggered(app.world_mut(), actor, 455476, Some(actor), 1);
    for _ in 0..20 {
        app.update();
    }
    let pet = {
        let mut query = app.world_mut().query::<(
            Entity,
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>();
        query
            .iter(app.world())
            .find(|(_, summon, npc)| {
                summon.owner == actor && summon.spell_id == 455476 && npc.template_id == 226269
            })
            .map(|(entity, _, _)| entity)
            .expect("actual Charhound")
    };
    assert!(
        crate::auras::has_aura(app.world(), pet, 428453),
        "Charhound owns native periodic Infernal Presence"
    );
    for _ in 0..180 {
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let ticks = events
        .iter()
        .filter(|event| {
            event.source == Some(pet.to_bits())
                && event.spell_id == Some(428455)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        ticks >= 3,
        "actual Charhound must land repeated Infernal Presence damage: {ticks}"
    );
    for _ in 0..140 {
        app.update();
    }
    assert!(
        app.world().get_entity(pet).is_err(),
        "Charhound and its aura end with native lifetime"
    );
}
