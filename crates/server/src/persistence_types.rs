use shared::components::{EquipmentAppearance, Health, Player, Position};

#[path = "character_data.rs"]
mod character_data;

pub use character_data::{CharacterData, DurabilityEntryData};

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

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Default)]
pub struct GuildProfileData {
    pub motd: String,
    pub info_text: String,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Default)]
pub struct GuildMemberNoteData {
    pub officer_note: String,
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
    /// Only one of this item_id may be listed on the auction house at a time.
    pub unique_auction: bool,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Default)]
pub struct InventoryData {
    /// Copper, capped at `gold::MAX_MONEY`.
    pub gold: u64,
    pub items: Vec<InventoryItemData>,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct AuctionData {
    pub auction_id: u64,
    pub seller_character_id: u64,
    pub seller_name: String,
    pub item: InventoryItemData,
    pub stack_count: u32,
    pub min_bid: u64,
    pub current_bid: Option<u64>,
    pub current_bidder_character_id: Option<u64>,
    pub buyout_price: Option<u64>,
    pub expires_at_unix_secs: u64,
    /// 0 = neutral, 1 = Alliance, 2 = Horde
    pub faction: u8,
    /// Deposit paid on listing, refunded on successful sale.
    pub deposit: u64,
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
            appearance: player.appearance.clone(),
            equipment_appearance: EquipmentAppearance::default(),
            durability: Vec::new(),
        }
    }

    pub fn sync_live_components(&mut self, player: &Player, pos: &Position, health: &Health) {
        // Position is in Bevy space; convert to WoW coords for DB storage.
        let (wx, wy, wz) = crate::terrain_height::bevy_to_wow(pos.x, pos.y, pos.z);
        self.name = player.name.clone();
        self.x = wx;
        self.y = wy;
        self.z = wz;
        self.health = health.current;
        self.max_health = health.max;
        self.appearance = player.appearance.clone();
    }
}
