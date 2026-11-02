use super::*;

#[test]
fn temple_avatar_first_wave_spawns_and_despawns_with_encounter() {
    let (mut h, avatar, map) = bound_boss_at_difficulty(133392, 1877, 3, 8, None);
    let source = crate::test_data::world_data()
        .templates
        .get(&262530)
        .unwrap()
        .clone();
    {
        let mut data = h.app.world_mut().resource_mut::<WorldData>();
        for (entry, name) in [
            (268344, "Corrupted Guardian"),
            (268427, "Essence Defiler"),
            (268491, "Twisted Hexxer"),
        ] {
            let mut template = source.clone();
            template.entry = entry;
            template.name = name.into();
            template.faction = 16;
            template.unit_flags = 0;
            template.unit_flags2 = 0;
            template.aggro_mode = crate::creature_aggro::AggroMode::Aggressive;
            data.templates.insert(entry, template);
        }
    }
    h.app
        .world_mut()
        .resource_mut::<MapStore>()
        .encounter_heal_objectives
        .insert((1877, 133392), 25.0);
    crate::scripted_ai::heal_objective::initialize(h.app.world_mut(), avatar);
    let max = h.health(avatar).max;
    let spells = crate::spell_cast::test_harness::world_db()
        .unwrap()
        .spell_store()
        .clone();
    h.app.insert_resource(spells);
    let player = h.player();
    {
        let mut health = h.app.world_mut().get_mut::<Health>(player).unwrap();
        health.max = 1_000_000_000.0;
        health.current = health.max;
    }
    crate::scripted_ai::heal_objective::start(h.app.world_mut(), avatar);
    h.advance(4.3);
    let guardian = h
        .app
        .world_mut()
        .query::<(Entity, &shared::components::Npc, &InMap)>()
        .iter(h.app.world())
        .find_map(|(entity, npc, copy)| {
            (*copy == map && npc.template_id == 268344).then_some(entity)
        })
        .expect("Guardian appears in the first wave");
    let position = *h.app.world().get::<Position>(guardian).unwrap();
    *h.app.world_mut().get_mut::<Position>(player).unwrap() = position;
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(guardian).insert(visibility);
    h.advance(15.7);
    assert!(
        h.inbox(0)
            .spell_gos
            .iter()
            .any(|go| go.caster == guardian.to_bits() && go.spell_id == 1300803),
        "Guardian must cast Tainted Strike"
    );
    let mut actors: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &shared::components::Npc, &InMap)>()
        .iter(h.app.world())
        .filter(|(_, npc, copy)| {
            **copy == map && [268344, 268427, 268491].contains(&npc.template_id)
        })
        .map(|(entity, npc, _)| (entity, npc.template_id))
        .collect();
    actors.sort_by_key(|(_, entry)| *entry);
    assert_eq!(
        actors.iter().map(|(_, entry)| *entry).collect::<Vec<_>>(),
        [268344, 268427, 268491]
    );
    for &(actor, entry) in &actors {
        assert!(
            h.app
                .world()
                .get::<shared::components::ThreatTable>(actor)
                .is_some_and(|threat| !threat.entries().is_empty()),
            "add {entry} must engage the party"
        );
    }
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let ctx = EffectCtx {
        caster: h.player(),
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::apply_fixed_heal(h.app.world_mut(), &ctx, avatar, max);
    h.advance(0.1);
    assert!(
        actors
            .iter()
            .all(|(entity, _)| h.app.world().get_entity(*entity).is_err()),
        "encounter adds leave when the Avatar is restored"
    );
}

#[test]
fn temple_avatar_add_delta_installs_sourced_templates_once() {
    let db = Connection::open_in_memory().unwrap();
    let path = crate::test_data::world_db_path();
    let uri = format!("file:{}?mode=ro", path.display());
    db.execute("ATTACH DATABASE ?1 AS source", [uri]).unwrap();
    for table in [
        "content_creature_template",
        "content_creature_template_model",
    ] {
        let sql: String = db
            .query_row(
                "SELECT sql FROM source.sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        db.execute_batch(&sql).unwrap();
    }
    for table in [
        "tdb_creature_template",
        "tdb_creature_template_difficulty",
        "tdb_creature_template_model",
    ] {
        db.execute_batch(&format!(
            "CREATE TEMP VIEW {table} AS SELECT * FROM source.{table};"
        ))
        .unwrap();
    }
    let sql = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/temple-avatar-adds.sql"
    ));
    db.execute_batch(sql).unwrap();
    let actors: Vec<(u32, u32)> = db
        .prepare("SELECT entry,faction FROM content_creature_template ORDER BY entry")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(actors, [(268344, 16), (268427, 16), (268491, 16)]);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM content_creature_template_model",
            [],
            |row| row.get::<_, u32>(0)
        )
        .unwrap(),
        3
    );
    db.execute_batch("COMMIT").unwrap();
    assert!(
        db.execute_batch(sql).is_err(),
        "reapplying the delta must fail its guard"
    );
}

#[test]
fn temple_avatar_healing_to_full_completes_bound_encounter() {
    let (mut h, avatar, map) = bound_boss_at_difficulty(133392, 1877, 3, 8, None);
    h.app
        .world_mut()
        .resource_mut::<MapStore>()
        .encounter_heal_objectives
        .insert((1877, 133392), 25.0);
    crate::scripted_ai::heal_objective::initialize(h.app.world_mut(), avatar);
    let max = h.health(avatar).max;
    assert!((h.health(avatar).current - max * 0.25).abs() < 1.0);
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let ctx = EffectCtx {
        caster: h.player(),
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::apply_fixed_heal(h.app.world_mut(), &ctx, avatar, max);
    assert!(
        (h.health(avatar).current - max * 0.25).abs() < 1.0,
        "healing before the encounter cannot fill the objective"
    );
    crate::scripted_ai::heal_objective::start(h.app.world_mut(), avatar);
    crate::spell_cast::effects::apply_fixed_heal(h.app.world_mut(), &ctx, avatar, max * 0.5);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map)
            & (1 << 3),
        0
    );
    crate::spell_cast::effects::apply_fixed_heal(h.app.world_mut(), &ctx, avatar, max);
    assert_eq!(h.health(avatar).current, max);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map)
            & (1 << 3),
        1 << 3
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(avatar)
            .is_none()
    );
}
