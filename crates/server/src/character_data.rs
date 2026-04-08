use shared::components::{CharacterAppearance, EquipmentAppearance};

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
struct LegacyCharacterDataV1 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
}

#[derive(bitcode::Decode)]
struct LegacyCharacterDataV2 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: CharacterAppearance,
}

#[derive(bitcode::Decode)]
struct LegacyCharacterAppearanceV1 {
    sex: u8,
    skin_color: u8,
    face: u8,
    hair_style: u8,
    hair_color: u8,
    facial_style: u8,
}

#[derive(bitcode::Decode)]
struct LegacyCharacterDataV2AppearanceV1 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: LegacyCharacterAppearanceV1,
}

#[derive(bitcode::Decode)]
struct LegacyCharacterDataV3AppearanceV1 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: LegacyCharacterAppearanceV1,
    equipment_appearance: EquipmentAppearance,
}

impl CharacterData {
    pub fn decode_compatible(bytes: &[u8]) -> Result<Self, String> {
        bitcode::decode::<CharacterData>(bytes)
            .or_else(|_| decode_legacy_character_data(bytes))
            .map_err(|error| format!("Failed to decode character data: {error}"))
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

fn upgrade_appearance_v1(appearance: LegacyCharacterAppearanceV1) -> CharacterAppearance {
    CharacterAppearance {
        sex: appearance.sex,
        skin_color: appearance.skin_color,
        face: appearance.face,
        eye_color: 0,
        hair_style: appearance.hair_style,
        hair_color: appearance.hair_color,
        facial_style: appearance.facial_style,
    }
}

macro_rules! upgrade_legacy {
    ($legacy:ident, $appearance:expr, $equipment:expr) => {{
        let appearance = $appearance;
        let equipment_appearance = $equipment;
        CharacterData {
            name: $legacy.name,
            x: $legacy.x,
            y: $legacy.y,
            z: $legacy.z,
            health: $legacy.health,
            max_health: $legacy.max_health,
            race: $legacy.race,
            class: $legacy.class,
            level: $legacy.level,
            appearance,
            equipment_appearance,
        }
    }};
}

fn character_data_from_v1(legacy: LegacyCharacterDataV1) -> CharacterData {
    upgrade_legacy!(
        legacy,
        CharacterAppearance::default(),
        EquipmentAppearance::default()
    )
}

fn character_data_from_legacy_v2(legacy: LegacyCharacterDataV2) -> CharacterData {
    upgrade_legacy!(legacy, legacy.appearance, EquipmentAppearance::default())
}

fn character_data_from_legacy_v2_appearance_v1(
    legacy: LegacyCharacterDataV2AppearanceV1,
) -> CharacterData {
    upgrade_legacy!(
        legacy,
        upgrade_appearance_v1(legacy.appearance),
        EquipmentAppearance::default()
    )
}

fn character_data_from_legacy_v3_appearance_v1(
    legacy: LegacyCharacterDataV3AppearanceV1,
) -> CharacterData {
    upgrade_legacy!(
        legacy,
        upgrade_appearance_v1(legacy.appearance),
        legacy.equipment_appearance
    )
}
