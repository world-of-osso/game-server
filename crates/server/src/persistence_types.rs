use shared::components::{CharacterAppearance, Health, Player, Position};

/// Serializable character data for persistence.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct CharacterData {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32,
    pub max_health: f32,
    pub race: u8,
    pub class: u8,
    pub level: u16,
    pub appearance: CharacterAppearance,
}

#[derive(bitcode::Decode)]
pub(crate) struct LegacyCharacterDataV1 {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32,
    pub max_health: f32,
    pub race: u8,
    pub class: u8,
    pub level: u16,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct GuildMembershipData {
    pub guild_id: u32,
    pub guild_name: String,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct PendingGuildInviteData {
    pub guild_id: u32,
    pub guild_name: String,
    pub inviter_name: String,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct InventoryItemData {
    pub item_guid: u64,
    pub item_id: u32,
    pub name: String,
    pub quality: u8,
    pub required_level: u16,
    pub stack_count: u32,
    pub vendor_sell_price: u32,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct InventoryData {
    pub gold: u32,
    pub items: Vec<InventoryItemData>,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct MailEntryData {
    pub mail_id: u64,
    pub subject: String,
    pub body: String,
    pub attached_money: u32,
    pub attached_item: Option<InventoryItemData>,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct AuctionData {
    pub auction_id: u64,
    pub seller_character_id: u64,
    pub seller_name: String,
    pub item: InventoryItemData,
    pub stack_count: u32,
    pub min_bid: u32,
    pub current_bid: Option<u32>,
    pub current_bidder_character_id: Option<u64>,
    pub buyout_price: Option<u32>,
    pub expires_at_unix_secs: u64,
}

impl CharacterData {
    pub fn from_components(player: &Player, pos: &Position, health: &Health) -> Self {
        Self {
            name: player.name.clone(),
            x: pos.x,
            y: pos.y,
            z: pos.z,
            health: health.current,
            max_health: health.max,
            race: 1,
            class: 1,
            level: 1,
            appearance: player.appearance,
        }
    }

    pub fn decode_compatible(bytes: &[u8]) -> Result<Self, String> {
        bitcode::decode::<CharacterData>(bytes).or_else(|_| {
            bitcode::decode::<LegacyCharacterDataV1>(bytes)
                .map(|legacy| Self {
                    name: legacy.name,
                    x: legacy.x,
                    y: legacy.y,
                    z: legacy.z,
                    health: legacy.health,
                    max_health: legacy.max_health,
                    race: legacy.race,
                    class: legacy.class,
                    level: legacy.level,
                    appearance: CharacterAppearance::default(),
                })
                .map_err(|e| format!("Failed to decode character data: {e}"))
        })
    }
}
