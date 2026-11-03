//! The world.db and DB2 definitions server startup installs. The headless
//! simulator installs the same set, so the two cannot drift apart.
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

trait Slot: Send + Sync {
    fn install(&mut self, world: &mut World);
    fn take(&mut self, world: &mut World);
}

struct Held<R: Resource>(Option<R>);

impl<R: Resource> Slot for Held<R> {
    fn install(&mut self, world: &mut World) {
        if let Some(value) = self.0.take() {
            world.insert_resource(value);
        }
    }

    fn take(&mut self, world: &mut World) {
        self.0 = world.remove_resource::<R>();
    }
}

/// Every definition `load` read. `install` moves them into a world and keeps
/// their types, so `take` can move the same set back out (seeded-world reuse).
#[derive(Resource, Default)]
pub struct WorldCatalogs(Vec<Box<dyn Slot>>);

impl WorldCatalogs {
    fn put<R: Resource>(&mut self, value: R) {
        self.0.push(Box::new(Held(Some(value))));
    }

    pub fn install(&mut self, world: &mut World) {
        for slot in &mut self.0 {
            slot.install(world);
        }
    }

    pub fn take(&mut self, world: &mut World) {
        for slot in &mut self.0 {
            slot.take(world);
        }
    }
}

/// Read every startup definition from `world_db` and the data tree `data`.
pub fn load(world_db: &Path, data: &Path) -> WorldCatalogs {
    let db2 = data.join("db2/active");
    let mut out = WorldCatalogs::default();
    out.put(crate::spell_cast::toy_requirements::load(data)
        .expect("Load current generated Toy casting data; run generate_toy_area_requirements.py and generate_toy_facing_requirements.py before startup"));
    let world = super::load_world_data(world_db);
    out.put(world.armor_constants.clone());
    out.put(world);
    let conn = Connection::open_with_flags(world_db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Failed to open world.db");
    let templates = crate::faction::load_faction_templates(&conn)
        .expect("Failed to load faction_template; run scripts/db2.py import");
    info!("Loaded {} faction templates", templates.0.len());
    out.put(templates);
    out.put(crate::area_table::load_area_table(&conn)
        .expect("Failed to load area_table; run scripts/import-area-table.py --csv <AreaTable.csv> --db <world.db> --build <source>"));
    out.put(
        crate::npc_interaction::load_gossip_catalog(&conn).expect("Failed to load gossip menus"),
    );
    out.put(
        crate::npc_interaction::load_spellclick_catalog(&conn).expect(
            "Failed to load source spellclick rows; run add_battleground_summon_templates.py",
        ),
    );
    out.put(
        crate::transmog::TransmogCatalog::load(&conn)
            .expect("Failed to load transmog source catalog"),
    );
    let vendors =
        crate::merchant::load_vendor_catalog(&conn).expect("Failed to load content_npc_vendor");
    info!("Loaded {} vendor lists", vendors.vendors.len());
    out.put(vendors);
    out.put(crate::guild_rewards::load_guild_reward_catalog(&conn).expect(
        "Failed to load tdb_guild_rewards; run scripts/import_guild_rewards.py on a world.db copy",
    ));
    out.put(
        crate::profession::load_profession_data(&conn)
            .expect("Failed to load profession data; run scripts/import-azerothcore.sh"),
    );
    out.put(
        shared::crafting_reagents::CraftingReagentCatalog::load(&db2)
            .expect("Failed to load local Retail crafting reagent slot CSVs"),
    );
    out.put(
        crate::profession::specializations::load_specialization_data(&conn)
            .expect("Failed to load profession specialization references; run scripts/profession_data.py with explicit --db and --source-dir"),
    );
    out.put(
        shared::concentration::ConcentrationCatalog::load(&db2)
            .expect("Failed to load local Retail concentration currency CSVs"),
    );
    let orders = crate::crafting_orders::load_catalog(&conn)
        .expect("Failed to load CraftingOrder reference catalog");
    if orders.0.is_empty() {
        warn!(
            "Crafting orders unavailable: run copy-only scripts/crafting_orders_import.py and select its world DB"
        );
    }
    out.put(orders);
    let loot = crate::loot::load_loot_tables(&conn)
        .expect("Failed to load content_creature_loot_template; run scripts/import-azerothcore.sh");
    info!("Loaded loot tables for {} creatures", loot.creature.len());
    out.put(loot);
    let taxi = crate::taxi::load_taxi_network(&conn)
        .expect("Failed to load taxi_nodes/taxi_path/taxi_path_node; run scripts/db2.py import");
    info!("Loaded {} flight points", taxi.nodes.len());
    out.put(taxi);
    out.put(
        crate::unit_creature_type::load_race_types(&conn)
            .expect("Failed to load chr_races.CreatureType; prepare the authored race catalog"),
    );
    let durability = crate::durability::load_item_durability(&conn)
        .expect("Failed to load item durability; run scripts/db2.py import");
    info!("Loaded durability for {} items", durability.0.len());
    out.put(durability);
    let game_objects = crate::game_objects::load_game_object_spawns(&conn)
        .expect("Failed to load content_gameobject; run scripts/import-azerothcore.sh");
    info!("Loaded {} game object spawns", game_objects.0.len());
    out.put(
        crate::pvp::arena_objects::SightCatalog::load(&conn, &game_objects)
            .expect("Load authored arena Shadow Sight metadata"),
    );
    out.put(game_objects);
    out.put(
        crate::pvp::isle_siege::Catalog::load(&conn)
            .expect("Load sourced Isle gate health; publish tdb_destructible_hitpoint first"),
    );
    out.put(
        crate::pvp::load_battleground_objects(&conn)
            .expect("Failed to load source scripted battleground templates/displays"),
    );
    let gathering = crate::gathering::load_data(&conn)
        .expect("Failed to load gathering metadata; apply scripts/gathering_metadata.py");
    info!(
        "Loaded {} gathering templates and {} skin mappings",
        gathering.nodes.len(),
        gathering.skins.len()
    );
    out.put(gathering);
    out.put(
        crate::game_objects::summoned::load_templates(&conn)
            .expect("Failed to load spell-summoned gameobject templates"),
    );
    out.put(
        crate::meeting_stone::load_meeting_stone_data(&conn)
            .expect("Failed to load meeting stone and ritual templates"),
    );
    let maps = crate::maps::store::load_map_store(&conn)
        .expect("Failed to load map data; run scripts/import-azerothcore.sh");
    info!(
        "Loaded {} maps, area trigger teleports on {} maps",
        maps.maps.len(),
        maps.area_triggers.len()
    );
    out.put(maps);
    out.put(
        crate::scripted_ai::ragefire_chasm::intro_triggers::load_triggers(&conn)
            .expect("Failed to load authored Ragefire intro areas7899/7904 from local DB2/TDB"),
    );
    out.put(
        crate::scripted_ai::nethekurse_intro::load_trigger(&conn)
            .expect("Failed to load authored Nethekurse sewer4347 from local DB2/TDB"),
    );
    out.put(
        crate::scripted_ai::kings_rest::load_molten_gold_areas(&conn)
            .expect("Failed to load authored Molten Gold area12928 from local TDB"),
    );
    out.put(
        crate::spell_cast::area_trigger::load_aura_field_catalog(&conn)
            .expect("Failed to load source aura395 area-trigger properties/actions/Curve inputs"),
    );
    out.put(
        crate::scripted_ai::kings_rest::load_gust_slash_shape(&conn).expect(
            "Failed to load Gust Slash area13339 and its orbit; run scripts/import-battlegrounds.py",
        ),
    );
    out.put(
        crate::scripted_ai::kings_rest::load_gold_model_reaches(&conn)
            .expect("Failed to load authored Kings' Rest model combat reaches"),
    );
    out.put(
        crate::scripted_ai::waypoint_path::load_script_paths(&conn)
            .expect("Failed to load TDB waypoint paths for scripted MovePath"),
    );
    let achievements = crate::achievement::load_achievement_catalog(&conn)
        .expect("Failed to load achievements; run scripts/import_achievements.py with locally extracted DB2s");
    out.put(crate::achievement::load_scenarios(&conn, &achievements).expect(
        "Failed to load scenarios; run scripts/import_scenarios.py on a private world data copy",
    ));
    out.put(crate::adventure_guide::load_catalog(
        &conn,
        &achievements,
        &db2.join("AdventureJournal.csv"),
    ));
    out.put(achievements);
    out.put(
        crate::bank::load_bank_tab_prices(&conn)
            .expect("Failed to load bank_tab; run scripts/db2.py import"),
    );
    out.put(super::load_creature_spell_stats(&conn));
    out.put(
        crate::transport_catalog::TransportCatalog::load(&conn)
            .expect("Load authored transport routes; run scripts/import-battlegrounds.py"),
    );
    out.put(
        crate::pvp::arena_objects::GateCatalog::load(&conn)
            .expect("Load source-backed arena gates; run scripts/import-arenas.py before startup"),
    );
    out.put(crate::pvp::arena_dalaran::Catalog::load(&conn).expect(
        "Load source Dalaran spout identities/pipe triggers; run scripts/import-arenas.py",
    ));
    out.put(crate::pvp::arena_cage::Catalog::load(&conn).expect(
        "Load Cage of Carnage entrance areatrigger actions; run scripts/import-battlegrounds.py",
    ));
    out.put(
        crate::pvp::random_bg::Catalog::load(&conn)
            .expect("Load configured Random32 battleground templates"),
    );
    out.put(
        crate::pvp::arena::Catalog::load(&conn)
            .expect("Load source-backed arena maps; run scripts/import-arenas.py before startup"),
    );
    out.put(
        crate::pvp::pvp_talents::Catalog::load(&conn)
            .expect("Load local PvP catalog; run scripts/import-pvp-progression.py before startup"),
    );
    out
}
