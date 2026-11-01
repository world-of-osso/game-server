//! Transport-free access to the real Font and keystone lifecycle.
use super::*;

pub struct Session {
    _database_file: tempfile::NamedTempFile,
    connection: Entity,
    font: Entity,
    owner: u64,
    map: InMap,
    key: KeystoneInfo,
    epoch: i64,
}

impl Session {
    /// Provision a real bag key and spawn the selected copy's authored Font.
    pub fn prepare(
        world: &mut World,
        path: &std::path::Path,
        player: Entity,
        level: u32,
    ) -> Result<Self, String> {
        if level == 0 {
            return Err("keystone level must be positive".into());
        }
        if world.contains_resource::<GameDatabase>() {
            return Err("headless keystone requires an isolated character database".into());
        }
        let map = *world
            .get::<InMap>(player)
            .ok_or("keystone player has no copy")?;
        let owner = world
            .get::<crate::persistence::ClientId>(player)
            .ok_or("keystone player has no character identity")?
            .0;
        let db =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|error| format!("open keystone catalog: {error}"))?;
        let definition = load_azure(&db)
            .map_err(|error| format!("read keystone challenge: {error}"))?
            .filter(|definition| definition.map_id == map.map_id)
            .ok_or("selected dungeon has no qualified keystone challenge")?;
        let difficulty = world
            .resource::<InstanceManager>()
            .copy_difficulty(map)
            .ok_or("keystone copy has no difficulty")?
            .difficulty_id;
        let objects = crate::game_objects::load_game_object_spawns(&db)
            .map_err(|error| format!("load authored Fonts: {error}"))?;
        let fonts: Vec<_> = objects
            .0
            .into_iter()
            .filter(|object| {
                object.map_id == map.map_id
                    && object.info.go_type == 49
                    && object.spawn_difficulties.contains(&difficulty)
            })
            .collect();
        let [font] = fonts.as_slice() else {
            return Err(format!(
                "expected one authored Font in map{}, found {}",
                map.map_id,
                fonts.len()
            ));
        };
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);
        let font = crate::game_objects::spawn_game_object(&mut commands, font, map);
        queue.apply(world);
        let file = tempfile::NamedTempFile::new()
            .map_err(|error| format!("create headless character database: {error}"))?;
        let database = redb::Database::create(file.path())
            .map_err(|error| format!("open headless character database: {error}"))?;
        persistence::create_tables(&database);
        let txn = database.begin_write().map_err(|error| error.to_string())?;
        txn.open_table(KEYS).map_err(|error| error.to_string())?;
        txn.open_table(HISTORY).map_err(|error| error.to_string())?;
        txn.commit().map_err(|error| error.to_string())?;
        let key = KeystoneInfo {
            item_guid: 1,
            item_id: KEYSTONE_ITEM,
            challenge_id: definition.challenge_id,
            level,
            affixes: Vec::new(),
        };
        let mut inventory = persistence::load_inventory(&database, owner);
        inventory.items.push(persistence::InventoryItemData {
            item_guid: key.item_guid,
            item_id: key.item_id,
            name: "Mythic Keystone".into(),
            quality: 4,
            required_level: 0,
            stack_count: 1,
            vendor_sell_price: 0,
            unique_auction: false,
        });
        persistence::save_inventory(&database, owner, &inventory);
        save_key(&database, &key);
        world.insert_resource(GameDatabase::new(database));
        world.insert_resource(Challenges {
            definition: Some(definition),
            ..Default::default()
        });
        let epoch = crate::maps::ServerClock::default().unix();
        world.insert_resource(crate::maps::ServerClock {
            fixed_unix: Some(epoch),
            offset_secs: 0,
        });
        let connection = world
            .spawn((
                Authenticated { account_id: owner },
                InWorld {
                    character_id: owner,
                },
                crate::networking::PlayerLink(player),
            ))
            .id();
        Ok(Self {
            _database_file: file,
            connection,
            font,
            owner,
            map,
            key,
            epoch,
        })
    }

    #[cfg(test)]
    pub(super) fn set_test_epoch(&mut self, world: &mut World, epoch: i64) {
        self.epoch = epoch;
        world.resource_mut::<crate::maps::ServerClock>().fixed_unix = Some(epoch);
    }

    pub fn font_position(&self, world: &World) -> Result<Position, String> {
        world
            .get::<Position>(self.font)
            .copied()
            .ok_or_else(|| "authored Font has no position".into())
    }

    /// Returns false only while the player has not yet reached the Font.
    pub fn in_font_range(&self, world: &World) -> Result<bool, String> {
        let player = world
            .get::<PlayerLink>(self.connection)
            .ok_or("headless connection has no player")?
            .0;
        let font_position = world
            .get::<Position>(self.font)
            .ok_or("Font has no position")?;
        let player_position = world
            .get::<Position>(player)
            .ok_or("player has no position")?;
        let bounds = world
            .get::<crate::game_objects::GameObjectBounds>(self.font)
            .ok_or("Font has no bounds")?;
        Ok(bounds.reaches(
            crate::game_objects::wow_position(font_position),
            crate::game_objects::wow_position(player_position),
            crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE,
        ))
    }

    pub fn activate(&self, world: &mut World) -> Result<(), String> {
        open_font(world, self.connection, self.font);
        slot(
            world,
            self.connection,
            self.font.to_bits(),
            self.key.item_guid,
        )?;
        super::start(world, self.connection)
    }

    pub fn advance(&self, world: &mut World, elapsed_seconds: u64) -> Result<(), String> {
        let elapsed = i64::try_from(elapsed_seconds).map_err(|_| "simulation clock overflow")?;
        world.resource_mut::<crate::maps::ServerClock>().fixed_unix = Some(
            self.epoch
                .checked_add(elapsed)
                .ok_or("simulation clock overflow")?,
        );
        update(world);
        Ok(())
    }

    pub fn snapshot(&self, world: &World) -> Option<ChallengeRun> {
        world
            .get_resource::<Challenges>()?
            .runs
            .get(&self.map)
            .map(|active| active.run.snapshot.clone())
    }

    pub fn resulting_key(&self, world: &World) -> Result<KeystoneInfo, String> {
        owned_keys(&world.resource::<GameDatabase>().db, self.owner)
            .into_iter()
            .find(|key| key.item_guid == self.key.item_guid)
            .ok_or_else(|| "headless owner's keystone disappeared".into())
    }
}
