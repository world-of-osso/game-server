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
        harness.request::<UseGameObject>();
        harness.request::<MailRequest>();
        harness.pair::<MailboxContents>();
        harness.pair::<MailFailed>();
        harness.app.init_resource::<SelectedCharacterId>();
        harness.request::<InteractNpc>();
        harness.request::<SelectGossipOption>();
        harness.request::<OpenAuctionHouse>();
        harness.request::<QueryAuctions>();
        harness.request::<QueryAuctionBrowse>();
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
        harness.pair::<AuctionBrowseResults>();
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

fn mailbox(money: u64, attached: bool) -> MailboxContents {
    MailboxContents {
        object: 77,
        now: 100,
        mails: vec![MailHeader {
            mail_id: 9,
            sender: "Auction House".into(),
            subject: "Sold".into(),
            body: "Proceeds".into(),
            money,
            cod: 0,
            attachments: if attached {
                vec![MailAttachment {
                    slot: 5,
                    item: item(2),
                    name: "Linen".into(),
                    quality: 1,
                }]
            } else {
                vec![]
            },
            expires_at: 10000,
            read: false,
            returned: false,
            from_player: false,
            returnable: false,
        }],
    }
}

impl Harness {
    fn mailbox_role(&mut self) {
        self.reply::<_, InteractionChannel>(InteractionOpened {
            npc: 77,
            kind: InteractionKind::Role(NpcRole::Mailbox),
        });
    }
    fn money(&mut self, copper: u64) -> Entity {
        let remote = Entity::from_bits(999);
        let local = self
            .app
            .world_mut()
            .spawn(shared::components::Gold(copper))
            .id();
        self.app.world_mut().resource_mut::<SelectedCharacterId>().0 = Some(remote.to_bits());
        self.app
            .world_mut()
            .get_mut::<MessageManager>(self.connection)
            .unwrap()
            .entity_mapper
            .insert(remote, local);
        self.tick();
        local
    }
}

#[test]
fn wire_mail_list_uses_object_and_waits_for_matching_role_and_inbox() {
    let mut h = Harness::new(EconomyOperation::MailList);
    h.tick();
    assert_eq!(
        h.requests::<UseGameObject>(),
        &[UseGameObject { object: 77 }]
    );
    assert!(h.requests::<InteractNpc>().is_empty());
    let mut wrong = mailbox(5000000000, true);
    wrong.object = 78;
    h.reply::<_, MailChannel>(wrong);
    h.reply::<_, MailChannel>(mailbox(5000000000, true));
    assert!(h.exit().is_none());
    h.reply::<_, InteractionChannel>(InteractionOpened {
        npc: 78,
        kind: InteractionKind::Role(NpcRole::Mailbox),
    });
    assert!(h.exit().is_none());
    h.mailbox_role();
    assert_eq!(h.exit(), Some(AppExit::Success));
    assert!(h.requests::<OpenAuctionHouse>().is_empty());
    assert!(h.requests::<MailRequest>().is_empty());
}

#[test]
fn wire_mail_money_requires_inbox_removal_and_own_gold_update() {
    let mut h = Harness::new(EconomyOperation::MailTakeMoney { mail_id: 9 });
    let own = h.money(100);
    h.mailbox_role();
    h.reply::<_, MailChannel>(mailbox(5000000000, false));
    assert_eq!(
        h.requests::<MailRequest>(),
        &[MailRequest {
            object: 77,
            mail_id: 9,
            action: MailAction::TakeMoney
        }]
    );
    assert!(h.exit().is_none());
    h.app
        .world_mut()
        .spawn(shared::components::Gold(5000000100));
    h.reply::<_, MailChannel>(mailbox(0, false));
    assert!(h.exit().is_none());
    h.app
        .world_mut()
        .entity_mut(own)
        .insert(shared::components::Gold(5000000100));
    h.tick();
    assert_eq!(h.exit(), Some(AppExit::Success));
    assert_eq!(h.requests::<MailRequest>().len(), 1);
}

#[test]
fn wire_mail_attachment_requires_bag_increase_and_fixed_slot_removal() {
    let mut h = Harness::new(EconomyOperation::MailTakeAttachment {
        mail_id: 9,
        slot: 5,
    });
    h.tick();
    h.mailbox_role();
    h.reply::<_, MailChannel>(mailbox(0, true));
    assert!(h.requests::<MailRequest>().is_empty());
    h.reply::<_, InventoryChannel>(InventorySnapshot { bags: vec![] });
    assert_eq!(
        h.requests::<MailRequest>(),
        &[MailRequest {
            object: 77,
            mail_id: 9,
            action: MailAction::TakeAttachment { slot: 5 }
        }]
    );
    h.reply::<_, MailChannel>(mailbox(0, false));
    assert!(h.exit().is_none());
    h.reply::<_, InventoryChannel>(InventoryDelta {
        changes: vec![InventorySlotChange {
            location: ItemLocation::Bag { bag: 0, slot: 2 },
            item: Some(item(2)),
        }],
    });
    assert_eq!(h.exit(), Some(AppExit::Success));
    assert_eq!(h.requests::<MailRequest>().len(), 1);
}

#[test]
fn wire_mail_gold_first_still_waits_for_inbox_and_ignores_wrong_object() {
    let mut h = Harness::new(EconomyOperation::MailTakeMoney { mail_id: 9 });
    let own = h.money(100);
    h.mailbox_role();
    h.reply::<_, MailChannel>(mailbox(200, false));
    h.app
        .world_mut()
        .entity_mut(own)
        .insert(shared::components::Gold(300));
    h.tick();
    assert!(h.exit().is_none());
    let mut wrong = mailbox(0, false);
    wrong.object = 78;
    h.reply::<_, MailChannel>(wrong);
    assert!(h.exit().is_none());
    h.reply::<_, MailChannel>(mailbox(0, false));
    assert_eq!(h.exit(), Some(AppExit::Success));
}

#[test]
fn wire_mail_money_waits_for_initial_own_gold() {
    let mut h = Harness::new(EconomyOperation::MailTakeMoney { mail_id: 9 });
    h.tick();
    h.mailbox_role();
    h.reply::<_, MailChannel>(mailbox(200, false));
    assert!(h.requests::<MailRequest>().is_empty());
    h.app.world_mut().spawn(shared::components::Gold(100));
    h.tick();
    assert!(h.requests::<MailRequest>().is_empty());
    h.money(100);
    assert_eq!(
        h.requests::<MailRequest>(),
        &[MailRequest {
            object: 77,
            mail_id: 9,
            action: MailAction::TakeMoney
        }]
    );
    assert!(h.exit().is_none());
}

#[test]
fn wire_mail_bag_first_does_not_complete_until_fixed_slot_removed() {
    let mut h = Harness::new(EconomyOperation::MailTakeAttachment {
        mail_id: 9,
        slot: 5,
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
    h.reply::<_, MailChannel>(mailbox(0, true));
    assert!(h.requests::<MailRequest>().is_empty());
    h.mailbox_role();
    assert_eq!(h.requests::<MailRequest>().len(), 1);
    h.reply::<_, InventoryChannel>(InventoryDelta {
        changes: vec![InventorySlotChange {
            location: ItemLocation::Bag { bag: 0, slot: 1 },
            item: Some(item(5)),
        }],
    });
    assert!(h.exit().is_none());
    h.reply::<_, MailChannel>(mailbox(0, true));
    assert!(h.exit().is_none());
    h.reply::<_, MailChannel>(mailbox(0, false));
    assert_eq!(h.exit(), Some(AppExit::Success));
}

#[test]
fn wire_mail_initial_rejection_closure_and_timeout_block_claims() {
    for mode in ["failed", "closed", "timeout"] {
        let mut h = Harness::new(EconomyOperation::MailTakeMoney { mail_id: 9 });
        h.tick();
        h.reply::<_, MailChannel>(mailbox(200, false));
        match mode {
            "failed" => h.reply::<_, InteractionChannel>(InteractionFailed {
                npc: 77,
                error: InteractionError::TooFarAway,
            }),
            "closed" => h.reply::<_, InteractionChannel>(InteractionClosed { npc: 77 }),
            _ => {
                h.app.world_mut().resource_mut::<EconomyFlow>().started =
                    std::time::Instant::now() - Duration::from_secs(11);
                h.tick();
            }
        }
        assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
        h.tick();
        assert!(h.requests::<MailRequest>().is_empty());
        assert_eq!(h.requests::<UseGameObject>().len(), 1);
    }
}

#[test]
fn wire_mail_rejections_close_and_deadlines_never_retry() {
    for mode in ["failed", "closed", "timeout"] {
        let mut h = Harness::new(EconomyOperation::MailTakeMoney { mail_id: 9 });
        h.money(100);
        h.mailbox_role();
        h.app.world_mut().resource_mut::<EconomyFlow>().started =
            std::time::Instant::now() - Duration::from_secs(9);
        h.reply::<_, MailChannel>(mailbox(200, false));
        assert!(h.app.world().resource::<EconomyFlow>().started.elapsed() < Duration::from_secs(1));
        h.reply::<_, MailChannel>(MailFailed {
            object: 78,
            error: MailError::NotInteracting,
        });
        assert!(h.exit().is_none());
        match mode {
            "failed" => h.reply::<_, MailChannel>(MailFailed {
                object: 77,
                error: MailError::TooMuchGold,
            }),
            "closed" => h.reply::<_, InteractionChannel>(InteractionClosed { npc: 77 }),
            _ => {
                h.app.world_mut().resource_mut::<EconomyFlow>().started =
                    std::time::Instant::now() - Duration::from_secs(11);
                h.tick();
            }
        }
        assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
        h.tick();
        assert_eq!(h.requests::<MailRequest>().len(), 1);
    }
}

#[test]
fn wire_mail_wrong_role_missing_mail_and_invalid_slot_fail_without_mutations() {
    let mut h = Harness::new(EconomyOperation::MailList);
    h.tick();
    h.reply::<_, InteractionChannel>(InteractionOpened {
        npc: 77,
        kind: InteractionKind::Role(NpcRole::Vendor),
    });
    assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
    for operation in [
        EconomyOperation::MailTakeMoney { mail_id: 10 },
        EconomyOperation::MailTakeAttachment {
            mail_id: 9,
            slot: 0,
        },
    ] {
        let mut h = Harness::new(operation);
        h.tick();
        h.mailbox_role();
        h.reply::<_, InventoryChannel>(InventorySnapshot { bags: vec![] });
        h.reply::<_, MailChannel>(mailbox(200, true));
        assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
        assert!(h.requests::<MailRequest>().is_empty());
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

fn browse_response(query: AuctionSearchQuery) -> AuctionBrowseResults {
    AuctionBrowseResults {
        query,
        total_results: 4,
        items: vec![AuctionBrowseItem {
            item_id: 2589,
            definition_source: shared::item_data::ItemDefinitionSource::Retail,
            name: "Linen".into(),
            quality: 1,
            required_level: 10,
            lowest_unit_price: 5_000_000_000,
            total_quantity: 6_000_000_000,
        }],
    }
}

#[test]
fn wire_browse_waits_for_role_open_and_grouped_response_only() {
    let query = AuctionSearchQuery {
        text: "Linen".into(),
        page: 1,
        page_size: 2,
        sort_field: AuctionSortField::Buyout,
        sort_dir: AuctionSortDir::Desc,
        item_id: Some(2589),
        class_id: Some(7),
        quality: Some(1),
        min_level: Some(10),
        max_level: Some(80),
        usable_only: true,
        ..Default::default()
    };
    let mut h = Harness::new(EconomyOperation::Browse(query.clone()));
    h.tick();
    assert_eq!(h.requests::<InteractNpc>(), &[InteractNpc { npc: 77 }]);
    assert!(h.requests::<QueryAuctionBrowse>().is_empty());
    h.reply::<_, AuctionChannel>(browse_response(query.clone()));
    assert!(h.exit().is_none());
    h.reply::<_, InteractionChannel>(InteractionOpened {
        npc: 77,
        kind: InteractionKind::Role(NpcRole::AuctionHouse),
    });
    assert_eq!(h.requests::<OpenAuctionHouse>(), &[OpenAuctionHouse]);
    assert!(h.requests::<QueryAuctionBrowse>().is_empty());
    h.reply::<_, AuctionChannel>(AuctionHouseOpened {
        success: true,
        error: None,
    });
    assert_eq!(
        h.requests::<QueryAuctionBrowse>(),
        &[QueryAuctionBrowse {
            query: query.clone()
        }]
    );
    assert!(h.requests::<QueryAuctions>().is_empty());
    h.reply::<_, AuctionChannel>(AuctionSearchResults {
        query: query.clone(),
        total_results: 8,
        results: Vec::new(),
    });
    assert!(h.exit().is_none());
    h.reply::<_, AuctionChannel>(browse_response(query));
    assert_eq!(h.exit(), Some(AppExit::Success));
    h.tick();
    assert_eq!(h.requests::<QueryAuctionBrowse>().len(), 1);
}

#[test]
fn wire_browse_rejection_and_timeout_exit_nonzero() {
    let mut h = Harness::new(EconomyOperation::Browse(Default::default()));
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
    assert!(h.requests::<QueryAuctionBrowse>().is_empty());

    let mut h = Harness::new(EconomyOperation::Browse(Default::default()));
    h.open_auction();
    h.reply::<_, AuctionChannel>(AuctionOperationResponse {
        success: false,
        message: "browse rejected".into(),
    });
    assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));

    let mut h = Harness::new(EconomyOperation::Browse(Default::default()));
    h.open_auction();
    h.app.world_mut().resource_mut::<EconomyFlow>().started =
        std::time::Instant::now() - Duration::from_secs(11);
    h.tick();
    assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
    assert_eq!(h.requests::<QueryAuctionBrowse>().len(), 1);
}

#[test]
fn browse_report_preserves_distinct_totals_and_global_u64_values() {
    let report = format_browse(
        &browse_response(AuctionSearchQuery {
            page: 1,
            page_size: 2,
            ..Default::default()
        }),
        Duration::from_micros(12500),
    );
    assert!(report.starts_with(
        "browse request_response_ms=12.500 page=1 page_size=2 total_results=4 returned=1 query="
    ));
    assert!(report.contains("item_id: 2589"));
    assert!(report.contains("lowest_unit_price: 5000000000"));
    assert!(report.contains("total_quantity: 6000000000"));
}

#[test]
fn wire_open_requires_house_confirmation() {
    let mut h = Harness::new(EconomyOperation::Open);
    h.tick();
    h.reply::<_, InteractionChannel>(InteractionOpened {
        npc: 77,
        kind: InteractionKind::Role(NpcRole::AuctionHouse),
    });
    assert_eq!(h.requests::<OpenAuctionHouse>(), &[OpenAuctionHouse]);
    assert!(h.exit().is_none());
    h.reply::<_, AuctionChannel>(AuctionHouseOpened {
        success: true,
        error: None,
    });
    assert_eq!(h.exit(), Some(AppExit::Success));
    assert!(h.requests::<QueryAuctions>().is_empty());
}

#[test]
fn wire_unoffered_gossip_fails_before_house_open() {
    let mut h = Harness::new(EconomyOperation::Inventory);
    h.tick();
    h.reply::<_, InteractionChannel>(InteractionOpened {
        npc: 77,
        kind: InteractionKind::Gossip(GossipMenu {
            menu_id: 1,
            text: "Hello".into(),
            options: vec![GossipMenuOption {
                option_id: 4,
                icon: 0,
                text: "Auction!".into(),
            }],
        }),
    });
    assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
    assert!(h.requests::<SelectGossipOption>().is_empty());
    assert!(h.requests::<OpenAuctionHouse>().is_empty());
}

#[test]
fn wire_interaction_close_aborts_pending_mutation() {
    let mut h = Harness::new(EconomyOperation::Buyout(BuyoutAuction { auction_id: 9 }));
    h.open_auction();
    h.reply::<_, InteractionChannel>(InteractionClosed { npc: 77 });
    assert_eq!(h.exit(), Some(AppExit::Error(1.try_into().unwrap())));
    assert_eq!(h.requests::<BuyoutAuction>().len(), 1);
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
        guild_repair_money: None,
        items: vec![VendorItem {
            slot: 0,
            item_id: 2589,
            name: "Linen".into(),
            quality: 1,
            price: 10,
            extra_costs: Vec::new(),
            stack_count: 2,
            max_stack: 20,
            num_available: Some(10),
            usable: true,
            max_durability: None,
        }],
    }
}
fn item(count: u32) -> ItemStack {
    ItemStack {
        item_guid: 42,
        item_id: 2589,
        definition_source: shared::item_data::ItemDefinitionSource::Retail,
        count,
        durability: None,
        soulbound: false,
        crafting_origin: None,
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
