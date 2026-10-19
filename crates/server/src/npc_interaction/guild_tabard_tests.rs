use super::*;
use shared::guild_creation::{GuildTabardDesign, TABARD_COST};
use shared::protocol::{GuildTabardReply, GuildTabardRequest};

#[test]
fn guild_tabard_vendor_host_messages_revalidate_range_role_auth_and_catalog_before_payment() {
    let mut h = harness_with(HUMAN_TEMPLATE, |app| {
        let catalog =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
        let conn = world_db().unwrap();
        app.insert_resource(crate::area_table::load_area_table(&conn).unwrap());
        app.insert_resource(crate::test_data::ground_map());
        app.add_plugins(crate::guild_tabards::GuildTabardPlugin::load(&catalog).unwrap());
        add_inbox::<GuildTabardReply>(app);
    })
    .unwrap();
    let owner = {
        let db = &h
            .app
            .world()
            .resource::<crate::persistence::GameDatabase>()
            .db;
        let (owner, _) = crate::persistence::create_character(db, 1, "Tabardmaster", 1, 1).unwrap();
        crate::persistence::set_character_guild_by_id(db, owner, 7, "Raid Team").unwrap();
        crate::persistence::banks::save_guild_member_rank(db, owner, 0);
        let mut record =
            crate::persistence::inventory_store::load_inventory_record(db, owner).unwrap();
        record.inventory.gold = u64::from(TABARD_COST) * 2;
        crate::persistence::inventory_store::save_inventory_record(db, owner, &record);
        owner
    };
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(shared::components::GuildMembership {
            guild_id: 7,
            guild_name: "Raid Team".into(),
        });
    h.app.world_mut().entity_mut(h.connection).insert((
        crate::auth::Authenticated { account_id: 1 },
        InWorld {
            character_id: owner,
        },
        pair::<GuildTabardRequest>(),
        pair::<GuildTabardReply>(),
        pair::<shared::protocol::GuildStateUpdate>(),
    ));
    // Source Jondor5130 uses genuine gossip, then opens the tabard-vendor service.
    let npc = h.spawn_real_npc(5130, 3.0);
    h.send::<InteractNpc, shared::protocol::InteractionChannel>(InteractNpc { npc: npc.to_bits() });
    let option_id = h
        .app
        .world()
        .get::<ActiveInteraction>(h.connection)
        .unwrap()
        .gossip_actions
        .iter()
        .find_map(|(id, action)| {
            (*action == GossipAction::Role(NpcRole::TabardDesigner)).then_some(*id)
        })
        .unwrap();
    h.send::<SelectGossipOption, shared::protocol::InteractionChannel>(SelectGossipOption {
        npc: npc.to_bits(),
        option_id,
    });
    h.send::<GuildTabardRequest, shared::protocol::GuildChannel>(GuildTabardRequest::Open {
        npc: npc.to_bits(),
    });
    assert!(
        matches!(h.app.world().resource::<Inbox<GuildTabardReply>>().0.last(), Some(GuildTabardReply::Vendor(snapshot)) if snapshot.own_rank == Some(0) && snapshot.design.is_none() && snapshot.cost == u64::from(TABARD_COST))
    );
    let design = GuildTabardDesign {
        icon: 1,
        icon_color: 2,
        border: 3,
        border_color: 4,
        background_color: 5,
    };
    h.send::<GuildTabardRequest, shared::protocol::GuildChannel>(GuildTabardRequest::Save {
        npc: npc.to_bits(),
        design: GuildTabardDesign {
            icon_color: 255,
            ..design
        },
    });
    assert_rejected(&h, owner, u64::from(TABARD_COST) * 2);
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<crate::auth::Authenticated>();
    let reply_count = h.app.world().resource::<Inbox<GuildTabardReply>>().0.len();
    h.send::<GuildTabardRequest, shared::protocol::GuildChannel>(GuildTabardRequest::Save {
        npc: npc.to_bits(),
        design,
    });
    assert_eq!(
        h.app.world().resource::<Inbox<GuildTabardReply>>().0.len(),
        reply_count
    );
    assert_rejected(&h, owner, u64::from(TABARD_COST) * 2);
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(crate::auth::Authenticated { account_id: 1 });
    h.send::<GuildTabardRequest, shared::protocol::GuildChannel>(GuildTabardRequest::Save {
        npc: h.player.to_bits(),
        design,
    });
    assert_rejected(&h, owner, u64::from(TABARD_COST) * 2);
    h.place_player(npc, 20.0);
    h.send::<GuildTabardRequest, shared::protocol::GuildChannel>(GuildTabardRequest::Save {
        npc: npc.to_bits(),
        design,
    });
    assert_rejected(&h, owner, u64::from(TABARD_COST) * 2);
    h.place_player(npc, 3.0);
    h.send::<InteractNpc, shared::protocol::InteractionChannel>(InteractNpc { npc: npc.to_bits() });
    h.send::<SelectGossipOption, shared::protocol::InteractionChannel>(SelectGossipOption {
        npc: npc.to_bits(),
        option_id,
    });
    h.send::<GuildTabardRequest, shared::protocol::GuildChannel>(GuildTabardRequest::Save {
        npc: npc.to_bits(),
        design,
    });
    let db = &h
        .app
        .world()
        .resource::<crate::persistence::GameDatabase>()
        .db;
    assert_eq!(
        crate::persistence::guild_tabards::load_design(db, 7),
        Some(design)
    );
    assert_eq!(
        crate::persistence::load_inventory(db, owner).gold,
        u64::from(TABARD_COST)
    );
    assert_eq!(
        h.app.world().get::<GuildTabardDesign>(h.player),
        Some(&design)
    );
    assert!(
        matches!(h.app.world().resource::<Inbox<GuildTabardReply>>().0.last(), Some(GuildTabardReply::Saved { design: saved }) if *saved == design)
    );
}

fn assert_rejected(h: &Harness, owner: u64, gold: u64) {
    let db = &h
        .app
        .world()
        .resource::<crate::persistence::GameDatabase>()
        .db;
    assert_eq!(crate::persistence::load_inventory(db, owner).gold, gold);
    assert_eq!(crate::persistence::guild_tabards::load_design(db, 7), None);
    assert!(matches!(
        h.app.world().resource::<Inbox<GuildTabardReply>>().0.last(),
        Some(GuildTabardReply::Failed { .. })
    ));
}
