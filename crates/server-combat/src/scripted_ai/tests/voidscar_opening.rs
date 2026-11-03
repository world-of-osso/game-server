use super::*;

fn install_opening_templates(h: &mut Harness) {
    let db = Connection::open_in_memory().unwrap();
    let uri = format!("file:{}?mode=ro", world_db_path().display());
    db.execute("ATTACH DATABASE ?1 AS source", [uri]).unwrap();
    for table in [
        "content_creature_template",
        "content_creature_template_model",
        "content_inferred_creature_ability",
        "content_inferred_creature_ability_sample",
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
        "faction_template",
        "creature_display_info",
        "creature_model_data",
        "content_tuning_level",
        "map_difficulty",
        "area_table",
        "difficulty",
        "content_creature_template_difficulty",
        "tdb_smart_scripts",
        "spell_effect",
    ] {
        db.execute_batch(&format!(
            "CREATE TEMP VIEW {table} AS SELECT * FROM source.{table};"
        ))
        .unwrap();
    }
    db.execute_batch(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/voidscar-arena-templates.sql"
    )))
    .unwrap();
    db.execute_batch("COMMIT").unwrap();
    db.execute_batch(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/voidscar-opening-abilities.sql"
    )))
    .unwrap();
    let inferred = smart_ai::inferred::load(&db).unwrap();
    let levels = crate::world_data::summon_levels::SummonLevels::load(&db).unwrap();
    let mut data = h.app.world_mut().resource_mut::<WorldData>();
    for entry in [267545, 267546, 267550] {
        data.summon_levels.extend_fixture_deltas(&levels, entry);
    }
    let mut templates = crate::world_data::load_templates(&db, &data.expected_stats);
    crate::world_data::load_template_models(&db, &mut templates);
    data.templates.extend(templates);
    drop(data);
    h.app
        .world_mut()
        .resource_mut::<smart_ai::SmartScripts>()
        .4
        .extend(
            inferred
                .into_iter()
                .map(|(key, events)| (key, std::sync::Arc::new(events))),
        );
}

#[test]
fn aegyra_casts_the_sampled_spear_and_summons_it() {
    let (mut h, _, map) = bound_boss_at_difficulty(238887, 2923, 0, 23, None);
    install_opening_templates(&mut h);
    let spells = crate::spell_cast::test_harness::world_db()
        .unwrap()
        .spell_store()
        .clone();
    h.app.insert_resource(spells);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut spawn = load_authored_spawn(&conn, 238887, 2923, 23);
    spawn.guid += 100_000;
    spawn.template_entry = 267545;
    let (miniboss, _) = spawn_prepared_actor(&mut h.app, spawn, map, 23);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(miniboss).insert(visibility);
    let player = h.player();
    let position = *h.app.world().get::<Position>(miniboss).unwrap();
    *h.app.world_mut().get_mut::<Position>(player).unwrap() = position;
    h.app.world_mut().get_mut::<Health>(player).unwrap().max = 1_000_000_000.0;
    h.app.world_mut().get_mut::<Health>(player).unwrap().current = 1_000_000_000.0;
    h.advance(0.1);
    h.engage(miniboss);
    h.advance(40.0);
    assert_eq!(
        h.casts(0, miniboss, 1298908),
        1,
        "Aegyra must cast Champion's Spear once; state={:?} health={:?}",
        h.app
            .world()
            .get::<crate::creature_ai::CreatureState>(miniboss),
        h.app.world().get::<Health>(miniboss)
    );
    assert_eq!(
        h.casts(1, miniboss, 1298908),
        0,
        "a player outside the copy must not receive the cast"
    );
    assert!(
        !h.creatures_of(267550).is_empty(),
        "the cast must summon the spear"
    );
}

#[test]
fn normal_voidscar_is_not_locked_without_mythic_opening_spawns() {
    let (mut h, boss, _) = bound_boss_at_difficulty(238887, 2923, 0, 1, None);
    h.advance(0.1);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::NOT_SELECTABLE,
        0
    );
}

#[test]
fn opening_miniboss_death_unlocks_tazrah_and_casts_its_proof() {
    for (entry, proof, difficulty) in [
        (267545, 1298903, 23),
        (267546, 1298902, 23),
        (267545, 1298903, 8),
        (267546, 1298902, 8),
    ] {
        let (mut h, boss, map) = bound_boss_at_difficulty(238887, 2923, 0, 23, None);
        install_opening_templates(&mut h);
        let spells = crate::spell_cast::test_harness::world_db()
            .unwrap()
            .spell_store()
            .clone();
        h.app.insert_resource(spells);
        let conn =
            Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut spawn = load_authored_spawn(&conn, 238887, 2923, 23);
        let mut other_spawn = spawn.clone();
        other_spawn.guid += 200_000;
        let other_map = InMap {
            instance_id: map.instance_id + 1,
            ..map
        };
        let (other_boss, _) = spawn_prepared_actor(&mut h.app, other_spawn, other_map, 23);
        spawn.guid += 100_000;
        spawn.template_entry = entry;
        let (miniboss, _) = spawn_prepared_actor(&mut h.app, spawn, map, 23);
        for actor in [boss, other_boss, miniboss] {
            h.app
                .world_mut()
                .entity_mut(actor)
                .insert(crate::zones::CreatureDifficulty(difficulty));
        }
        let player = h.player();
        let position = *h.app.world().get::<Position>(miniboss).unwrap();
        *h.app.world_mut().get_mut::<Position>(player).unwrap() = position;
        h.advance(0.1);
        assert_ne!(
            h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::NOT_SELECTABLE,
            0
        );
        let health = h.health(miniboss).current;
        crate::combat::deal_direct_damage(h.app.world_mut(), player, miniboss, health + 1.0);
        h.advance(0.1);
        assert_eq!(
            h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::NOT_SELECTABLE,
            0
        );
        assert!(
            h.has_aura(player, proof),
            "entry {entry} must grant proof {proof}"
        );
        assert_ne!(
            h.app.world().get::<UnitFlags>(other_boss).unwrap().0 & UnitFlags::NOT_SELECTABLE,
            0,
            "another copy remains locked"
        );
        assert!(
            !h.has_aura(h.players[1].0, proof),
            "another copy receives no Proof"
        );
    }
}
