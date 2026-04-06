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

#[derive(bitcode::Decode)]
pub(crate) struct LegacyCharacterAppearanceV1 {
    pub sex: u8,
    pub skin_color: u8,
    pub face: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub facial_style: u8,
}

#[derive(bitcode::Decode)]
pub(crate) struct LegacyCharacterDataV2AppearanceV1 {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32,
    pub max_health: f32,
    pub race: u8,
    pub class: u8,
    pub level: u16,
    pub appearance: LegacyCharacterAppearanceV1,
}

#[derive(bitcode::Decode)]
pub(crate) struct LegacyCharacterDataV3AppearanceV1 {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32,
    pub max_health: f32,
    pub race: u8,
    pub class: u8,
    pub level: u16,
    pub appearance: LegacyCharacterAppearanceV1,
    pub equipment_appearance: EquipmentAppearance,
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
    /// 0 = neutral, 1 = Alliance, 2 = Horde
    pub faction: u8,
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

    pub fn sync_live_components(&mut self, player: &Player, pos: &Position, health: &Health) {
        // Position is in Bevy space; convert to WoW coords for DB storage.
        let (wx, wy, wz) = crate::terrain_height::bevy_to_wow(pos.x, pos.y, pos.z);
        self.name = player.name.clone();
        self.x = wx;
        self.y = wy;
        self.z = wz;
        self.health = health.current;
        self.max_health = health.max;
        self.appearance = player.appearance;
    }

    pub fn decode_compatible(bytes: &[u8]) -> Result<Self, String> {
        bitcode::decode::<CharacterData>(bytes)
            .or_else(|_| decode_legacy_character_data(bytes))
            .map_err(|e| format!("Failed to decode character data: {e}"))
    }
}

fn decode_legacy_character_data(bytes: &[u8]) -> Result<CharacterData, bitcode::Error> {
    bitcode::decode::<LegacyCharacterDataV2>(bytes)
        .map(character_data_from_legacy_v2)
        .or_else(|_| {
            bitcode::decode::<LegacyCharacterDataV3AppearanceV1>(bytes)
                .map(character_data_from_legacy_v3_appearance_v1)
        })
        .or_else(|_| {
            bitcode::decode::<LegacyCharacterDataV2AppearanceV1>(bytes)
                .map(character_data_from_legacy_v2_appearance_v1)
        })
        .or_else(|_| bitcode::decode::<LegacyCharacterDataV1>(bytes).map(character_data_from_v1))
}

fn character_data_from_legacy_v2(legacy: LegacyCharacterDataV2) -> CharacterData {
    CharacterData {
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
    }
}

fn character_data_from_legacy_v2_appearance_v1(
    legacy: LegacyCharacterDataV2AppearanceV1,
) -> CharacterData {
    CharacterData {
        name: legacy.name,
        x: legacy.x,
        y: legacy.y,
        z: legacy.z,
        health: legacy.health,
        max_health: legacy.max_health,
        race: legacy.race,
        class: legacy.class,
        level: legacy.level,
        appearance: CharacterAppearance {
            sex: legacy.appearance.sex,
            skin_color: legacy.appearance.skin_color,
            face: legacy.appearance.face,
            eye_color: 0,
            hair_style: legacy.appearance.hair_style,
            hair_color: legacy.appearance.hair_color,
            facial_style: legacy.appearance.facial_style,
        },
        equipment_appearance: EquipmentAppearance::default(),
    }
}

fn character_data_from_legacy_v3_appearance_v1(
    legacy: LegacyCharacterDataV3AppearanceV1,
) -> CharacterData {
    CharacterData {
        name: legacy.name,
        x: legacy.x,
        y: legacy.y,
        z: legacy.z,
        health: legacy.health,
        max_health: legacy.max_health,
        race: legacy.race,
        class: legacy.class,
        level: legacy.level,
        appearance: CharacterAppearance {
            sex: legacy.appearance.sex,
            skin_color: legacy.appearance.skin_color,
            face: legacy.appearance.face,
            eye_color: 0,
            hair_style: legacy.appearance.hair_style,
            hair_color: legacy.appearance.hair_color,
            facial_style: legacy.appearance.facial_style,
        },
        equipment_appearance: legacy.equipment_appearance,
    }
}

fn character_data_from_v1(legacy: LegacyCharacterDataV1) -> CharacterData {
    CharacterData {
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
    }
}
