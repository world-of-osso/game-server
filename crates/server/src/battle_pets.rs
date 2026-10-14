use crate::auth::{Authenticated, InWorld};
use crate::collection::{CollectionPet, PlayerCollectionState, build_collection_snapshot};
use crate::creature_ai::FollowOwner;
use crate::interest::NetworkVisibility;
use crate::maps::InMap;
use crate::networking::PlayerLink;
use crate::persistence::GameDatabase;
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::{Database, ReadableDatabase, TableDefinition};
use rusqlite::{Connection, OpenFlags};
use shared::components::{
    CreatureClassification, CreatureMotion, Health, ModelDisplay, MovementSpeed, Npc, Position,
    Rotation, UnitFactionTemplate, UnitFlags, UnitLevel, UnitSummonedBy,
};
use shared::pet_battle::PetJournal;
use shared::protocol::{CollectionChannel, CollectionStateUpdate, DismissPet, SummonPet};
use std::collections::HashMap;
#[path = "pet_journal_details.rs"]
mod journal_details;
#[cfg(test)]
pub(crate) use journal_details::choose_ability;
use journal_details::receive_ability_requests;
pub(crate) use journal_details::{
    load_or_initialize_pet_states, persist_battle_progress, send_pet_details,
};

pub(crate) const ACCOUNT_PET_JOURNALS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("account_pet_journals");

/// Load by authenticated account, never character id. An absent journal is empty.
pub(crate) fn load_journal(db: &Database, account_id: u64) -> PetJournal {
    let transaction = db.begin_read().expect("begin pet journal load");
    let journal = {
        let table = match transaction.open_table(ACCOUNT_PET_JOURNALS) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return PetJournal::default(),
            Err(error) => panic!("open account pet journals: {error}"),
        };
        table
            .get(account_id)
            .expect("read account pet journal")
            .map(|value| serde_json::from_slice(value.value()).expect("decode account pet journal"))
            .unwrap_or_default()
    };
    journal
}

pub(crate) fn save_journal(db: &Database, account_id: u64, journal: &PetJournal) {
    let bytes = serde_json::to_vec(journal).expect("encode account pet journal");
    let transaction = db.begin_write().expect("begin account pet journal save");
    {
        let mut table = transaction
            .open_table(ACCOUNT_PET_JOURNALS)
            .expect("open account pet journals");
        table
            .insert(account_id, bytes.as_slice())
            .expect("save account pet journal");
    }
    transaction
        .commit()
        .expect("commit account pet journal save");
}

#[derive(Clone)]
struct Species {
    species_id: u32,
    creature_id: u32,
    name: String,
    display_id: u32,
    summon_spell_id: u32,
    can_battle: bool,
}

#[derive(Resource)]
pub(crate) struct PetCatalog {
    pub(crate) battle: ::server::pet_battle::Catalog,
    species: HashMap<u32, Species>,
    spells: HashMap<u32, u32>,
    ability_art: HashMap<u32, (String, u32)>,
    next_level_xp: HashMap<u8, u32>,
}

impl PetCatalog {
    pub(crate) fn creature_id(&self, species: u32) -> Option<u32> {
        self.species.get(&species).map(|entry| entry.creature_id)
    }

    pub(crate) fn load(conn: &Connection) -> rusqlite::Result<Self> {
        let mut query = conn.prepare(
            "SELECT s.species_id, s.creature_id, s.name, s.display_id, s.summon_spell_id, m.Flags \
             FROM battle_pet_species s JOIN battle_pet_species_metadata m ON m.ID=s.species_id ORDER BY s.species_id",
        )?;
        let entries = query
            .query_map([], |row| {
                Ok(Species {
                    species_id: row.get(0)?,
                    creature_id: row.get(1)?,
                    name: row.get(2)?,
                    display_id: row.get(3)?,
                    summon_spell_id: row.get(4)?,
                    can_battle: row.get::<_, u32>(5)? & 0x80 == 0,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let spells = entries
            .iter()
            .filter(|entry| entry.summon_spell_id != 0)
            .map(|entry| (entry.summon_spell_id, entry.species_id))
            .collect();
        let species = entries
            .into_iter()
            .map(|entry| (entry.species_id, entry))
            .collect();
        let battle = ::server::pet_battle::Catalog::load_world_db(conn)
            .map_err(rusqlite::Error::InvalidParameterName)?;
        let ability_art = conn
            .prepare("SELECT ID,Name_lang,IconFileDataID FROM battle_pet_ability")?
            .query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))?
            .collect::<rusqlite::Result<_>>()?;
        let next_level_xp = conn
            .prepare("SELECT level,next_level_xp FROM battle_pet_xp WHERE level BETWEEN 1 AND 25")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Self {
            battle,
            species,
            spells,
            ability_art,
            next_level_xp,
        })
    }
}

fn load_catalog(mut commands: Commands) {
    let conn = Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open battle-pet catalog world.db");
    let catalog = PetCatalog::load(&conn)
        .expect("load battle-pet catalog; run scripts/import-battle-pets.py first");
    info!(
        "Loaded {} BattlePetSpecies and {} battle abilities from world.db",
        catalog.species.len(),
        catalog.battle.abilities.len()
    );
    commands.insert_resource(catalog);
}

#[cfg(test)]
pub(crate) fn load_test_catalog() -> PetCatalog {
    let conn = Connection::open_with_flags(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/world.db"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open test battle-pet world.db");
    PetCatalog::load(&conn).expect("load test battle-pet catalog")
}

#[derive(Component)]
pub(crate) struct AccountPetJournal {
    pub(crate) account_id: u64,
    pub(crate) journal: PetJournal,
    summoned: Option<(u64, Entity)>,
}

#[derive(Component)]
struct Companion {
    owner: Entity,
}

pub struct BattlePetPlugin;
impl Plugin for BattlePetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (load_catalog, crate::wild_pet_battles::load_wild_catalog),
        );
        app.add_systems(
            Update,
            (
                load_account_journals,
                receive_pet_requests,
                receive_ability_requests,
                despawn_ownerless_companions,
                crate::wild_pet_battles::receive_wild_battle_requests,
                crate::wild_pet_battles::remove_abandoned_battles,
            )
                .chain(),
        );
    }
}

pub(crate) fn load_account_journals(world: &mut World) {
    let mut query = world.query_filtered::<(Entity, &Authenticated, &PlayerLink), Added<InWorld>>();
    let entered: Vec<_> = query
        .iter(world)
        .map(|(connection, account, link)| (connection, account.account_id, link.0))
        .collect();
    for (connection, account_id, owner) in entered {
        let journal = load_journal(&world.resource::<GameDatabase>().db, account_id);
        world.entity_mut(owner).insert(AccountPetJournal {
            account_id,
            journal,
            summoned: None,
        });
        load_or_initialize_pet_states(world, owner).expect("initialize owned pet battle states");
        refresh_catalog_snapshot(world, owner);
        send_pet_update(world, connection, owner, None);
    }
}

pub(crate) fn refresh_catalog_snapshot(world: &mut World, owner: Entity) {
    let Some(journal) = world.get::<AccountPetJournal>(owner) else {
        return;
    };
    let active_species = journal
        .summoned
        .and_then(|(id, _)| journal.journal.get(id))
        .map(|pet| pet.species_id);
    let mut pets: Vec<_> = world
        .resource::<PetCatalog>()
        .species
        .values()
        .map(|species| CollectionPet {
            pet_id: species.species_id,
            name: species.name.clone(),
            known: journal.journal.count_species(species.species_id) > 0,
            active: active_species == Some(species.species_id),
        })
        .collect();
    pets.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.pet_id.cmp(&right.pet_id))
    });
    world
        .get_mut::<PlayerCollectionState>(owner)
        .expect("player collection at world entry")
        .pets = pets;
}

fn receive_pet_requests(world: &mut World) {
    let mut summons = world.query_filtered::<(Entity, &PlayerLink, &mut MessageReceiver<SummonPet>), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = summons
        .iter_mut(world)
        .flat_map(|(connection, link, mut receiver)| {
            receiver
                .receive()
                .map(|request| (connection, link.0, request.pet_id))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, owner, id) in requests {
        let error = summon_companion(world, owner, id).err();
        refresh_catalog_snapshot(world, owner);
        send_pet_update(world, connection, owner, error);
    }
    receive_dismiss_requests(world);
}

fn receive_dismiss_requests(world: &mut World) {
    let mut dismissals = world.query_filtered::<(Entity, &PlayerLink, &mut MessageReceiver<DismissPet>), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = dismissals
        .iter_mut(world)
        .flat_map(|(connection, link, mut receiver)| {
            receiver
                .receive()
                .map(|_| (connection, link.0))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, owner) in requests {
        let error = dismiss_companion(world, owner).err();
        refresh_catalog_snapshot(world, owner);
        send_pet_update(world, connection, owner, error);
    }
}

pub(crate) fn send_pet_update(
    world: &mut World,
    connection: Entity,
    owner: Entity,
    error: Option<String>,
) {
    let state = world
        .get::<AccountPetJournal>(owner)
        .expect("account journal loaded");
    let message = CollectionStateUpdate {
        snapshot: world
            .get::<PlayerCollectionState>(owner)
            .map(build_collection_snapshot),
        message: None,
        error: error.clone(),
        pet_journal: Some(state.journal.clone()),
        summoned_pet_id: state.summoned.map(|(id, _)| id),
    };
    if let Some(mut sender) = world.get_mut::<MessageSender<CollectionStateUpdate>>(connection) {
        sender.send::<CollectionChannel>(message);
    }
    send_pet_details(world, connection, owner, error);
}

/// ItemEffect's ITEM_SPELLTRIGGER_LEARN_SPELL identifies the species summon spell.
pub(crate) fn learn_companion_from_item(
    world: &mut World,
    owner: Entity,
    spell: u32,
    level: u8,
) -> Result<bool, String> {
    let Some(species) = world
        .get_resource::<PetCatalog>()
        .and_then(|catalog| catalog.spells.get(&spell))
        .copied()
    else {
        return Ok(false);
    };
    let state = world
        .get::<AccountPetJournal>(owner)
        .ok_or("account pet journal unavailable")?;
    let account_id = state.account_id;
    let mut journal = state.journal.clone();
    let guid =
        crate::persistence::next_id(&world.resource::<GameDatabase>().db, "battle_pet_instance");
    journal
        .add_with_guid(
            species,
            level,
            shared::pet_battle::PetQuality::Common,
            3,
            guid,
        )
        .map_err(|reason| format!("cannot learn pet: {reason:?}"))?;
    save_journal(&world.resource::<GameDatabase>().db, account_id, &journal);
    world
        .get_mut::<AccountPetJournal>(owner)
        .expect("journal checked")
        .journal = journal;
    crate::achievement::record_learned_pet(world, owner, species);
    refresh_catalog_snapshot(world, owner);
    let mut connections = world.query::<(Entity, &PlayerLink)>();
    let connection = connections
        .iter(world)
        .find(|(_, link)| link.0 == owner)
        .map(|(connection, _)| connection);
    if let Some(connection) = connection {
        send_pet_update(world, connection, owner, None);
    }
    Ok(true)
}

fn summon_companion(world: &mut World, owner: Entity, id: u64) -> Result<Entity, String> {
    let journal = world
        .get::<AccountPetJournal>(owner)
        .ok_or("pet journal unavailable")?;
    let pet = journal.journal.get(id).ok_or("pet not owned")?.clone();
    let species = world
        .resource::<PetCatalog>()
        .species
        .get(&pet.species_id)
        .ok_or("species unavailable")?
        .clone();
    if species.display_id == 0 {
        return Err("species display unavailable".into());
    }
    let map = *world.get::<InMap>(owner).ok_or("owner map unavailable")?;
    let faction = *world
        .get::<UnitFactionTemplate>(owner)
        .ok_or("owner faction unavailable")?;
    let at = *world
        .get::<Position>(owner)
        .ok_or("owner position unavailable")?;
    let rotation = *world
        .get::<Rotation>(owner)
        .ok_or("owner rotation unavailable")?;
    // PET_FOLLOW_DIST + the owner's and follower's DEFAULT_PLAYER_COMBAT_REACH.
    const FOLLOW_REACH: f32 = 4.0;
    let angle = rotation.y + std::f32::consts::FRAC_PI_2;
    let position = Position {
        x: at.x + FOLLOW_REACH * angle.cos(),
        y: at.y,
        z: at.z - FOLLOW_REACH * angle.sin(),
    };
    if journal.summoned.is_some() {
        dismiss_companion(world, owner)?;
    }
    let entity = spawn_companion(
        world,
        owner,
        &species,
        pet.level,
        (position, rotation, map, faction),
    );
    world
        .get_mut::<AccountPetJournal>(owner)
        .expect("journal checked")
        .summoned = Some((id, entity));
    Ok(entity)
}

fn spawn_companion(
    world: &mut World,
    owner: Entity,
    species: &Species,
    level: u8,
    location: (Position, Rotation, InMap, UnitFactionTemplate),
) -> Entity {
    // Retail critter minions are immune to player/NPC attacks and never acquire threats.
    use shared::pet_battle::COMPANION_UNIT_FLAGS;
    let replicate = world.get::<Replicate>(owner).is_some();
    let mut pet = world.spawn((
        location,
        Npc {
            template_id: species.creature_id,
            name: species.name.clone(),
        },
        ModelDisplay {
            display_id: species.display_id,
        },
        Health {
            current: 1.0,
            max: 1.0,
        },
        UnitLevel(level),
        CreatureClassification::Normal,
        UnitFlags(COMPANION_UNIT_FLAGS),
        shared::protocol::NpcFlags(0),
        CreatureMotion::Still,
        MovementSpeed(0.0),
        UnitSummonedBy(owner.to_bits()),
        FollowOwner::new(owner),
        Companion { owner },
    ));
    if replicate {
        pet.insert((
            Replicate::to_clients(NetworkTarget::All),
            NetworkVisibility::default(),
        ));
    }
    pet.id()
}

fn dismiss_companion(world: &mut World, owner: Entity) -> Result<(), String> {
    let mut journal = world
        .get_mut::<AccountPetJournal>(owner)
        .ok_or("pet journal unavailable")?;
    let (_, entity) = journal.summoned.take().ok_or("no companion summoned")?;
    world.despawn(entity);
    Ok(())
}

fn despawn_ownerless_companions(world: &mut World) {
    let mut query = world.query::<(Entity, &Companion, &InMap)>();
    let expired: Vec<_> = query
        .iter(world)
        .filter(|(_, companion, map)| world.get::<InMap>(companion.owner) != Some(*map))
        .map(|(entity, companion, _)| (entity, companion.owner))
        .collect();
    for (entity, owner) in expired {
        world.despawn(entity);
        if let Some(mut journal) = world.get_mut::<AccountPetJournal>(owner) {
            journal.summoned = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::pet_battle::{PetJournal, PetJournalError, PetQuality};

    #[test]
    fn catalog_requires_imported_world_table() {
        let conn = Connection::open_in_memory().unwrap();
        let error = PetCatalog::load(&conn)
            .err()
            .expect("missing table must fail");
        assert!(
            error
                .to_string()
                .contains("no such table: battle_pet_species")
        );
    }

    #[test]
    fn imported_catalog_resolves_mechanical_squirrel_identity() {
        let catalog = load_test_catalog();
        let species = &catalog.species[&39];
        assert_eq!(species.name, "Mechanical Squirrel");
        assert_eq!(species.creature_id, 2671);
        assert_eq!(species.display_id, 7937);
        assert_eq!(catalog.spells[&4055], 39);
    }

    fn follower_app() -> (App, Entity, u64) {
        use shared::components::{Position, Rotation, UnitFactionTemplate};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(load_test_catalog());
        app.add_systems(Update, despawn_ownerless_companions);
        let mut journal = PetJournal::default();
        let id = journal
            .add_with_breed(39, 1, PetQuality::Common, 3)
            .unwrap();
        let owner = app
            .world_mut()
            .spawn((
                Position {
                    x: 10.0,
                    y: 2.0,
                    z: 20.0,
                },
                Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                crate::maps::InMap {
                    map_id: 0,
                    instance_id: 0,
                },
                UnitFactionTemplate(1),
                AccountPetJournal {
                    account_id: 42,
                    journal,
                    summoned: None,
                },
            ))
            .id();
        (app, owner, id)
    }

    #[test]
    fn summon_owned_instance_spawns_display_and_dismiss_and_logout_despawn() {
        use shared::components::{ModelDisplay, Position, UnitSummonedBy};
        let (mut app, owner, id) = follower_app();
        let pet = summon_companion(app.world_mut(), owner, id).unwrap();
        assert_eq!(
            app.world().get::<ModelDisplay>(pet).unwrap().display_id,
            7937
        );
        assert_eq!(
            app.world().get::<UnitSummonedBy>(pet).unwrap().0,
            owner.to_bits()
        );
        assert_eq!(
            app.world()
                .get::<crate::creature_ai::FollowOwner>(pet)
                .unwrap()
                .owner,
            owner
        );
        let position = app.world().get::<Position>(pet).unwrap();
        assert!((position.x - 10.0).abs() < 0.001);
        assert!((position.z - 16.0).abs() < 0.001);
        assert!(summon_companion(app.world_mut(), owner, id + 10).is_err());
        assert!(app.world().get_entity(pet).is_ok());
        dismiss_companion(app.world_mut(), owner).unwrap();
        assert!(app.world().get_entity(pet).is_err());
        let pet = summon_companion(app.world_mut(), owner, id).unwrap();
        app.world_mut().despawn(owner);
        app.update();
        assert!(app.world().get_entity(pet).is_err());
    }

    #[test]
    fn companion_follows_owner_on_authoritative_movement_ticks() {
        use shared::components::Position;
        let (mut app, owner, id) = follower_app();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(100),
        ));
        app.init_resource::<ReplicationDirtyQueue>();
        app.init_resource::<crate::interest::InterestDirtyQueue>();
        crate::creature_ai::install_follow_fixture(&mut app);
        let pet = summon_companion(app.world_mut(), owner, id).unwrap();
        app.update();
        let before = *app.world().get::<Position>(pet).unwrap();
        app.world_mut().get_mut::<Position>(owner).unwrap().x += 20.0;
        for _ in 0..10 {
            app.update();
        }
        let after = app.world().get::<Position>(pet).unwrap();
        assert!(
            after.x > before.x + 1.0,
            "follower did not move: {before:?} -> {after:?}"
        );
        assert_eq!(after.y, before.y);
    }

    #[test]
    fn another_character_loads_the_same_authenticated_account_journal() {
        let db = crate::persistence::temp_database();
        let (first, _) = crate::persistence::create_character(&db, 42, "Petone", 1, 1).unwrap();
        let (second, _) = crate::persistence::create_character(&db, 42, "Pettwo", 1, 1).unwrap();
        assert_ne!(first, second);
        let mut journal = PetJournal::default();
        journal.add_with_breed(39, 7, PetQuality::Rare, 4).unwrap();
        save_journal(&db, 42, &journal);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GameDatabase::new(db));
        app.insert_resource(load_test_catalog());
        app.add_systems(Update, load_account_journals);
        let owner = app
            .world_mut()
            .spawn(crate::collection::default_collection_state())
            .id();
        app.world_mut().spawn((
            Authenticated { account_id: 42 },
            InWorld {
                character_id: second,
            },
            PlayerLink(owner),
        ));
        app.update();
        let loaded = &app.world().get::<AccountPetJournal>(owner).unwrap().journal;
        assert_eq!(loaded, &journal);
        assert_eq!(loaded.pets[0].level, 7);
        assert_eq!(loaded.pets[0].quality, PetQuality::Rare);
    }

    #[test]
    fn companion_guids_are_unique_between_accounts_and_preserve_grant_level() {
        let (mut app, first, _) = follower_app();
        app.insert_resource(GameDatabase::new(crate::persistence::temp_database()));
        app.world_mut()
            .get_mut::<AccountPetJournal>(first)
            .unwrap()
            .journal = PetJournal::default();
        let second = app
            .world_mut()
            .spawn((
                crate::collection::default_collection_state(),
                AccountPetJournal {
                    account_id: 43,
                    journal: PetJournal::default(),
                    summoned: None,
                },
            ))
            .id();
        app.world_mut()
            .entity_mut(first)
            .insert(crate::collection::default_collection_state());
        assert!(learn_companion_from_item(app.world_mut(), first, 4055, 7).unwrap());
        assert!(learn_companion_from_item(app.world_mut(), second, 4055, 1).unwrap());
        let db = &app.world().resource::<GameDatabase>().db;
        let first_journal = load_journal(db, 42);
        let second_journal = load_journal(db, 43);
        assert_ne!(first_journal.pets[0].id, second_journal.pets[0].id);
        assert_eq!(first_journal.pets[0].level, 7);
        assert_eq!(second_journal.pets[0].level, 1);
    }

    #[test]
    fn pet_battle_startup_loads_real_stats_from_shipped_world_db() {
        let catalog = load_test_catalog();
        assert_eq!(catalog.species.get(&39).unwrap().creature_id, 2671);
        assert_eq!(
            ::server::pet_battle::calculate_stats(&catalog.battle, 39, 3, 3, 25).unwrap(),
            ::server::pet_battle::Stats {
                health: 1563,
                power: 260,
                speed: 276
            }
        );
    }

    #[test]
    fn account_journal_survives_database_close_and_character_change() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("journal.redb");
        let first_character_account = 42;
        let other_character_account = 42;
        let first_id;
        {
            let db = redb::Database::create(&path).unwrap();
            let mut journal = load_journal(&db, first_character_account);
            first_id = journal
                .add_with_breed(39, 1, PetQuality::Common, 3)
                .unwrap();
            save_journal(&db, first_character_account, &journal);
        }
        let reopened = redb::Database::open(&path).unwrap();
        let mut journal = load_journal(&reopened, other_character_account);
        assert_eq!(journal.get(first_id).unwrap().species_id, 39);
        assert_eq!(journal.get(first_id).unwrap().breed_id, 3);
        journal
            .add_with_breed(39, 1, PetQuality::Common, 3)
            .unwrap();
        journal
            .add_with_breed(39, 1, PetQuality::Common, 3)
            .unwrap();
        assert_eq!(
            journal.add_with_breed(39, 1, PetQuality::Common, 3),
            Err(PetJournalError::SpeciesLimitReached)
        );
        assert_eq!(load_journal(&reopened, 43), PetJournal::default());
    }
}
