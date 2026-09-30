use super::*;
use lightyear::connection::host::HostClient;

#[derive(Resource)]
struct Inbox<M: lightyear::prelude::Message>(Vec<M>);

fn collect<M: lightyear::prelude::Message>(
    mut receivers: Query<&mut MessageReceiver<M>>,
    mut inbox: ResMut<Inbox<M>>,
) {
    for mut receiver in &mut receivers {
        inbox.0.extend(receiver.receive());
    }
}

struct Harness {
    app: App,
    connection: Entity,
}

impl Harness {
    fn new(operation: EconomyOperation) -> Self {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(ClientPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(shared::ProtocolPlugin);
        app.init_state::<CliState>();
        app.insert_resource(State::new(CliState::InWorld));
        app.insert_resource(CliAction {
            command: CliCommand::Economy(EconomyAction {
                npc: 77,
                gossip_option: Some(3),
                operation,
            }),
            server: "127.0.0.1:5098".parse().unwrap(),
            username: None,
            password: None,
            token: Some("fixture".into()),
        });
        register(&mut app);
        app.finish();
        app.cleanup();
        let connection = app
            .world_mut()
            .spawn((
                Connected,
                RemoteId(PeerId::Local(1)),
                HostClient { buffer: Vec::new() },
            ))
            .id();
        let mut harness = Self { app, connection };
        harness.request::<InteractNpc>();
        harness.request::<SelectGossipOption>();
        harness.request::<OpenAuctionHouse>();
        harness.request::<QueryAuctions>();
        harness.request::<QueryAuctionInventory>();
        harness.request::<QueryOwnedAuctions>();
        harness.request::<QueryBidAuctions>();
        harness.request::<CreateAuction>();
        harness.request::<PlaceBid>();
        harness.request::<BuyoutAuction>();
        harness.request::<CancelAuction>();
        harness.request::<BuyItem>();
        harness.pair::<InteractionOpened>();
        harness.pair::<InteractionFailed>();
        harness.pair::<InteractionClosed>();
        harness.pair::<AuctionHouseOpened>();
        harness.pair::<AuctionSearchResults>();
        harness.pair::<AuctionInventorySnapshot>();
        harness.pair::<OwnedAuctionListResponse>();
        harness.pair::<BidAuctionListResponse>();
        harness.pair::<AuctionOperationResponse>();
        harness.pair::<VendorInventory>();
        harness.pair::<MerchantFailed>();
        harness.pair::<InventorySnapshot>();
        harness.pair::<InventoryDelta>();
        harness
    }
    fn pair<M: lightyear::prelude::Message>(&mut self) {
        self.app.world_mut().entity_mut(self.connection).insert((
            MessageSender::<M>::default(),
            MessageReceiver::<M>::default(),
        ));
    }
    fn request<M: lightyear::prelude::Message>(&mut self) {
        self.pair::<M>();
        self.app.insert_resource(Inbox::<M>(Vec::new()));
        self.app.add_systems(First, collect::<M>);
    }
    fn tick(&mut self) {
        for _ in 0..3 {
            self.app.update();
        }
    }
    fn reply<M: lightyear::prelude::Message, C: Channel>(&mut self, response: M) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<M>>(self.connection)
            .unwrap()
            .send::<C>(response);
        self.tick();
    }
    fn requests<M: lightyear::prelude::Message>(&self) -> &[M] {
        &self.app.world().resource::<Inbox<M>>().0
    }
    fn exit(&self) -> Option<AppExit> {
        self.app.should_exit()
    }
    fn open_auction(&mut self) {
        self.tick();
        assert_eq!(self.requests::<InteractNpc>(), &[InteractNpc { npc: 77 }]);
        assert!(self.requests::<OpenAuctionHouse>().is_empty());
        self.reply::<_, InteractionChannel>(InteractionOpened {
            npc: 77,
            kind: InteractionKind::Role(NpcRole::AuctionHouse),
        });
        assert_eq!(self.requests::<OpenAuctionHouse>(), &[OpenAuctionHouse]);
        self.reply::<_, AuctionChannel>(AuctionHouseOpened {
            success: true,
            error: None,
        });
    }
}

#[test]
fn wire_search_waits_for_gossip_role_and_open_then_preserves_query() {
    let query = AuctionSearchQuery {
        text: "Linen".into(),
        page: 1,
        page_size: 2,
        sort_field: AuctionSortField::Buyout,
        sort_dir: AuctionSortDir::Desc,
        item_id: Some(2589),
        class_id: Some(7),
        ..Default::default()
    };
    let mut h = Harness::new(EconomyOperation::Search(query.clone()));
    h.tick();
    assert_eq!(h.requests::<InteractNpc>(), &[InteractNpc { npc: 77 }]);
    assert!(h.requests::<QueryAuctions>().is_empty());
    h.reply::<_, InteractionChannel>(InteractionOpened {
        npc: 77,
        kind: InteractionKind::Gossip(GossipMenu {
            menu_id: 0,
            text: "Hello".into(),
            options: vec![GossipMenuOption {
                option_id: 3,
                icon: 0,
                text: "Auction!".into(),
            }],
        }),
    });
    assert_eq!(
        h.requests::<SelectGossipOption>(),
        &[SelectGossipOption {
            npc: 77,
            option_id: 3
        }]
    );
    assert!(h.requests::<OpenAuctionHouse>().is_empty());
    h.reply::<_, InteractionChannel>(InteractionOpened {
        npc: 77,
        kind: InteractionKind::Role(NpcRole::AuctionHouse),
    });
    assert_eq!(h.requests::<OpenAuctionHouse>(), &[OpenAuctionHouse]);
    assert!(h.requests::<QueryAuctions>().is_empty());
    h.reply::<_, AuctionChannel>(AuctionHouseOpened {
        success: true,
        error: None,
    });
    assert_eq!(
        h.requests::<QueryAuctions>(),
        &[QueryAuctions {
            query: query.clone()
        }]
    );
    assert!(h.exit().is_none());
    h.reply::<_, AuctionChannel>(AuctionSearchResults {
        query,
        total_results: 4,
        results: Vec::new(),
    });
    assert_eq!(h.exit(), Some(AppExit::Success));
    h.tick();
    assert_eq!(h.requests::<QueryAuctions>().len(), 1);
}

#[test]
fn wire_auction_reads_wait_for_their_own_response() {
    let mut h = Harness::new(EconomyOperation::Inventory);
    h.open_auction();
    assert_eq!(
        h.requests::<QueryAuctionInventory>(),
        &[QueryAuctionInventory]
    );
    h.reply::<_, AuctionChannel>(OwnedAuctionListResponse {
        listings: Vec::new(),
    });
    assert!(h.exit().is_none());
    h.reply::<_, AuctionChannel>(AuctionInventorySnapshot {
        gold: 123,
        items: Vec::new(),
    });
    assert_eq!(h.exit(), Some(AppExit::Success));

    let mut h = Harness::new(EconomyOperation::Owned);
    h.open_auction();
    assert_eq!(h.requests::<QueryOwnedAuctions>(), &[QueryOwnedAuctions]);
    h.reply::<_, AuctionChannel>(OwnedAuctionListResponse {
        listings: Vec::new(),
    });
    assert_eq!(h.exit(), Some(AppExit::Success));

    let mut h = Harness::new(EconomyOperation::Bids);
    h.open_auction();
    assert_eq!(h.requests::<QueryBidAuctions>(), &[QueryBidAuctions]);
    h.reply::<_, AuctionChannel>(BidAuctionListResponse {
        listings: Vec::new(),
    });
    assert_eq!(h.exit(), Some(AppExit::Success));
}

#[test]
fn wire_mutations_send_exact_payload_once_and_require_acknowledgement() {
    let request = CreateAuction {
        item_guid: 42,
        stack_count: 2,
        min_bid: 4_500_000_000,
        buyout_price: Some(5_000_000_000),
        duration: AuctionDuration::Long,
    };
    let mut h = Harness::new(EconomyOperation::Create(request.clone()));
    h.open_auction();
    assert_eq!(h.requests::<CreateAuction>(), &[request]);
    assert!(h.exit().is_none());
    h.reply::<_, AuctionChannel>(AuctionOperationResponse {
        success: true,
        message: "created".into(),
    });
    assert_eq!(h.exit(), Some(AppExit::Success));

    let request = PlaceBid {
        auction_id: 9,
        amount: 4_500_000_000,
    };
    let mut h = Harness::new(EconomyOperation::Bid(request.clone()));
    h.open_auction();
    assert_eq!(h.requests::<PlaceBid>(), &[request]);
    h.reply::<_, AuctionChannel>(AuctionOperationResponse {
        success: false,
        message: "insufficient money".into(),
    });
    assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));

    let mut h = Harness::new(EconomyOperation::Buyout(BuyoutAuction { auction_id: 9 }));
    h.open_auction();
    assert_eq!(
        h.requests::<BuyoutAuction>(),
        &[BuyoutAuction { auction_id: 9 }]
    );
    h.reply::<_, AuctionChannel>(AuctionOperationResponse {
        success: true,
        message: "bought".into(),
    });
    assert_eq!(h.exit(), Some(AppExit::Success));

    let mut h = Harness::new(EconomyOperation::Cancel(CancelAuction { auction_id: 9 }));
    h.open_auction();
    assert_eq!(
        h.requests::<CancelAuction>(),
        &[CancelAuction { auction_id: 9 }]
    );
    h.reply::<_, AuctionChannel>(AuctionOperationResponse {
        success: true,
        message: "cancelled".into(),
    });
    assert_eq!(h.exit(), Some(AppExit::Success));
}

#[test]
fn wire_rejections_and_timeouts_exit_nonzero_without_requests() {
    let mut h = Harness::new(EconomyOperation::Inventory);
    h.tick();
    h.reply::<_, InteractionChannel>(InteractionFailed {
        npc: 77,
        error: InteractionError::TooFarAway,
    });
    assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
    assert!(h.requests::<QueryAuctionInventory>().is_empty());

    let mut h = Harness::new(EconomyOperation::Inventory);
    h.tick();
    h.reply::<_, InteractionChannel>(InteractionOpened {
        npc: 77,
        kind: InteractionKind::Role(NpcRole::AuctionHouse),
    });
    h.reply::<_, AuctionChannel>(AuctionHouseOpened {
        success: false,
        error: Some("not interacting".into()),
    });
    assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
    assert!(h.requests::<QueryAuctionInventory>().is_empty());

    let mut h = Harness::new(EconomyOperation::Search(Default::default()));
    h.open_auction();
    h.app.world_mut().resource_mut::<EconomyFlow>().started =
        std::time::Instant::now() - Duration::from_secs(11);
    h.tick();
    assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
    assert_eq!(h.requests::<QueryAuctions>().len(), 1);
}

fn vendor() -> VendorInventory {
    VendorInventory {
        npc: 77,
        can_repair: false,
        items: vec![VendorItem {
            slot: 0,
            item_id: 2589,
            name: "Linen".into(),
            quality: 1,
            price: 10,
            stack_count: 2,
            max_stack: 20,
            num_available: Some(10),
            usable: true,
        }],
    }
}
fn item(count: u32) -> ItemStack {
    ItemStack {
        item_guid: 42,
        item_id: 2589,
        count,
        durability: None,
        soulbound: false,
    }
}

#[test]
fn wire_merchant_buy_retains_early_lists_and_requires_real_item_gain() {
    let mut h = Harness::new(EconomyOperation::MerchantBuy {
        slot: 0,
        item_id: 2589,
        count: 2,
    });
    h.tick();
    h.reply::<_, InventoryChannel>(InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items: vec![BagSlotItem {
                slot: 1,
                item: item(3),
            }],
        }],
    });
    h.reply::<_, MerchantChannel>(vendor());
    assert!(h.requests::<BuyItem>().is_empty());
    h.reply::<_, InteractionChannel>(InteractionOpened {
        npc: 77,
        kind: InteractionKind::Role(NpcRole::Vendor),
    });
    assert_eq!(
        h.requests::<BuyItem>(),
        &[BuyItem {
            npc: 77,
            slot: 0,
            item_id: 2589,
            count: 2,
            destination: None
        }]
    );
    h.reply::<_, MerchantChannel>(vendor());
    assert!(h.exit().is_none());
    h.reply::<_, InventoryChannel>(InventoryDelta {
        changes: vec![InventorySlotChange {
            location: ItemLocation::Bag { bag: 0, slot: 1 },
            item: Some(item(7)),
        }],
    });
    assert_eq!(h.exit(), Some(AppExit::Success));
    h.tick();
    assert_eq!(h.requests::<BuyItem>().len(), 1);
}

#[test]
fn wire_merchant_list_and_rejection_wait_for_matching_npc() {
    let mut h = Harness::new(EconomyOperation::MerchantList);
    h.tick();
    let mut other = vendor();
    other.npc = 78;
    h.reply::<_, MerchantChannel>(other);
    h.reply::<_, InteractionChannel>(InteractionOpened {
        npc: 77,
        kind: InteractionKind::Role(NpcRole::Vendor),
    });
    assert!(h.exit().is_none());
    h.reply::<_, MerchantChannel>(vendor());
    assert_eq!(h.exit(), Some(AppExit::Success));

    let mut h = Harness::new(EconomyOperation::MerchantBuy {
        slot: 0,
        item_id: 2589,
        count: 2,
    });
    h.tick();
    h.reply::<_, MerchantChannel>(MerchantFailed {
        npc: 77,
        error: MerchantError::NotEnoughMoney,
    });
    assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
}
