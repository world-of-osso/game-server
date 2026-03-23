use shared::components::{CharacterAppearance, EquipmentAppearance, Health, Player, Position};

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
    pub equipment_appearance: EquipmentAppearance,
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

#[derive(bitcode::Decode)]
pub(crate) struct LegacyCharacterDataV2 {
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
        // Position is in Bevy space; convert to WoW coords for DB storage.
        let (wx, wy, wz) = crate::terrain_height::bevy_to_wow(pos.x, pos.y, pos.z);
        Self {
            name: player.name.clone(),
            x: wx,
            y: wy,
            z: wz,
            health: health.current,
            max_health: health.max,
            race: 1,
            class: 1,
            level: 1,
            appearance: player.appearance,
            equipment_appearance: EquipmentAppearance::default(),
        }
    }

    pub fn decode_compatible(bytes: &[u8]) -> Result<Self, String> {
        bitcode::decode::<CharacterData>(bytes).or_else(|_| {
            bitcode::decode::<LegacyCharacterDataV2>(bytes)
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
                    appearance: legacy.appearance,
                    equipment_appearance: EquipmentAppearance::default(),
                })
                .or_else(|_| {
                    bitcode::decode::<LegacyCharacterDataV1>(bytes).map(|legacy| Self {
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
                        equipment_appearance: EquipmentAppearance::default(),
                    })
                })
                .map_err(|e| format!("Failed to decode character data: {e}"))
        })
    }
}
