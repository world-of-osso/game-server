use super::*;
use crate::components::CharacterAppearance;
use crate::components::{EquipmentVisualSlot, EquippedAppearanceEntry};

#[test]
fn chat_type_serialization_round_trip() {
    let types = vec![
        ChatType::Say,
        ChatType::Yell,
        ChatType::Party,
        ChatType::Guild,
        ChatType::Whisper("TargetPlayer".into()),
        ChatType::Emote,
    ];
    for ct in types {
        let serialized = serde_json::to_string(&ct).unwrap();
        let deserialized: ChatType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(ct, deserialized);
    }
}

#[test]
fn chat_message_serialization_round_trip() {
    let msg = ChatMessage {
        sender: "Alice".into(),
        content: "Hello world".into(),
        channel: ChatType::Whisper("Bob".into()),
    };
    let serialized = serde_json::to_string(&msg).unwrap();
    let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();
    assert_eq!(msg.sender, deserialized.sender);
    assert_eq!(msg.content, deserialized.content);
    assert_eq!(msg.channel, deserialized.channel);
}

#[test]
fn guild_invite_messages_round_trip() {
    let invite = GuildInviteRequest {
        target_character_name: "TargetOne".into(),
    };
    let encoded = serde_json::to_string(&invite).unwrap();
    let decoded: GuildInviteRequest = serde_json::from_str(&encoded).unwrap();
    assert_eq!(invite.target_character_name, decoded.target_character_name);

    let invite_resp = GuildInviteResponse {
        success: true,
        error: None,
    };
    let encoded = serde_json::to_string(&invite_resp).unwrap();
    let decoded: GuildInviteResponse = serde_json::from_str(&encoded).unwrap();
    assert_eq!(invite_resp.success, decoded.success);
    assert_eq!(invite_resp.error, decoded.error);

    let accept_resp = GuildAcceptInviteResponse {
        success: true,
        guild_id: Some(3),
        guild_name: Some("Raid Team".into()),
        error: None,
    };
    let encoded = serde_json::to_string(&accept_resp).unwrap();
    let decoded: GuildAcceptInviteResponse = serde_json::from_str(&encoded).unwrap();
    assert_eq!(accept_resp.success, decoded.success);
    assert_eq!(accept_resp.guild_id, decoded.guild_id);
    assert_eq!(accept_resp.guild_name, decoded.guild_name);
    assert_eq!(accept_resp.error, decoded.error);

    let query = QueryGuild;
    let encoded = serde_json::to_string(&query).unwrap();
    let _: QueryGuild = serde_json::from_str(&encoded).unwrap();

    let set_motd = SetGuildMotd {
        text: "Raid tonight".into(),
    };
    let encoded = serde_json::to_string(&set_motd).unwrap();
    let decoded: SetGuildMotd = serde_json::from_str(&encoded).unwrap();
    assert_eq!(set_motd.text, decoded.text);

    let set_info = SetGuildInfo {
        text: "Wed/Sun raids".into(),
    };
    let encoded = serde_json::to_string(&set_info).unwrap();
    let decoded: SetGuildInfo = serde_json::from_str(&encoded).unwrap();
    assert_eq!(set_info.text, decoded.text);

    let set_note = SetGuildOfficerNote {
        character_name: "Alice".into(),
        note: "Reliable healer".into(),
    };
    let encoded = serde_json::to_string(&set_note).unwrap();
    let decoded: SetGuildOfficerNote = serde_json::from_str(&encoded).unwrap();
    assert_eq!(set_note.character_name, decoded.character_name);
    assert_eq!(set_note.note, decoded.note);

    let state = GuildStateUpdate {
        guild: Some(GuildSnapshot {
            guild_id: 3,
            guild_name: "Raid Team".into(),
            motd: "Bring flasks".into(),
            info_text: "Wed/Sun raids".into(),
            members: vec![GuildMemberSnapshot {
                character_name: "Alice".into(),
                level: 60,
                class_name: "Priest".into(),
                rank_name: "Member".into(),
                is_online: true,
                officer_note: "Reliable healer".into(),
                last_online: "Online".into(),
            }],
        }),
        message: Some("guild updated".into()),
        error: None,
    };
    let encoded = serde_json::to_string(&state).unwrap();
    let decoded: GuildStateUpdate = serde_json::from_str(&encoded).unwrap();
    assert_eq!(state.message, decoded.message);
    assert_eq!(
        state.guild.as_ref().unwrap().motd,
        decoded.guild.as_ref().unwrap().motd
    );
}

#[test]
fn chat_history_response_round_trip() {
    let resp = ChatHistoryResponse {
        messages: vec![
            ChatMessage {
                sender: "Alice".into(),
                content: "hello".into(),
                channel: ChatType::Say,
            },
            ChatMessage {
                sender: "Bob".into(),
                content: "guild hi".into(),
                channel: ChatType::Guild,
            },
        ],
        error: None,
    };
    let encoded = serde_json::to_string(&resp).unwrap();
    let decoded: ChatHistoryResponse = serde_json::from_str(&encoded).unwrap();
    assert_eq!(resp.messages.len(), decoded.messages.len());
    assert_eq!(resp.messages[0].sender, decoded.messages[0].sender);
    assert_eq!(resp.messages[1].channel, decoded.messages[1].channel);
    assert_eq!(resp.error, decoded.error);
}

#[test]
fn emote_messages_round_trip() {
    let intent = EmoteIntent {
        emote: EmoteKind::Sit,
    };
    let encoded = serde_json::to_string(&intent).unwrap();
    let decoded: EmoteIntent = serde_json::from_str(&encoded).unwrap();
    assert_eq!(intent, decoded);

    let event = EmoteEvent {
        player_entity: 77,
        sender: "Alice".into(),
        emote: EmoteKind::Sleep,
    };
    let encoded = serde_json::to_string(&event).unwrap();
    let decoded: EmoteEvent = serde_json::from_str(&encoded).unwrap();
    assert_eq!(event, decoded);
}

fn sample_auction_query() -> AuctionSearchQuery {
    AuctionSearchQuery {
        text: "linen".into(),
        page: 0,
        page_size: 20,
        min_level: Some(1),
        max_level: Some(10),
        quality: Some(1),
        usable_only: false,
        sort_field: AuctionSortField::Name,
        sort_dir: AuctionSortDir::Asc,
        faction: 0,
    }
}

fn sample_auction_listing() -> AuctionListingSummary {
    AuctionListingSummary {
        auction_id: 7,
        item: AuctionInventoryItem {
            item_guid: 12,
            item_id: 2589,
            name: "Linen Cloth".into(),
            quality: 1,
            required_level: 1,
            stack_count: 20,
            vendor_sell_price: 13,
        },
        owner_name: "Seller".into(),
        stack_count: 20,
        min_bid: 100,
        current_bid: Some(125),
        min_next_bid: 131,
        buyout_price: Some(200),
        time_left: AuctionTimeLeft::Long,
    }
}

fn sample_auction_search_results() -> AuctionSearchResults {
    AuctionSearchResults {
        query: sample_auction_query(),
        total_results: 1,
        results: vec![sample_auction_listing()],
    }
}

#[test]
fn auction_search_results_round_trip() {
    let msg = sample_auction_search_results();
    let encoded = serde_json::to_string(&msg).unwrap();
    let decoded: AuctionSearchResults = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.query.text, "linen");
    assert_eq!(decoded.total_results, 1);
    assert_eq!(decoded.results[0].auction_id, 7);
    assert_eq!(decoded.results[0].item.name, "Linen Cloth");
    assert_eq!(decoded.results[0].time_left, AuctionTimeLeft::Long);
}

#[test]
fn forced_disconnect_round_trip() {
    let notice = ForcedDisconnect {
        message: "You were kicked: testing".to_string(),
        reconnect_allowed: false,
    };
    let encoded = serde_json::to_string(&notice).unwrap();
    let decoded: ForcedDisconnect = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, notice);
}

#[test]
fn equipment_appearance_round_trips() {
    let snapshot = EquipmentAppearance {
        entries: vec![
            EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Head,
                item_id: Some(19019),
                display_info_id: Some(12345),
                inventory_type: 1,
                hidden: false,
            },
            EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::MainHand,
                item_id: Some(17182),
                display_info_id: Some(54321),
                inventory_type: 21,
                hidden: false,
            },
        ],
    };

    let bitcode_encoded = bitcode::encode(&snapshot);
    let bitcode_decoded: EquipmentAppearance = bitcode::decode(&bitcode_encoded).unwrap();
    assert_eq!(bitcode_decoded, snapshot);

    let json_encoded = serde_json::to_string(&snapshot).unwrap();
    let json_decoded: EquipmentAppearance = serde_json::from_str(&json_encoded).unwrap();
    assert_eq!(json_decoded, snapshot);
}

#[test]
fn ignore_list_state_update_round_trip() {
    let update = IgnoreListStateUpdate {
        snapshot: Some(IgnoreListSnapshot {
            names: vec!["Alice".into(), "Bob".into()],
        }),
        message: Some("ignore list updated".into()),
        error: None,
    };

    let encoded = serde_json::to_string(&update).unwrap();
    let decoded: IgnoreListStateUpdate = serde_json::from_str(&encoded).unwrap();
    assert_eq!(update, decoded);
}

#[test]
fn who_state_update_round_trip() {
    let update = WhoStateUpdate {
        snapshot: Some(WhoSnapshot {
            query: "ther".into(),
            entries: vec![WhoCharacterSnapshot {
                name: "Theron".into(),
                level: 12,
                class_name: "Paladin".into(),
                area: "Elwynn Forest".into(),
            }],
        }),
        message: Some("who: 1 result(s)".into()),
        error: None,
    };

    let encoded = serde_json::to_string(&update).unwrap();
    let decoded: WhoStateUpdate = serde_json::from_str(&encoded).unwrap();
    assert_eq!(update, decoded);
}

#[test]
fn calendar_state_update_round_trip() {
    let update = CalendarStateUpdate {
        snapshot: Some(CalendarSnapshot {
            events: vec![CalendarEventSnapshot {
                event_id: 7,
                title: "Karazhan".into(),
                organizer_name: "Theron".into(),
                starts_at_unix_secs: 1_710_000_000,
                max_signups: 10,
                is_raid: true,
                signups: vec![CalendarSignupSnapshot {
                    character_name: "Alice".into(),
                    status: CalendarSignupStatusSnapshot::Confirmed,
                }],
            }],
        }),
        message: Some("calendar updated".into()),
        error: None,
    };

    let encoded = serde_json::to_string(&update).unwrap();
    let decoded: CalendarStateUpdate = serde_json::from_str(&encoded).unwrap();
    assert_eq!(update, decoded);
}

#[test]
fn lfg_state_update_round_trip() {
    let update = LfgStateUpdate {
        snapshot: Some(LfgSnapshot {
            queued: true,
            selected_role: Some(GroupRoleSnapshot::Tank),
            dungeon_ids: vec![100],
            queue_size: 3,
            average_wait_secs: 42,
            in_demand_roles: vec![GroupRoleSnapshot::Healer],
            role_check: Some(LfgRoleCheckSnapshot {
                dungeon_id: 100,
                dungeon_name: "Deadmines".into(),
                assigned_role: GroupRoleSnapshot::Tank,
                accepted_count: 2,
                total_count: 5,
            }),
            match_found: Some(LfgMatchFoundSnapshot {
                dungeon_id: 100,
                dungeon_name: "Deadmines".into(),
                assigned_role: GroupRoleSnapshot::Tank,
                members: vec![LfgMatchMemberSnapshot {
                    name: "Theron".into(),
                    role: GroupRoleSnapshot::Tank,
                }],
            }),
        }),
        message: Some("role check started".into()),
        error: None,
    };

    let encoded = serde_json::to_string(&update).unwrap();
    let decoded: LfgStateUpdate = serde_json::from_str(&encoded).unwrap();
    assert_eq!(update, decoded);
}

#[test]
fn death_state_update_round_trip() {
    let update = DeathStateUpdate {
        snapshot: Some(DeathSnapshot {
            state: DeathStateSnapshot::Ghost,
            corpse: Some(DeathPositionSnapshot {
                map_id: 0,
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }),
            graveyard: Some(DeathPositionSnapshot {
                map_id: 0,
                x: 4.0,
                y: 5.0,
                z: 6.0,
            }),
            can_resurrect_at_corpse: true,
            spirit_healer_available: false,
        }),
        message: Some("released spirit".into()),
        error: None,
    };

    let encoded = serde_json::to_string(&update).unwrap();
    let decoded: DeathStateUpdate = serde_json::from_str(&encoded).unwrap();
    assert_eq!(update, decoded);
}

#[test]
fn durability_state_update_round_trip() {
    let update = DurabilityStateUpdate {
        snapshot: Some(DurabilitySnapshot {
            total_repair_cost: 1_250,
            slots: vec![
                DurabilitySlotSnapshot {
                    slot: EquipmentVisualSlot::Head,
                    current: 72,
                    max: 80,
                    repair_cost: 400,
                },
                DurabilitySlotSnapshot {
                    slot: EquipmentVisualSlot::Chest,
                    current: 45,
                    max: 100,
                    repair_cost: 850,
                },
            ],
        }),
        message: Some("durability updated".into()),
        error: None,
    };

    let encoded = serde_json::to_string(&update).unwrap();
    let decoded: DurabilityStateUpdate = serde_json::from_str(&encoded).unwrap();
    assert_eq!(update, decoded);
}

#[test]
fn rest_state_update_round_trip() {
    let update = RestStateUpdate {
        snapshot: Some(RestSnapshot {
            in_rest_area: true,
            rest_area_kind: Some(RestAreaKindSnapshot::Inn),
            rested_xp: 240,
            rested_xp_max: 600,
        }),
        message: Some("rest state updated".into()),
        error: None,
    };

    let encoded = serde_json::to_string(&update).unwrap();
    let decoded: RestStateUpdate = serde_json::from_str(&encoded).unwrap();
    assert_eq!(update, decoded);
}

#[test]
fn pvp_state_update_round_trip() {
    let update = PvpStateUpdate {
        snapshot: Some(PvpSnapshot {
            honor: 750,
            honor_max: 15_000,
            conquest: 120,
            conquest_max: 1_800,
            brackets: vec![
                PvpBracketStatsSnapshot {
                    bracket: PvpBracketSnapshot::Arena2v2,
                    rating: 1516,
                    season_wins: 1,
                    season_losses: 0,
                    weekly_wins: 1,
                    weekly_losses: 0,
                },
                PvpBracketStatsSnapshot {
                    bracket: PvpBracketSnapshot::RatedBattleground,
                    rating: 1500,
                    season_wins: 0,
                    season_losses: 0,
                    weekly_wins: 0,
                    weekly_losses: 0,
                },
            ],
            queue: Some(PvpQueueSnapshot {
                kind: PvpQueueKindSnapshot::Battleground {
                    battleground_id: 1,
                    name: "Warsong Gulch".into(),
                },
            }),
        }),
        message: Some("queued for Warsong Gulch".into()),
        error: None,
    };

    let encoded = serde_json::to_string(&update).unwrap();
    let decoded: PvpStateUpdate = serde_json::from_str(&encoded).unwrap();
    assert_eq!(update, decoded);
}

#[test]
fn barber_shop_state_update_round_trip() {
    let update = BarberShopStateUpdate {
        snapshot: Some(BarberShopSnapshot {
            appearance: CharacterAppearance {
                sex: 0,
                skin_color: 2,
                face: 3,
                eye_color: 4,
                hair_style: 5,
                hair_color: 6,
                facial_style: 1,
            },
            gold: 87_500,
        }),
        message: Some("barber shop ready".into()),
        error: None,
    };

    let encoded = serde_json::to_string(&update).unwrap();
    let decoded: BarberShopStateUpdate = serde_json::from_str(&encoded).unwrap();
    assert_eq!(update, decoded);
}

#[test]
fn combat_event_type_all_variants_serialize() {
    let variants = vec![
        CombatEventType::MeleeDamage,
        CombatEventType::SpellDamage,
        CombatEventType::SpellHeal,
        CombatEventType::PeriodicDamage,
        CombatEventType::PeriodicHeal,
        CombatEventType::Absorb,
        CombatEventType::Miss,
        CombatEventType::Dodge,
        CombatEventType::Parry,
        CombatEventType::Block,
        CombatEventType::CriticalHit,
        CombatEventType::Interrupt,
        CombatEventType::Death,
        CombatEventType::Respawn,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let decoded: CombatEventType = serde_json::from_str(&json).unwrap();
        assert_eq!(*v, decoded);
    }
    assert_eq!(variants.len(), 14);
}

#[test]
fn combat_event_round_trip() {
    let event = CombatEvent {
        attacker: 42,
        target: 99,
        amount: 1500.0,
        spell_id: 12345,
        event_type: CombatEventType::SpellDamage,
    };
    let json = serde_json::to_string(&event).unwrap();
    let decoded: CombatEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.attacker, 42);
    assert_eq!(decoded.target, 99);
    assert_eq!(decoded.amount, 1500.0);
    assert_eq!(decoded.spell_id, 12345);
    assert_eq!(decoded.event_type, CombatEventType::SpellDamage);
}
