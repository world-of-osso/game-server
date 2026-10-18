//! Quest objectives credited by spells and SmartAI, on real world.db data: item use,
//! implicit-target conditions and `smart_scripts`.

use rusqlite::{Connection, OpenFlags};
use shared::components::{Health, Rotation, UnitFactionTemplate};
use shared::protocol::{InventorySnapshot, ItemLocation};

use super::*;
use crate::maps::InMap;
use crate::world_data::{CreatureSpawn, WorldData};

/// Milly Osworth gives Extinguishing Hope (26391) and Milly's Fire Extinguisher 58362.
const MILLY_OSWORTH: u32 = 9296;
const EXTINGUISHING_HOPE: u32 = 26391;
const MILLYS_FIRE_EXTINGUISHER: u32 = 58362;
/// Northshire Vineyards Fire Trigger: SmartAI, credit on Spray Water 80208.
const VINEYARD_FIRE: u32 = 42940;
/// Spray Water (the item's 5 s channel, 80208 every second), Spray Water (60° cone,
/// 15 yd), Vineyard Fire (the trigger's burning aura) and the spell it pulses.
const SPELLS: [u32; 4] = [80199, 80208, 80175, 80187];
/// Fire trigger spawn 20279884 (TDB1210), WoW coordinates.
const FIRE_AT: [f32; 3] = [-9067.02, -328.793, 73.5351];

fn world_db_path() -> PathBuf {
    crate::test_data::world_db_path()
}

/// The quest app with combat, spells, auras and creature AI on the real spells,
/// conditions, SmartAI scripts and the fire trigger's template.
fn spell_quest_app() -> Option<App> {
    let mut app = setup_app_with(|app| {
        app.add_plugins((
            crate::combat::CombatPlugin,
            crate::spell_cast::SpellCastPlugin,
            crate::auras::AuraPlugin,
            crate::creature_ai::CreatureAiPlugin,
            crate::creature_aggro::CreatureAggroPlugin,
            crate::scripted_ai::ScriptedAiPlugin,
        ));
        // These quests run no boss encounter; skip the production world-copy load.
        app.insert_resource(crate::scripted_ai::bossmod::Catalog::default());
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(50),
        ));
        app.insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(50)));
    })?;
    app.update();
    let path = world_db_path();
    let spells = crate::spell_cast::test_harness::world_db()?;
    app.insert_resource(crate::spell_info::SpellStore::fixture(
        SPELLS
            .iter()
            .map(|&id| spells.store.get(id).expect("spell in world.db").clone())
            .collect(),
    ));
    app.insert_resource(crate::spell_cast::test_harness::power_table(spells));
    app.insert_resource(crate::faction::world_db_faction_templates()?);
    app.insert_resource(crate::conditions::load_spell_target_conditions(&path).unwrap());
    app.insert_resource(crate::scripted_ai::smart_ai::load_smart_scripts(&path).unwrap());
    app.insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap());
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates.retain(|entry, _| *entry == VINEYARD_FIRE);
    crate::world_data::load_template_models(&conn, &mut templates);
    app.insert_resource(WorldData {
        templates,
        ..Default::default()
    });
    Some(app)
}

/// The fire trigger at its TDB spawn.
fn spawn_fire(app: &mut App) -> Entity {
    let spawn = CreatureSpawn {
        guid: 20279884,
        template_entry: VINEYARD_FIRE,
        map: 0,
        zone_id: 12,
        position_x: FIRE_AT[0],
        position_y: FIRE_AT[1],
        position_z: FIRE_AT[2],
        orientation: 0.0,
        wander_distance: 0.0,
        movement_type: 0,
        spawntimesecs: 120,
        display_id: None,
        unit_flags: None,
        spawn_difficulties: vec![0],
        equipment_id: 0,
    };
    let world_data = app.world().resource::<WorldData>();
    let template = world_data.templates.get(&VINEYARD_FIRE).unwrap();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let fire = crate::zones::spawn_creature(
        &mut commands,
        world_data,
        template,
        &mut crate::zones::CreatureSpawnContext {
            map: InMap::continent(0),
            difficulty_id: crate::zones::map_difficulty_id(app.world(), InMap::continent(0)),
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        &spawn,
        &mut rand::thread_rng(),
    );
    queue.apply(app.world_mut());
    fire
}

/// Put the player `distance` yards west of the fire, facing it (WoW orientation 0 = +x).
fn stand_before_fire(app: &mut App, client: &Client, distance: f32, orientation: f32) {
    let (x, y, z) =
        crate::terrain_height::wow_to_bevy(FIRE_AT[0] - distance, FIRE_AT[1], FIRE_AT[2]);
    app.world_mut().entity_mut(client.player).insert((
        Position { x, y, z },
        Rotation {
            x: 0.0,
            y: orientation + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
        Health {
            current: 100.0,
            max: 100.0,
        },
        UnitFactionTemplate(1),
        crate::networking::Moving(false),
        crate::networking::CombatTarget::default(),
    ));
}

/// Where the bags hold `item_id`.
fn bag_location(app: &App, client: &Client, item_id: u32) -> ItemLocation {
    let templates = app.world().resource::<ItemTemplates>();
    let model = crate::inventory::load_model(db(app), templates, client.character_id).unwrap();
    let snapshot: InventorySnapshot = model.inventory_snapshot();
    snapshot
        .bags
        .iter()
        .find_map(|bag| {
            bag.items
                .iter()
                .find(|slot| slot.item.item_id == item_id)
                .map(|slot| ItemLocation::Bag {
                    bag: bag.bag,
                    slot: slot.slot,
                })
        })
        .expect("item in bags")
}

fn fires_put_out(app: &App, client: &Client) -> Option<Vec<u32>> {
    load_quest_log(db(app), client.character_id)
        .active(EXTINGUISHING_HOPE)
        .map(|active| active.counts.clone())
}

/// Accept Extinguishing Hope, then stand `distance` yards from a burning fire facing
/// `orientation`; the fire's SmartAI has run its reset.
fn extinguishing_hope(distance: f32, orientation: f32) -> Option<(App, Client, Entity)> {
    let mut app = spell_quest_app()?;
    let client = spawn_client(&mut app, 1);
    let milly = spawn_npc(&mut app, MILLY_OSWORTH, 2.0);
    enter_world(&mut app, &client);
    accept(&mut app, &client, milly, EXTINGUISHING_HOPE);
    assert_eq!(fires_put_out(&app, &client), Some(vec![0]));
    let fire = spawn_fire(&mut app);
    stand_before_fire(&mut app, &client, distance, orientation);
    for _ in 0..3 {
        app.update();
    }
    // SmartAI: RESET → phase 1, HAS_AURA (none) → Vineyard Fire on itself.
    assert!(crate::auras::has_aura(app.world(), fire, 80175));
    Some((app, client, fire))
}

fn spray(app: &mut App, client: &Client) {
    let location = bag_location(app, client, MILLYS_FIRE_EXTINGUISHER);
    request(
        app,
        client,
        UseItem {
            location,
            target: None,
        },
    );
    // Spray Water channels 5 s and sprays every second.
    for _ in 0..40 {
        app.update();
    }
}

#[test]
fn spraying_a_vineyard_fire_with_millys_extinguisher_puts_it_out_for_credit() {
    let Some((mut app, client, fire)) = extinguishing_hope(8.0, 0.0) else {
        return;
    };
    spray(&mut app, &client);
    assert_eq!(fires_put_out(&app, &client), Some(vec![1]));
    // Same hit: phase 2's SPELLHIT kills the trigger.
    assert!(app.world().get::<DeathState>(fire).is_some());
}

#[test]
fn spraying_away_from_the_fire_puts_nothing_out() {
    // Facing west, the fire behind.
    let Some((mut app, client, fire)) = extinguishing_hope(8.0, std::f32::consts::PI) else {
        return;
    };
    spray(&mut app, &client);
    assert_eq!(fires_put_out(&app, &client), Some(vec![0]));
    assert!(app.world().get::<DeathState>(fire).is_none());
}

#[test]
fn a_fire_beyond_the_spray_range_is_not_hit() {
    let Some((mut app, client, fire)) = extinguishing_hope(20.0, 0.0) else {
        return;
    };
    spray(&mut app, &client);
    assert_eq!(fires_put_out(&app, &client), Some(vec![0]));
    assert!(app.world().get::<DeathState>(fire).is_none());
}
