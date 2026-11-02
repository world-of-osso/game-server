//! Selected Dimensional Rift uses the actual follower's landed spender.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_destruction_dimensional_rift_summons_a_damaging_guardian() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 267, 70);
    assert!(
        app.world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .contains(1280868)
    );
    let templates = &app
        .world()
        .resource::<crate::world_data::WorldData>()
        .templates;
    for entry in [198547, 196280, 198555] {
        assert!(
            templates.contains_key(&entry),
            "rift template {entry} missing from loaded world"
        );
    }
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    app.world_mut()
        .insert_resource(crate::spell_cast::test_harness::ObservedTriggeredCasts::default());
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    let mut guardians = std::collections::BTreeSet::new();
    for _ in 0..2400 {
        app.update();
        for (unit, summon, npc) in app
            .world_mut()
            .query::<(
                Entity,
                &crate::spell_cast::summon::SummonedCreature,
                &shared::components::Npc,
            )>()
            .iter(app.world())
        {
            if summon.owner == actor {
                guardians.insert((summon.spell_id, npc.template_id, unit.to_bits()));
            }
        }
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let rift_damage: Vec<_> = events
        .iter()
        .filter(|event| {
            matches!(event.spell_id, Some(394238 | 387985 | 394246 | 434589))
                && event
                    .source
                    .is_some_and(|source| guardians.iter().any(|entry| entry.2 == source))
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect();
    let triggered = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
        .0;
    let relevant: Vec<_> = triggered
        .iter()
        .filter(|(_, spell, _)| {
            matches!(
                spell,
                394235 | 387979 | 394243 | 434587 | 394238 | 387985 | 394246 | 434589
            )
        })
        .collect();
    let actions: Vec<_> = events
        .iter()
        .filter(|event| {
            event
                .source
                .is_some_and(|source| guardians.iter().any(|entry| entry.2 == source))
        })
        .map(|event| (event.source, event.spell_id, &event.kind, event.target))
        .collect();
    assert!(
        !rift_damage.is_empty(),
        "selected Dimensional Rift must produce an actual damaging guardian: guardians={guardians:?} triggered={relevant:?} actions={actions:?}"
    );
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Rift templates"]
fn follower_destruction_rift_tears_cast_without_melee() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 267, 70);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    start_fixed_time_fight(&mut app, owner, actor, target, 120.0);
    app.world_mut()
        .entity_mut(actor)
        .remove::<server_follower_apl::FollowerApl>();
    let sources = [
        (394235, 198547, 394238),
        (387979, 196280, 387985),
        (394243, 198555, 394246),
    ];
    let mut actors = Vec::new();
    for (summon, entry, damage) in sources {
        assert!(crate::spell_cast::try_cast_triggered(
            app.world_mut(),
            actor,
            summon,
            Some(target),
            1
        ));
        let unit = app
            .world_mut()
            .query::<(
                Entity,
                &crate::spell_cast::summon::SummonedCreature,
                &shared::components::Npc,
            )>()
            .iter(app.world())
            .find(|(_, s, n)| s.owner == actor && s.spell_id == summon && n.template_id == entry)
            .map(|(unit, _, _)| unit)
            .expect("native Rift guardian");
        actors.push((unit, damage));
    }
    for _ in 0..400 {
        app.update();
    }
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for (unit, damage) in actors {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(unit.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(damage)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "Rift {unit:?} must deal native {damage}"
        );
        assert!(
            !events
                .iter()
                .any(|event| event.source == Some(unit.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage),
            "Rift {unit:?} must not melee"
        );
    }
}
