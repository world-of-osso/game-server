use super::*;
use crate::achievement::{AchievementPlugin, PlayerAchievementState};
use crate::bank_test_harness::{Harness, LINEN};
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::protocol::{AuctionChannel, InteractNpc, InteractionChannel};

fn add_auction_transport(h: &mut Harness, connection: Entity, player: Entity) {
    // Authored Auctioneer Jaxon, relocated to the private fixture; session setup is not a gossip proof.
    let (name, faction, flags): (String, u32, u32) = h
        .conn
        .query_row(
            "SELECT name,faction,npcflag FROM content_creature_template WHERE entry=15659",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let npc = h
        .app
        .world_mut()
        .spawn((
            shared::components::Npc {
                template_id: 15659,
                name,
            },
            shared::protocol::NpcFlags::from_azerothcore(flags),
            shared::components::UnitFactionTemplate(faction),
            shared::components::Position {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
            shared::components::Health {
                current: 100.0,
                max: 100.0,
            },
            crate::maps::InMap::continent(0),
        ))
        .id();
    h.app.world_mut().entity_mut(connection).insert((
        MessageSender::<CreateAuction>::default(),
        MessageReceiver::<CreateAuction>::default(),
        MessageSender::<PlaceBid>::default(),
        MessageReceiver::<PlaceBid>::default(),
        MessageSender::<BuyoutAuction>::default(),
        MessageReceiver::<BuyoutAuction>::default(),
        MessageSender::<AuctionOperationResponse>::default(),
        MessageReceiver::<AuctionOperationResponse>::default(),
    ));
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(PlayerAchievementState::default());
    h.send_from::<_, InteractionChannel>(connection, InteractNpc { npc: npc.to_bits() });
}

pub(crate) fn prepare_auction_players(path: &std::path::Path) -> (Harness, Entity, u64) {
    let mut h = crate::bank_test_harness::harness_at(path, "Achseller").unwrap();
    let buyer = h.create_on_account(2, "Achbuyer", 1);
    let (connection, player) =
        crate::bank_test_harness::spawn_connection(&mut h.app, 2, buyer, "Achbuyer");
    h.app
        .world_mut()
        .entity_mut(connection)
        .insert(crate::auth::Authenticated { account_id: 2 });
    let seller_connection = h.connection;
    let seller_player = h.player;
    add_auction_transport(&mut h, seller_connection, seller_player);
    add_auction_transport(&mut h, connection, player);
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&h.conn).unwrap());
    bevy::app::Plugin::build(&AchievementPlugin, &mut h.app);
    bevy::app::Plugin::build(&AuctionHousePlugin, &mut h.app);
    h.app.update();
    (h, connection, buyer)
}

pub(crate) fn progress(h: &Harness, character: u64, criterion: u32) -> Option<u64> {
    persistence::load_achievements(h.db(), character)
        .criteria_progress
        .get(&criterion)
        .copied()
}

#[test]
fn achievements_criteria_auctions_native_post_bid_and_buyout_credit_only_committed_outcomes() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let (mut h, buyer_connection, buyer) = prepare_auction_players(file.path());
    let seller = h.character_id;
    let guid = h.guid_of(LINEN);
    let request = CreateAuction {
        item_guid: guid,
        stack_count: 1,
        min_bid: 100,
        buyout_price: Some(1_000),
        duration: AuctionDuration::Short,
    };
    h.send::<_, AuctionChannel>(CreateAuction {
        stack_count: 0,
        ..request.clone()
    });
    assert!(
        h.app
            .world()
            .resource::<AuctionHouseState>()
            .auctions
            .is_empty()
    );
    assert_eq!(progress(&h, seller, 4220), None);
    h.send::<_, AuctionChannel>(request);
    let auction = h
        .app
        .world()
        .resource::<AuctionHouseState>()
        .auctions
        .values()
        .next()
        .unwrap()
        .clone();
    assert_eq!(auction.item.item_id, LINEN);
    assert_eq!(persistence::load_auctions(h.db()).len(), 1);
    assert_eq!(progress(&h, seller, 4220), Some(1));
    assert_eq!(progress(&h, buyer, 4220), None);

    h.send_from::<_, AuctionChannel>(
        buyer_connection,
        PlaceBid {
            auction_id: auction.auction_id,
            amount: 100,
        },
    );
    assert_eq!(progress(&h, buyer, 4222), Some(100));
    assert_eq!(progress(&h, buyer, 4221), None);
    assert_eq!(progress(&h, seller, 4225), None);
    h.send_from::<_, AuctionChannel>(
        buyer_connection,
        PlaceBid {
            auction_id: auction.auction_id,
            amount: 99,
        },
    );
    assert_eq!(progress(&h, buyer, 4222), Some(100));

    h.send_from::<_, AuctionChannel>(
        buyer_connection,
        BuyoutAuction {
            auction_id: auction.auction_id,
        },
    );
    assert!(persistence::load_auctions(h.db()).is_empty());
    let payout = 950 + auction.deposit;
    assert_eq!(mail_store::load_mailbox(h.db(), seller)[0].money, payout);
    assert_eq!(progress(&h, seller, 4225), Some(payout));
    assert_eq!(progress(&h, seller, 4223), Some(1_000));
    assert_eq!(progress(&h, buyer, 4221), Some(1));
    assert_eq!(progress(&h, buyer, 4222), Some(1_000));
    assert_eq!(progress(&h, buyer, 4225), None);
    h.send_from::<_, AuctionChannel>(
        buyer_connection,
        BuyoutAuction {
            auction_id: auction.auction_id,
        },
    );
    for _ in 0..3 {
        h.app.update();
    }
    assert_eq!(progress(&h, seller, 4225), Some(payout));
    assert_eq!(progress(&h, buyer, 4221), Some(1));
    drop(h);
    let reopened = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&reopened, seller)
            .criteria_progress
            .get(&4225),
        Some(&payout)
    );
    assert_eq!(
        persistence::load_achievements(&reopened, buyer)
            .criteria_progress
            .get(&4221),
        Some(&1)
    );
}

fn post_linen(h: &mut Harness) -> AuctionData {
    let guid = h.guid_of(LINEN);
    h.send::<_, AuctionChannel>(CreateAuction {
        item_guid: guid,
        stack_count: 1,
        min_bid: 100,
        buyout_price: Some(1_000),
        duration: AuctionDuration::Short,
    });
    h.app
        .world()
        .resource::<AuctionHouseState>()
        .auctions
        .values()
        .next()
        .unwrap()
        .clone()
}

#[test]
fn achievements_criteria_auctions_native_bid_at_buyout_settles_once() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let (mut h, connection, buyer) = prepare_auction_players(file.path());
    let seller = h.character_id;
    let auction = post_linen(&mut h);
    h.send_from::<_, AuctionChannel>(
        connection,
        PlaceBid {
            auction_id: auction.auction_id,
            amount: 1_000,
        },
    );
    assert!(persistence::load_auctions(h.db()).is_empty());
    assert_eq!(progress(&h, seller, 4225), Some(950 + auction.deposit));
    assert_eq!(progress(&h, seller, 4223), Some(1_000));
    assert_eq!(progress(&h, buyer, 4221), Some(1));
    assert_eq!(progress(&h, buyer, 4222), Some(1_000));
    h.send_from::<_, AuctionChannel>(
        connection,
        PlaceBid {
            auction_id: auction.auction_id,
            amount: 1_000,
        },
    );
    assert_eq!(progress(&h, seller, 4225), Some(950 + auction.deposit));
    assert_eq!(progress(&h, buyer, 4221), Some(1));
}

#[test]
fn achievements_criteria_auctions_native_expiry_distinguishes_sold_unsold_and_offline() {
    for (with_bid, offline) in [(false, false), (true, false), (true, true)] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let (mut h, connection, buyer) = prepare_auction_players(file.path());
        let seller = h.character_id;
        let auction = post_linen(&mut h);
        if with_bid {
            h.send_from::<_, AuctionChannel>(
                connection,
                PlaceBid {
                    auction_id: auction.auction_id,
                    amount: 200,
                },
            );
        }
        if offline {
            h.app
                .world_mut()
                .entity_mut(connection)
                .remove::<crate::auth::InWorld>();
            h.app
                .world_mut()
                .entity_mut(h.connection)
                .remove::<crate::auth::InWorld>();
        }
        h.app.insert_resource(crate::maps::ServerClock {
            fixed_unix: Some(auction.expires_at_unix_secs as i64),
            offset_secs: 0,
        });
        // The production expiry function at its injected deadline, not a 24-hour wall-clock claim.
        h.app
            .add_systems(Update, super::super::handlers::expire_auctions);
        h.app.update();
        finish_market_work(&mut h.app);
        assert!(persistence::load_auctions(h.db()).is_empty());
        let sale = with_bid && !offline;
        let payout = sale.then_some(190 + auction.deposit);
        let win = sale.then_some(1);
        assert_eq!(progress(&h, seller, 4225), payout);
        assert_eq!(progress(&h, seller, 4223), sale.then_some(200));
        assert_eq!(progress(&h, buyer, 4221), win);
        assert_eq!(
            mail_store::load_mailbox(h.db(), seller)[0].money,
            if with_bid {
                190 + auction.deposit
            } else {
                auction.deposit
            }
        );
        if offline {
            h.app
                .world_mut()
                .entity_mut(connection)
                .insert(crate::auth::InWorld {
                    character_id: buyer,
                });
            h.app
                .world_mut()
                .entity_mut(h.connection)
                .insert(crate::auth::InWorld {
                    character_id: seller,
                });
            for _ in 0..3 {
                h.app.update();
            }
            assert_eq!(progress(&h, seller, 4225), None);
            assert_eq!(progress(&h, buyer, 4221), None);
        }
        drop(h);
        let reopened = redb::Database::open(file.path()).unwrap();
        assert_eq!(
            persistence::load_achievements(&reopened, seller)
                .criteria_progress
                .get(&4225)
                .copied(),
            payout
        );
        assert_eq!(
            persistence::load_achievements(&reopened, buyer)
                .criteria_progress
                .get(&4221)
                .copied(),
            win
        );
    }
}
