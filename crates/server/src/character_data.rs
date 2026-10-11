use shared::components::{CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot};

#[path = "stored_equipment_appearance.rs"]
mod stored_equipment;
use stored_equipment::StoredEquipmentAppearance;

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Eq)]
pub struct DurabilityEntryData {
    pub slot: EquipmentVisualSlot,
    pub current: u32,
    pub max: u32,
}

// Runtime/wire fields must never be implicitly serialized into player storage.
#[derive(Debug, Clone, PartialEq)]
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
    pub durability: Vec<DurabilityEntryData>,
}

// V7 layout, unchanged by the addition of wire item definition sources.
#[derive(bitcode::Encode, bitcode::Decode)]
struct StoredCharacterData {
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
    equipment_appearance: StoredEquipmentAppearance,
    durability: Vec<DurabilityEntryData>,
}

impl From<&CharacterData> for StoredCharacterData {
    fn from(data: &CharacterData) -> Self {
        Self {
            name: data.name.clone(),
            x: data.x,
            y: data.y,
            z: data.z,
            health: data.health,
            max_health: data.max_health,
            race: data.race,
            class: data.class,
            level: data.level,
            appearance: data.appearance.clone(),
            equipment_appearance: (&data.equipment_appearance).into(),
            durability: data.durability.clone(),
        }
    }
}

impl From<StoredCharacterData> for CharacterData {
    fn from(data: StoredCharacterData) -> Self {
        Self {
            name: data.name,
            x: data.x,
            y: data.y,
            z: data.z,
            health: data.health,
            max_health: data.max_health,
            race: data.race,
            class: data.class,
            level: data.level,
            appearance: data.appearance,
            equipment_appearance: data.equipment_appearance.into(),
            durability: data.durability,
        }
    }
}

// Appearance written before the Dracthyr visage form (shared-protocol 88bc3fe).
#[derive(bitcode::Decode)]
struct LegacyCharacterAppearanceV3 {
    sex: u8,
    skin_color: u8,
    face: u8,
    eye_color: u8,
    hair_style: u8,
    hair_color: u8,
    facial_style: u8,
    customization_choices: Vec<shared::components::CustomizationChoiceSelection>,
}

#[derive(bitcode::Decode)]
struct LegacyCharacterDataV6 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: LegacyCharacterAppearanceV3,
    equipment_appearance: StoredEquipmentAppearance,
    durability: Vec<DurabilityEntryData>,
}

// Frozen seven-byte appearance used by records written before additional choices.
#[derive(bitcode::Decode)]
struct LegacyCharacterAppearanceV2 {
    sex: u8,
    skin_color: u8,
    face: u8,
    eye_color: u8,
    hair_style: u8,
    hair_color: u8,
    facial_style: u8,
}

#[derive(bitcode::Decode)]
struct LegacyCharacterDataV5 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: LegacyCharacterAppearanceV2,
    equipment_appearance: StoredEquipmentAppearance,
    durability: Vec<DurabilityEntryData>,
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
    appearance: LegacyCharacterAppearanceV2,
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
    equipment_appearance: StoredEquipmentAppearance,
}

#[derive(bitcode::Decode)]
struct LegacyCharacterDataV4 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: LegacyCharacterAppearanceV2,
    equipment_appearance: StoredEquipmentAppearance,
}

impl CharacterData {
    pub fn encode_stored(&self) -> Vec<u8> {
        bitcode::encode(&StoredCharacterData::from(self))
    }

    pub(crate) fn decode_stored(bytes: &[u8]) -> Result<Self, bitcode::Error> {
        bitcode::decode::<StoredCharacterData>(bytes).map(Into::into)
    }

    pub fn decode_compatible(bytes: &[u8]) -> Result<Self, String> {
        Self::decode_stored(bytes)
            .or_else(|_| decode_legacy_character_data(bytes))
            .map_err(|error| format!("Failed to decode character data: {error}"))
    }
}

fn decode_legacy_character_data(bytes: &[u8]) -> Result<CharacterData, bitcode::Error> {
    bitcode::decode::<LegacyCharacterDataV6>(bytes)
        .map(character_data_from_v6)
        .or_else(|_| bitcode::decode::<LegacyCharacterDataV5>(bytes).map(character_data_from_v5))
        .or_else(|_| {
            bitcode::decode::<LegacyCharacterDataV2>(bytes).map(character_data_from_legacy_v2)
        })
        .or_else(|_| bitcode::decode::<LegacyCharacterDataV4>(bytes).map(character_data_from_v4))
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

fn upgrade_appearance_v2(appearance: LegacyCharacterAppearanceV2) -> CharacterAppearance {
    CharacterAppearance {
        sex: appearance.sex,
        skin_color: appearance.skin_color,
        face: appearance.face,
        eye_color: appearance.eye_color,
        hair_style: appearance.hair_style,
        hair_color: appearance.hair_color,
        facial_style: appearance.facial_style,
        customization_choices: Vec::new(),
        visage: None,
    }
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
        customization_choices: Vec::new(),
        visage: None,
    }
}

macro_rules! upgrade_legacy {
    ($legacy:ident, $appearance:expr, $equipment:expr) => {{
        let appearance = $appearance;
        let equipment_appearance = ($equipment).into();
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
            durability: Vec::new(),
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
    upgrade_legacy!(
        legacy,
        upgrade_appearance_v2(legacy.appearance),
        EquipmentAppearance::default()
    )
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

fn character_data_from_v4(legacy: LegacyCharacterDataV4) -> CharacterData {
    upgrade_legacy!(
        legacy,
        upgrade_appearance_v2(legacy.appearance),
        legacy.equipment_appearance
    )
}

fn character_data_from_v6(legacy: LegacyCharacterDataV6) -> CharacterData {
    let durability = legacy.durability;
    let old = legacy.appearance;
    let mut data = upgrade_legacy!(
        legacy,
        CharacterAppearance {
            sex: old.sex,
            skin_color: old.skin_color,
            face: old.face,
            eye_color: old.eye_color,
            hair_style: old.hair_style,
            hair_color: old.hair_color,
            facial_style: old.facial_style,
            customization_choices: old.customization_choices,
            visage: None,
        },
        legacy.equipment_appearance
    );
    data.durability = durability;
    data
}

fn character_data_from_v5(legacy: LegacyCharacterDataV5) -> CharacterData {
    let durability = legacy.durability;
    let mut data = upgrade_legacy!(
        legacy,
        upgrade_appearance_v2(legacy.appearance),
        legacy.equipment_appearance
    );
    data.durability = durability;
    data
}

#[cfg(test)]
mod visage_upgrade_tests {
    use super::*;
    use shared::components::CustomizationChoiceSelection;

    /// The layout written before `CharacterAppearance.visage` (shared-protocol 88bc3fe).
    #[derive(bitcode::Encode)]
    struct AppearanceV3 {
        sex: u8,
        skin_color: u8,
        face: u8,
        eye_color: u8,
        hair_style: u8,
        hair_color: u8,
        facial_style: u8,
        customization_choices: Vec<CustomizationChoiceSelection>,
    }

    #[derive(bitcode::Encode)]
    struct DataV6 {
        name: String,
        x: f32,
        y: f32,
        z: f32,
        health: f32,
        max_health: f32,
        race: u8,
        class: u8,
        level: u16,
        appearance: AppearanceV3,
        equipment_appearance: StoredEquipmentAppearance,
        durability: Vec<DurabilityEntryData>,
    }

    #[test]
    fn records_written_before_the_visage_form_load_without_one() {
        let bytes = bitcode::encode(&DataV6 {
            name: "Elara".into(),
            x: 1.0,
            y: 2.0,
            z: 3.0,
            health: 90.0,
            max_health: 100.0,
            race: 4,
            class: 12,
            level: 8,
            appearance: AppearanceV3 {
                sex: 1,
                skin_color: 2,
                face: 3,
                eye_color: 4,
                hair_style: 5,
                hair_color: 6,
                facial_style: 7,
                customization_choices: vec![CustomizationChoiceSelection {
                    option_id: 48,
                    choice_id: 790,
                }],
            },
            equipment_appearance: StoredEquipmentAppearance::default(),
            durability: Vec::new(),
        });
        let data = CharacterData::decode_compatible(&bytes).unwrap();
        assert_eq!(
            (data.name.as_str(), data.level, data.class),
            ("Elara", 8, 12)
        );
        assert_eq!(data.appearance.hair_color, 6);
        assert_eq!(data.appearance.customization_choices[0].choice_id, 790);
        assert_eq!(data.appearance.visage, None);
    }
}
