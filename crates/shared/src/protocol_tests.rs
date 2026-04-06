use super::*;
use crate::components::{EquipmentVisualSlot, EquippedAppearanceEntry};

#[test]
fn chat_type_serialization_round_trip() {
    let types = vec![
        ChatType::Say,
        ChatType::Yell,
        ChatType::Party,
        ChatType::Guild,
        ChatType::Whisper("TargetPlayer".into()),
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

fn sample_auction_search_results() -> AuctionSearchResults {
    AuctionSearchResults {
        query: AuctionSearchQuery {
            text: "linen".into(),
            page: 0,
            page_size: 20,
            min_level: Some(1),
            max_level: Some(10),
            quality: Some(1),
            usable_only: false,
            sort_field: AuctionSortField::Name,
            sort_dir: AuctionSortDir::Asc,
        },
        total_results: 1,
        results: vec![AuctionListingSummary {
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
        }],
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
