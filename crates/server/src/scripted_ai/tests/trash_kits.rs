//! Real source trash actors must enter the existing native SmartAI cast pipeline.
use super::*;

fn trash_actor(entry: u32, map_id: u32, spell: u32) -> (Harness, Entity) {
    let mut app = stockade_app(data().expect("source world.db"));
    app.insert_resource(crate::test_data::world_data().clone());
    let source = crate::spell_cast::test_harness::world_db().unwrap();
    // TC Wild Eruption's dummy hook casts375650, whose DB2 effect32 triggers375647.
    let spells = if spell == 375652 {
        vec![spell, 375650, 375647]
    } else {
        vec![spell]
    };
    app.insert_resource(SpellStore::fixture(
        spells
            .into_iter()
            .map(|id| {
                source
                    .store
                    .get(id)
                    .expect("70448 trash ability/child")
                    .clone()
            })
            .collect(),
    ));
    app.insert_resource(smart_ai::load_smart_scripts(&world_db_path()).unwrap());
    let maps = app.world().resource::<MapStore>().clone();
    let mut instances = crate::maps::InstanceManager::default();
    let map = instances
        .create_map(
            &maps,
            map_id,
            crate::maps::instances::InstanceEntrant {
                character_id: 2001,
                leader: 2001,
                difficulty_id: 1,
                ..Default::default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 0,
            },
        )
        .unwrap()
        .map;
    app.insert_resource(instances);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawn = conn.query_row(
        "SELECT guid,position_x,position_y,position_z,orientation,spawntimesecs FROM content_creature WHERE id1=? AND map=? ORDER BY guid LIMIT 1",
        [entry, map_id],
        |row| Ok(CreatureSpawn {
            guid: row.get(0)?, template_entry: entry, map: map_id as u16, zone_id: 0,
            position_x: row.get(1)?, position_y: row.get(2)?, position_z: row.get(3)?,
            orientation: row.get(4)?, spawntimesecs: row.get(5)?,
            wander_distance: 0.0, movement_type: 0, display_id: None, unit_flags: None,
            spawn_difficulties: vec![1], equipment_id: 0,
        }),
    ).expect("authentic TDB trash spawn");
    let wd = app.world().resource::<WorldData>();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let actor = crate::zones::spawn_creature(
        &mut commands,
        wd,
        &wd.templates[&entry],
        &mut crate::zones::CreatureSpawnContext {
            map,
            difficulty_id: 1,
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        &spawn,
        &mut rand::thread_rng(),
    )
    .expect("native trash spawn");
    queue.apply(app.world_mut());
    let (player, connection) = spawn_player(
        &mut app,
        map,
        [
            spawn.position_x + 3.0,
            spawn.position_y,
            spawn.position_z,
            std::f32::consts::PI,
        ],
        1,
    );
    let mut h = Harness {
        app,
        players: vec![(player, connection, Inbox::default())],
    };
    h.advance(0.05);
    (h, actor)
}

fn assert_native_trash_cast(entry: u32, map: u32, spell: u32) {
    let (mut h, actor) = trash_actor(entry, map, spell);
    h.engage(actor);
    for _ in 0..240 {
        h.advance(0.05);
        if let Some(cast) = h.app.world().get::<CastState>(actor) {
            if cast.spell_id == spell {
                assert!(
                    cast.duration > 0.0,
                    "native DB2 cast time, not triggered cast"
                );
                let remaining = cast.duration - cast.elapsed;
                h.advance(remaining + 0.2);
                let gos: Vec<_> = h
                    .inbox(0)
                    .spell_gos
                    .iter()
                    .filter(|go| go.caster == actor.to_bits() && go.spell_id == spell)
                    .collect();
                assert!(
                    !gos.is_empty(),
                    "native cast completion missing; actual SpellGo={gos:?}, state={:?}, health={:?}",
                    h.app.world().get::<CastState>(actor),
                    h.health(actor)
                );
                return;
            }
        }
    }
    panic!("sourced trash {entry} never natively casts {spell}");
}

#[test]
fn follower_trash_azure_conjured_lasher_natively_casts_mystic_vapors() {
    // LittleWigs2777a9d TheAzureVault/Trash.lua: Conjured Lasher196102,387564.
    assert_native_trash_cast(196102, 2515, 387564);
}

#[test]
fn follower_trash_azure_arcane_tender_natively_casts_tc_wild_eruption() {
    // TC a352b1fa boss_leymor.cpp npc_arcane_tender: Wild Eruption at12s.
    assert_native_trash_cast(191164, 2515, 375652);
}

#[test]
fn follower_trash_azure_wild_eruption_releases_native_missile_damage() {
    let (mut h, actor) = trash_actor(191164, 2515, 375652);
    let before = h.health(h.player()).current;
    h.engage(actor);
    // TC first12s + native1500ms cast and native missile travel; no clock bypass.
    h.advance(15.0);
    assert!(
        h.inbox(0)
            .spell_gos
            .iter()
            .any(|go| go.caster == actor.to_bits() && go.spell_id == 375652),
        "ordinary source parent cast must complete before child assertion"
    );
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(h.player().to_bits())
                && event.spell_id == Some(375647)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "source375650 missile must release native375647 damage"
    );
    assert!(h.health(h.player()).current < before);
}

#[test]
fn follower_trash_stockade_riverpaw_shaman_natively_casts_lightning_bolt() {
    // Cached https://www.wowhead.com/npc=46252 Abilities:12167,23381; DB2 times.
    assert_native_trash_cast(46252, 34, 12167);
}
