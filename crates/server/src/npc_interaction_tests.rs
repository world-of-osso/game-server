//! Real-DB tests against data/world.db.

use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{Channel, Message};
use rusqlite::{Connection, OpenFlags};
use shared::protocol::{
    AuctionChannel, AuctionHouseOpened, AuctionOperationResponse, AuctionSearchQuery,
    AuctionSearchResults, AuctionSortDir, AuctionSortField, GossipMenu, GossipMenuOption,
    OpenAuctionHouse, ProtocolPlugin, QueryAuctions,
};

use super::*;
use crate::faction::world_db_faction_templates;
use crate::gold::GoldUpdateQueue;

#[path = "npc_interaction/guild_tabard_tests.rs"]
mod guild_tabard_tests;
#[path = "transmog_tests.rs"]
mod transmog_tests;

#[path = "housing_interaction_tests.rs"]
mod housing_interaction_tests;

const HUMAN_TEMPLATE: u32 = 1;
const ORC_TEMPLATE: u32 = 2;
/// Auctioneer Fitch, Stormwind Trade District (creature guid 79705, npcflag 0x200000, faction 12).
const AUCTIONEER_FITCH: u32 = 8719;
/// Auctioneer Drezmit, Orgrimmar (TrinityCore-only template: Retail Orgrimmar no longer
/// spawns AzerothCore's Auctioneer Wabang; npcflag auctioneer | questgiver | gossip, faction 29).
const AUCTIONEER_DREZMIT: u32 = 44866;
/// Innkeeper Farley, Goldshire (npcflag gossip | questgiver | vendor | innkeeper, gossip menu 1291).
const INNKEEPER_FARLEY: u32 = 295;
/// Innkeeper Gryshka, Orgrimmar (faction 29, hostile to Humans).
const INNKEEPER_GRYSHKA: u32 = 6929;

fn world_db() -> Option<Connection> {
    let path = crate::test_data::world_db_path();
    Some(Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap())
}

/// Server-to-client messages drained in `First`, before lightyear's `Last` clear.
#[derive(Resource)]
struct Inbox<M>(Vec<M>);

fn collect<M: Message>(mut receivers: Query<&mut MessageReceiver<M>>, mut inbox: ResMut<Inbox<M>>) {
    for mut receiver in &mut receivers {
        inbox.0.extend(receiver.receive());
    }
}

fn add_inbox<M: Message>(app: &mut App) {
    app.insert_resource(Inbox::<M>(Vec::new()));
    app.add_systems(First, collect::<M>);
}

struct Harness {
    app: App,
    conn: Connection,
    player: Entity,
    connection: Entity,
}

fn harness(player_faction: u32) -> Option<Harness> {
    harness_with(player_faction, |_| {})
}

fn harness_with(player_faction: u32, extra: impl FnOnce(&mut App)) -> Option<Harness> {
    let conn = world_db()?;
    let templates = world_db_faction_templates()?;
    let catalog = load_gossip_catalog(&conn).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    add_inbox::<InteractionOpened>(&mut app);
    add_inbox::<InteractionFailed>(&mut app);
    add_inbox::<InteractionClosed>(&mut app);
    add_inbox::<AuctionHouseOpened>(&mut app);
    add_inbox::<AuctionSearchResults>(&mut app);
    add_inbox::<AuctionOperationResponse>(&mut app);
    app.insert_resource(templates);
    app.insert_resource(crate::persistence::GameDatabase::new(
        crate::persistence::temp_database(),
    ));
    app.init_resource::<GoldUpdateQueue>();
    app.add_plugins((
        NpcInteractionPlugin,
        crate::auction_house::AuctionHousePlugin,
    ));
    app.insert_resource(catalog);
    extra(&mut app);
    app.finish();
    app.cleanup();
    let player = app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitFactionTemplate(player_faction),
            crate::maps::InMap::continent(0),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            InWorld { character_id: 1 },
            PlayerLink(player),
            message_pairs(),
        ))
        .id();
    app.update();
    Some(Harness {
        app,
        conn,
        player,
        connection,
    })
}

type Pair<M> = (MessageSender<M>, MessageReceiver<M>);

fn pair<M: Message>() -> Pair<M> {
    (MessageSender::default(), MessageReceiver::default())
}

type InteractionPairs = (
    Pair<InteractNpc>,
    Pair<SelectGossipOption>,
    Pair<CloseInteraction>,
    Pair<InteractionOpened>,
    Pair<InteractionFailed>,
    Pair<InteractionClosed>,
);

type AuctionPairs = (
    Pair<OpenAuctionHouse>,
    Pair<AuctionHouseOpened>,
    Pair<QueryAuctions>,
    Pair<AuctionSearchResults>,
    Pair<AuctionOperationResponse>,
);

fn message_pairs() -> (InteractionPairs, AuctionPairs) {
    (
        (pair(), pair(), pair(), pair(), pair(), pair()),
        (pair(), pair(), pair(), pair(), pair()),
    )
}

impl Harness {
    /// Spawn a creature from its world.db template at its first spawn point and put
    /// the player `offset` yards east of it.
    fn spawn_real_npc(&mut self, entry: u32, offset: f32) -> Entity {
        let (name, faction, npcflag, x, y, z,map): (String, u32, u32, f32, f32, f32,u32) = self
            .conn
            .query_row(
                "SELECT t.name, t.faction, t.npcflag, c.position_x, c.position_y, c.position_z,c.map
                 FROM content_creature_template t JOIN content_creature c ON c.id1 = t.entry
                 WHERE t.entry = ?1 ORDER BY c.guid LIMIT 1",
                [entry],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .unwrap();
        let (bx, by, bz) = crate::terrain_height::wow_to_bevy(x, y, z);
        let npc = self
            .app
            .world_mut()
            .spawn((
                Npc {
                    template_id: entry,
                    name,
                },
                NpcFlags(u64::from(npcflag)),
                Position {
                    x: bx,
                    y: by,
                    z: bz,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                UnitFactionTemplate(faction),
                crate::maps::InMap::continent(map),
            ))
            .id();
        self.place_player(npc, offset);
        npc
    }

    fn place_player(&mut self, npc: Entity, offset: f32) {
        let npc_position = *self.app.world().get::<Position>(npc).unwrap();
        let map = *self.app.world().get::<crate::maps::InMap>(npc).unwrap();
        self.app.world_mut().entity_mut(self.player).insert(map);
        *self
            .app
            .world_mut()
            .get_mut::<Position>(self.player)
            .unwrap() = Position {
            x: npc_position.x + offset,
            ..npc_position
        };
    }

    fn send<M: Message, C: Channel>(&mut self, message: M) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<M>>(self.connection)
            .unwrap()
            .send::<C>(message);
        self.advance();
    }

    /// Send + HostClient loopback delivery takes 3 frames.
    fn advance(&mut self) {
        for _ in 0..3 {
            self.app.update();
        }
    }

    fn interact(&mut self, npc: Entity) {
        self.send::<_, InteractionChannel>(InteractNpc { npc: npc.to_bits() });
    }

    fn select(&mut self, npc: Entity, option_id: u32) {
        self.send::<_, InteractionChannel>(SelectGossipOption {
            npc: npc.to_bits(),
            option_id,
        });
    }

    fn browse(&mut self) {
        self.send::<_, AuctionChannel>(QueryAuctions {
            query: AuctionSearchQuery {
                item_id: None,
                class_id: None,
                subcategory_filters: Vec::new(),
                text: String::new(),
                page: 0,
                page_size: 50,
                min_level: None,
                max_level: None,
                quality: None,
                usable_only: false,
                sort_field: AuctionSortField::Name,
                sort_dir: AuctionSortDir::Asc,
                faction: 0,
            },
        });
    }

    fn take<M: Message>(&mut self) -> Vec<M> {
        std::mem::take(&mut self.app.world_mut().resource_mut::<Inbox<M>>().0)
    }

    fn session(&self) -> Option<&ActiveInteraction> {
        self.app.world().get::<ActiveInteraction>(self.connection)
    }
}

#[test]
fn crafting_broker_opens_customer_service_and_revalidates_range() {
    let mut h = harness(HUMAN_TEMPLATE).unwrap();
    let (name, flags, faction): (String, u64, u32) = h
        .conn
        .query_row(
            "SELECT name,npcflag,faction FROM tdb_creature_template WHERE entry=215258",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let broker = h
        .app
        .world_mut()
        .spawn((
            Npc {
                template_id: 215258,
                name,
            },
            NpcFlags(flags),
            Position {
                x: 0.0,
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
    h.interact(broker);
    let events = h.take::<InteractionOpened>();
    let InteractionKind::Gossip(menu) = &events[0].kind else {
        panic!("broker must open authored gossip")
    };
    assert_eq!(menu.menu_id, 30243);
    assert_eq!(menu.options[0].text, "I'd like to create a crafting order.");
    h.select(broker, menu.options[0].option_id);
    assert_eq!(
        h.take::<InteractionOpened>(),
        vec![opened(
            broker,
            InteractionKind::Role(NpcRole::CraftingOrders)
        )]
    );
    assert!(find_interactable_crafting_broker(h.app.world(), h.player, broker).is_ok());
    h.place_player(broker, INTERACTION_DISTANCE + 1.0);
    assert_eq!(
        find_interactable_crafting_broker(h.app.world(), h.player, broker).unwrap_err(),
        InteractionError::TooFarAway
    );
    h.advance();
    assert_eq!(
        h.take::<InteractionClosed>(),
        vec![InteractionClosed {
            npc: broker.to_bits()
        }]
    );
}

fn opened(npc: Entity, kind: InteractionKind) -> InteractionOpened {
    InteractionOpened {
        npc: npc.to_bits(),
        kind,
    }
}

fn option(option_id: u32, icon: u8, text: &str) -> GossipMenuOption {
    GossipMenuOption {
        option_id,
        icon,
        text: text.into(),
    }
}

#[test]
fn stormwind_auctioneer_opens_alliance_auction_house_within_range_only() {
    let Some(mut h) = harness(HUMAN_TEMPLATE) else {
        return;
    };
    let fitch = h.spawn_real_npc(AUCTIONEER_FITCH, 3.0);
    h.interact(fitch);
    assert_eq!(
        h.take::<InteractionOpened>(),
        vec![opened(fitch, InteractionKind::Role(NpcRole::AuctionHouse))]
    );
    assert_eq!(
        h.session().unwrap().auction_house,
        Some(AUCTION_HOUSE_ALLIANCE)
    );

    h.send::<_, InteractionChannel>(CloseInteraction {
        npc: fitch.to_bits(),
    });
    assert!(h.session().is_none());
    h.place_player(fitch, INTERACTION_DISTANCE + 1.0);
    h.interact(fitch);
    assert!(h.take::<InteractionOpened>().is_empty());
    assert_eq!(
        h.take::<InteractionFailed>(),
        vec![InteractionFailed {
            npc: fitch.to_bits(),
            error: InteractionError::TooFarAway,
        }]
    );
    assert!(h.session().is_none());
}

#[test]
fn orgrimmar_auctioneer_with_quest_flag_offers_gossip_option_to_horde_auction_house() {
    let Some(mut h) = harness(ORC_TEMPLATE) else {
        return;
    };
    let drezmit = h.spawn_real_npc(AUCTIONEER_DREZMIT, 2.0);
    h.interact(drezmit);
    // TrinityCore menu 12949: a quest-flavour option (GossipOptionNpc None) and the
    // auction option (GossipOptionNpc Auctioneer → type 13, npcflag auctioneer, icon 6).
    assert_eq!(
        h.take::<InteractionOpened>(),
        vec![opened(
            drezmit,
            InteractionKind::Gossip(GossipMenu {
                menu_id: 12949,
                text: "Feel free to browse the items up for bid. I can help you place an item up for auction too."
                    .into(),
                options: vec![
                    option(
                        0,
                        0,
                        "Have you seen anyone suspicious trying to sell a crate of objects recently?"
                    ),
                    option(1, 6, "Let me see the items up for auction."),
                ],
            })
        )]
    );
    assert_eq!(h.session().unwrap().auction_house, None);

    h.select(drezmit, 1);
    assert_eq!(
        h.take::<InteractionOpened>(),
        vec![opened(
            drezmit,
            InteractionKind::Role(NpcRole::AuctionHouse)
        )]
    );
    assert_eq!(
        h.session().unwrap().auction_house,
        Some(AUCTION_HOUSE_HORDE)
    );
}

#[test]
fn innkeeper_gossip_menu_lists_vendor_option_that_opens_vendor() {
    let Some(mut h) = harness(HUMAN_TEMPLATE) else {
        return;
    };
    let farley = h.spawn_real_npc(INNKEEPER_FARLEY, 2.0);
    h.interact(farley);
    // TrinityCore gossip_menu_option rows of menu 1291 (Retail has no "Trick or Treat!"
    // row there; AzerothCore's was a Hallow's End option).
    assert_eq!(
        h.take::<InteractionOpened>(),
        vec![opened(
            farley,
            InteractionKind::Gossip(GossipMenu {
                menu_id: 1291,
                text: "Welcome to my Inn, weary traveler. What can I do for you?".into(),
                options: vec![
                    option(0, 0, "What can I do at an inn?"),
                    // Retail BroadcastText of the option (TrinityCore's OptionText is the older wording).
                    option(1, 5, "Bind your hearthstone to this inn."),
                    option(2, 1, "I want to browse your goods."),
                ],
            })
        )]
    );

    h.select(farley, 0);
    let submenu = h.take::<InteractionOpened>();
    assert!(
        matches!(&submenu[..], [InteractionOpened { kind: InteractionKind::Gossip(menu), .. }] if menu.menu_id == 1221),
        "{submenu:?}"
    );

    h.interact(farley);
    h.take::<InteractionOpened>();
    h.select(farley, 2);
    assert_eq!(
        h.take::<InteractionOpened>(),
        vec![opened(farley, InteractionKind::Role(NpcRole::Vendor))]
    );
    assert_eq!(h.session().unwrap().role, Some(NpcRole::Vendor));
}

#[test]
fn hostile_npc_cannot_be_interacted_with() {
    let Some(mut h) = harness(HUMAN_TEMPLATE) else {
        return;
    };
    let gryshka = h.spawn_real_npc(INNKEEPER_GRYSHKA, 2.0);
    h.interact(gryshka);
    assert!(h.take::<InteractionOpened>().is_empty());
    assert_eq!(
        h.take::<InteractionFailed>(),
        vec![InteractionFailed {
            npc: gryshka.to_bits(),
            error: InteractionError::TargetHostile,
        }]
    );
    assert!(h.session().is_none());
}

#[test]
fn npc_in_combat_cannot_be_interacted_with() {
    let Some(mut h) = harness(HUMAN_TEMPLATE) else {
        return;
    };
    let farley = h.spawn_real_npc(INNKEEPER_FARLEY, 2.0);
    h.app
        .world_mut()
        .entity_mut(farley)
        .insert(CombatStatus(true));
    h.interact(farley);
    assert_eq!(
        h.take::<InteractionFailed>(),
        vec![InteractionFailed {
            npc: farley.to_bits(),
            error: InteractionError::TargetInCombat,
        }]
    );
}

#[test]
fn walking_away_from_auctioneer_closes_auction_house_session() {
    let Some(mut h) = harness(HUMAN_TEMPLATE) else {
        return;
    };
    let fitch = h.spawn_real_npc(AUCTIONEER_FITCH, 3.0);
    h.interact(fitch);
    h.take::<InteractionOpened>();
    h.browse();
    assert_eq!(h.take::<AuctionSearchResults>().len(), 1);

    h.place_player(fitch, 12.0);
    h.advance();
    assert_eq!(
        h.take::<InteractionClosed>(),
        vec![InteractionClosed {
            npc: fitch.to_bits()
        }]
    );
    assert!(h.session().is_none());
    h.browse();
    assert!(h.take::<AuctionSearchResults>().is_empty());
    assert_eq!(
        h.take::<AuctionOperationResponse>(),
        vec![AuctionOperationResponse {
            success: false,
            message: NO_AUCTIONEER_MESSAGE.into(),
        }]
    );
}

#[test]
fn auction_requests_without_auctioneer_session_are_rejected() {
    let Some(mut h) = harness(HUMAN_TEMPLATE) else {
        return;
    };
    h.browse();
    assert!(h.take::<AuctionSearchResults>().is_empty());
    assert_eq!(
        h.take::<AuctionOperationResponse>(),
        vec![AuctionOperationResponse {
            success: false,
            message: NO_AUCTIONEER_MESSAGE.into(),
        }]
    );
    h.send::<_, AuctionChannel>(OpenAuctionHouse);
    assert_eq!(
        h.take::<AuctionHouseOpened>(),
        vec![AuctionHouseOpened {
            success: false,
            error: Some(NO_AUCTIONEER_MESSAGE.into()),
        }]
    );
}

const NO_AUCTIONEER_MESSAGE: &str = "not interacting with an auctioneer";

#[test]
fn auction_house_follows_auctioneer_faction_group() {
    let Some(templates) = world_db_faction_templates() else {
        return;
    };
    // FactionTemplate 12 Stormwind, 29 Orgrimmar, 120 (FactionGroup 0).
    assert_eq!(auction_house_for(&templates, 12), AUCTION_HOUSE_ALLIANCE);
    assert_eq!(auction_house_for(&templates, 29), AUCTION_HOUSE_HORDE);
    assert_eq!(auction_house_for(&templates, 120), AUCTION_HOUSE_NEUTRAL);
}
