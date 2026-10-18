//! Test harness for the bank, guild bank and mail plugins: a real world.db, a temp game database, one connection whose character can be switched,
//! and inboxes for the messages the server sends.
use std::time::Duration;

use bevy::prelude::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use lightyear::prelude::{Channel, Connected, PeerId, RemoteId};
use rusqlite::{Connection, OpenFlags};
use shared::components::{CharacterAppearance, Health, Player, Position, UnitFactionTemplate};
use shared::protocol::*;

use crate::auth::{Authenticated, InWorld};
use crate::bank::{BankPlugin, load_bank_tab_prices};
use crate::faction::world_db_faction_templates;
use crate::gold::GoldUpdateQueue;
use crate::guild_bank::GuildBankPlugin;
use crate::item_templates::load_item_templates;
use crate::mail::MailPlugin;
use crate::networking::PlayerLink;
use crate::npc_interaction::{NpcInteractionPlugin, load_gossip_catalog};
use crate::persistence::{self, ClientId, GameDatabase};

pub(crate) const ACCOUNT: u64 = 1;
const HUMAN_TEMPLATE: u32 = 1;
pub(crate) const LINEN: u32 = 2589;
pub(crate) const PEACEBLOOM: u32 = 2447;
pub(crate) const COPPER_ORE: u32 = 2770;

/// Messages the server sent, with the connection they went to.
#[derive(Resource)]
struct Inbox<M>(Vec<(Entity, M)>);

fn collect<M: lightyear::prelude::Message>(
    mut receivers: Query<(Entity, &mut MessageReceiver<M>)>,
    mut inbox: ResMut<Inbox<M>>,
) {
    for (connection, mut receiver) in &mut receivers {
        inbox
            .0
            .extend(receiver.receive().map(|message| (connection, message)));
    }
}

fn add_inbox<M: lightyear::prelude::Message>(app: &mut App) {
    app.insert_resource(Inbox::<M>(Vec::new()));
    app.add_systems(First, collect::<M>);
}

fn pair<M: lightyear::prelude::Message>() -> (MessageSender<M>, MessageReceiver<M>) {
    (MessageSender::default(), MessageReceiver::default())
}

pub(crate) struct Harness {
    pub app: App,
    pub conn: Connection,
    pub connection: Entity,
    pub player: Entity,
    pub character_id: u64,
}

pub(crate) fn harness() -> Option<Harness> {
    harness_with(persistence::temp_database(), "Banker")
}

/// A harness on the game database file at `path`, created or reopened as a server
/// start does; its first character is `name`.
pub(crate) fn harness_at(path: &std::path::Path, name: &str) -> Option<Harness> {
    let db = redb::Database::create(path).unwrap();
    persistence::create_tables(&db);
    harness_with(db, name)
}

fn harness_with(db: redb::Database, name: &str) -> Option<Harness> {
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    add_inbox::<InteractionOpened>(&mut app);
    add_inbox::<InteractionClosed>(&mut app);
    add_inbox::<InteractionFailed>(&mut app);
    add_inbox::<BankContents>(&mut app);
    add_inbox::<BankFailed>(&mut app);
    add_inbox::<GuildBankContents>(&mut app);
    add_inbox::<GuildRanksState>(&mut app);
    add_inbox::<GuildInvitation>(&mut app);
    add_inbox::<GuildInviteResponse>(&mut app);
    add_inbox::<GuildBankFailed>(&mut app);
    add_inbox::<GuildBankLog>(&mut app);
    add_inbox::<MailboxContents>(&mut app);
    add_inbox::<MailSent>(&mut app);
    add_inbox::<MailFailed>(&mut app);
    add_inbox::<PendingMail>(&mut app);
    add_inbox::<InventoryError>(&mut app);
    app.insert_resource(world_db_faction_templates()?);
    app.insert_resource(load_gossip_catalog(&conn).unwrap());
    app.insert_resource(load_item_templates(&path).unwrap());
    app.insert_resource(load_bank_tab_prices(&conn).unwrap());
    app.insert_resource(GameDatabase::new(db));
    app.init_resource::<GoldUpdateQueue>();
    app.add_plugins((
        NpcInteractionPlugin,
        BankPlugin,
        GuildBankPlugin,
        crate::guild::GuildPlugin,
        MailPlugin,
        crate::inventory::InventoryPlugin,
    ));
    app.finish();
    app.cleanup();
    let character_id = create_character(&mut app, name);
    let (connection, player) = spawn_connection(&mut app, 1, character_id, name);
    app.update();
    Some(Harness {
        app,
        conn,
        connection,
        player,
        character_id,
    })
}

/// A logged-in connection (peer `peer`) whose player is `name` at the origin.
pub(crate) fn spawn_connection(
    app: &mut App,
    peer: u64,
    character_id: u64,
    name: &str,
) -> (Entity, Entity) {
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            ClientId(character_id),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitFactionTemplate(HUMAN_TEMPLATE),
            crate::maps::InMap::continent(0),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(peer)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated {
                account_id: ACCOUNT,
            },
            InWorld { character_id },
            PlayerLink(player),
            (
                pair::<InteractNpc>(),
                pair::<SelectGossipOption>(),
                pair::<UseGameObject>(),
                pair::<CloseInteraction>(),
                pair::<InteractionOpened>(),
                pair::<InteractionClosed>(),
                pair::<InteractionFailed>(),
            ),
            (
                pair::<BankContents>(),
                pair::<BankFailed>(),
                pair::<BankDeposit>(),
                pair::<BankWithdraw>(),
                pair::<BankPurchaseTab>(),
                pair::<BankMoneyTransfer>(),
                pair::<BankAutoDeposit>(),
                pair::<BankUpdateTabSettings>(),
            ),
            (
                pair::<GuildBankContents>(),
                pair::<GuildRankRequest>(),
                pair::<GuildRanksState>(),
                pair::<GuildInvitation>(),
                pair::<GuildInviteRequest>(),
                pair::<GuildInviteResponse>(),
                pair::<GuildBankLog>(),
                pair::<GuildBankFailed>(),
                pair::<GuildBankDeposit>(),
                pair::<GuildBankWithdraw>(),
                pair::<GuildBankMoneyTransfer>(),
                pair::<GuildBankBuyTab>(),
                pair::<GuildBankSetTabInfo>(),
                pair::<GuildBankSetTabText>(),
                pair::<GuildBankQueryLog>(),
            ),
            (
                pair::<MailboxContents>(),
                pair::<MailSent>(),
                pair::<MailFailed>(),
                pair::<PendingMail>(),
                pair::<SendMail>(),
                pair::<MailRequest>(),
            ),
            (
                pair::<InventorySnapshot>(),
                pair::<EquipmentSnapshot>(),
                pair::<InventoryDelta>(),
                pair::<InventoryError>(),
                pair::<SwapItem>(),
                pair::<SplitItem>(),
            ),
        ))
        .id();
    (connection, player)
}

fn create_character(app: &mut App, name: &str) -> u64 {
    let db = &app.world().resource::<GameDatabase>().db;
    persistence::create_character_with_test_bags(db, ACCOUNT, name, 1, 1)
        .expect("create character")
        .0
}

impl Harness {
    pub fn db(&self) -> &redb::Database {
        &self.app.world().resource::<GameDatabase>().db
    }

    /// A second character on the same account.
    pub fn create_alt(&mut self, name: &str) -> u64 {
        create_character(&mut self.app, name)
    }

    /// Log the connection into `character_id` (the account stays the same).
    pub fn play_as(&mut self, character_id: u64) {
        self.character_id = character_id;
        let connection = self.connection;
        self.app
            .world_mut()
            .entity_mut(connection)
            .insert(InWorld { character_id });
        let player = self.player;
        self.app
            .world_mut()
            .entity_mut(player)
            .insert(ClientId(character_id));
        self.app.update();
    }

    pub fn send<M: lightyear::prelude::Message, C: Channel>(&mut self, message: M) {
        let connection = self.connection;
        self.send_from::<M, C>(connection, message);
    }

    /// Send as the connection `connection`.
    pub fn send_from<M: lightyear::prelude::Message, C: Channel>(
        &mut self,
        connection: Entity,
        message: M,
    ) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<M>>(connection)
            .unwrap()
            .send::<C>(message);
        for _ in 0..3 {
            self.app.update();
        }
    }

    /// What the server sent the harness connection since the last take.
    pub fn take<M: lightyear::prelude::Message>(&mut self) -> Vec<M> {
        let connection = self.connection;
        self.take_for(connection)
    }

    /// What the server sent `connection` since the last take.
    pub fn take_for<M: lightyear::prelude::Message>(&mut self, connection: Entity) -> Vec<M> {
        let mut inbox = self.app.world_mut().resource_mut::<Inbox<M>>();
        let (taken, kept) = std::mem::take(&mut inbox.0)
            .into_iter()
            .partition(|(to, _)| *to == connection);
        inbox.0 = kept;
        taken.into_iter().map(|(_, message)| message).collect()
    }

    /// Log a second character of the account in on its own connection; returns it.
    pub fn connect_character(&mut self, character_id: u64, name: &str) -> Entity {
        let peer = 1_000 + character_id;
        let (connection, _) = spawn_connection(&mut self.app, peer, character_id, name);
        self.app.update();
        connection
    }

    pub fn move_player(&mut self, x: f32) {
        let player = self.player;
        self.app
            .world_mut()
            .entity_mut(player)
            .insert(Position { x, y: 0.0, z: 0.0 });
        for _ in 0..3 {
            self.app.update();
        }
    }

    pub fn inventory(&self) -> persistence::InventoryData {
        persistence::load_inventory(self.db(), self.character_id)
    }

    pub fn set_gold(&mut self, gold: u64) {
        let db = self.db();
        let mut inventory = persistence::load_inventory(db, self.character_id);
        inventory.gold = gold;
        persistence::save_inventory(db, self.character_id, &inventory);
    }

    pub fn gold(&self) -> u64 {
        self.inventory().gold
    }

    pub fn count_of(&self, item_id: u32) -> u32 {
        self.inventory()
            .items
            .iter()
            .filter(|item| item.item_id == item_id)
            .map(|item| item.stack_count)
            .sum()
    }

    pub fn guid_of(&self, item_id: u32) -> u64 {
        self.inventory()
            .items
            .iter()
            .find(|item| item.item_id == item_id)
            .expect("item in bags")
            .item_guid
    }

    pub fn make_soulbound(&self, item_guid: u64) {
        use crate::persistence::inventory_store::{ITEM_INSTANCES, ItemInstanceData};
        let txn = self.db().begin_write().unwrap();
        txn.open_table(ITEM_INSTANCES)
            .unwrap()
            .insert(
                item_guid,
                bitcode::encode(&ItemInstanceData { soulbound: true }).as_slice(),
            )
            .unwrap();
        txn.commit().unwrap();
    }

    /// Spawn Newton Burnside (Stormwind banker) 2 yd away and open his bank.
    pub fn open_banker(&mut self) -> Entity {
        const NEWTON_BURNSIDE: u32 = 2456;
        let (name, faction, npcflag): (String, u32, u32) = self
            .conn
            .query_row(
                "SELECT name, faction, npcflag FROM content_creature_template WHERE entry = ?1",
                [NEWTON_BURNSIDE],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let npc = self
            .app
            .world_mut()
            .spawn((
                shared::components::Npc {
                    template_id: NEWTON_BURNSIDE,
                    name,
                },
                NpcFlags::from_azerothcore(npcflag),
                Position {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                UnitFactionTemplate(faction),
                crate::maps::InMap::continent(0),
            ))
            .id();
        self.send::<_, InteractionChannel>(InteractNpc { npc: npc.to_bits() });
        // Gossip menu 699: "I would like to check my deposit box." (option 0, type 9).
        self.send::<_, InteractionChannel>(SelectGossipOption {
            npc: npc.to_bits(),
            option_id: 0,
        });
        // The opened banker's contents go out one frame after the role opens.
        self.app.update();
        npc
    }

    /// Spawn the Stormwind Guild Vault 3 yd away and use it.
    pub fn open_vault(&mut self) -> Entity {
        let spawns = crate::game_objects::load_game_object_spawns(&self.conn).unwrap();
        let mut spawn = spawns
            .0
            .into_iter()
            .find(|spawn| spawn.info.entry == 205107)
            .expect("Stormwind Guild Vault (TrinityCore entry 205107)");
        spawn.position = Vec3::new(3.0, 0.0, 0.0);
        spawn.orientation = 0.0;
        spawn.bounds.orientation = 0.0;
        let object = self
            .app
            .world_mut()
            .spawn(crate::game_objects::game_object_bundle(&spawn))
            .id();
        self.send::<_, InteractionChannel>(UseGameObject {
            object: object.to_bits(),
        });
        // The guild bank goes out one frame after the role opens.
        self.app.update();
        object
    }

    /// `connection` uses the already spawned game object `object`.
    pub fn use_object_from(&mut self, connection: Entity, object: Entity) {
        self.send_from::<_, InteractionChannel>(
            connection,
            UseGameObject {
                object: object.to_bits(),
            },
        );
        self.app.update();
    }

    /// Spawn the Goldshire mailbox (TrinityCore guid 209585 + 20,000,000) 2 yd away and use it.
    pub fn open_mailbox(&mut self) -> Entity {
        let spawns = crate::game_objects::load_game_object_spawns(&self.conn).unwrap();
        let mut spawn = spawns
            .0
            .into_iter()
            .find(|spawn| spawn.guid == 20_209_585)
            .expect("Goldshire mailbox");
        spawn.position = Vec3::new(2.0, 0.0, 0.0);
        spawn.orientation = 0.0;
        spawn.bounds.orientation = 0.0;
        let object = self
            .app
            .world_mut()
            .spawn(crate::game_objects::game_object_bundle(&spawn))
            .id();
        self.send::<_, InteractionChannel>(UseGameObject {
            object: object.to_bits(),
        });
        // The inbox goes out one frame after the role opens.
        self.app.update();
        object
    }

    /// Spawn the game object of spawn `guid` 2 yd away and use it.
    pub fn use_game_object(&mut self, guid: u32) -> Entity {
        let spawns = crate::game_objects::load_game_object_spawns(&self.conn).unwrap();
        let mut spawn = spawns
            .0
            .into_iter()
            .find(|spawn| spawn.guid == guid)
            .unwrap_or_else(|| panic!("game object spawn {guid}"));
        spawn.position = Vec3::new(2.0, 0.0, 0.0);
        spawn.orientation = 0.0;
        spawn.bounds.orientation = 0.0;
        let object = self
            .app
            .world_mut()
            .spawn(crate::game_objects::game_object_bundle(&spawn))
            .id();
        self.send::<_, InteractionChannel>(UseGameObject {
            object: object.to_bits(),
        });
        self.app.update();
        object
    }

    /// A character on another account.
    pub fn create_on_account(&mut self, account_id: u64, name: &str, race: u8) -> u64 {
        let db = &self.app.world().resource::<GameDatabase>().db;
        persistence::create_character_with_test_bags(db, account_id, name, race, 1)
            .expect("create character")
            .0
    }
}
