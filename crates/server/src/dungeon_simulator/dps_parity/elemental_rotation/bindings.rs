//! Native70448 identities; SimC76f8118 shaman.cpp supplies literal aliases.
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

pub(super) const ACTIONS: &[(&str, u32)] = &[
    ("flametongue_weapon", 318038),
    ("lightning_shield", 192106),
    ("thunderstrike_ward", 462757),
    ("stormkeeper", 191634),
    ("spiritwalkers_grace", 79206),
    ("wind_shear", 57994),
    ("blood_fury", 33697),
    ("berserking", 26297),
    ("fireblood", 265221),
    ("ancestral_call", 274738),
    ("natures_swiftness", 378081),
    ("ancestral_swiftness", 443454),
    ("ascendance", 1219480),
    ("flame_shock", 188389),
    ("voltaic_blaze", 470057),
    ("elemental_blast", 117014),
    ("earthquake", 61882),
    ("earth_shock", 8042),
    ("lava_burst", 51505),
    ("tempest", 452201),
    ("chain_lightning", 188443),
    ("lightning_bolt", 188196),
    ("frost_shock", 196840),
    ("potion", 1236994),
];

const BUFFS: &[(&str, u32)] = &[
    ("ascendance", 1219480),
    ("bloodlust", 2825),
    ("call_of_the_ancestors", 447244),
    ("elemental_blast_critical_strike", 118522),
    ("elemental_blast_haste", 173183),
    ("elemental_blast_mastery", 173184),
    ("fire_elemental", 188592),
    ("flowing_elements", 1300219),
    ("lava_surge", 77762),
    ("lightning_shield", 192106),
    // Selected native16166 effect0 triggers260734, not the Enhancement462377.
    ("master_of_the_elements", 260734),
    ("overcharge_tier", 1300222),
    ("purging_flames", 1259491),
    ("storm_elemental", 157299),
    ("stormkeeper", 191634),
    ("tempest", 454015),
    ("wind_gust", 263806),
];

const TALENTS: &[(&str, u32)] = &[
    ("call_of_the_ancestors", 443450),
    ("crackling_fury", 1269215),
    ("elemental_blast", 117014),
    ("eye_of_the_storm", 381708),
    ("flametongue_weapon", 318038),
    ("inferno_arc", 1259047),
    ("master_of_the_elements", 16166),
    ("molten_wrath", 1258843),
    ("purging_flames", 1259471),
    ("tempest", 454009),
];

pub(super) fn build() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    Bindings {
        actions: names(ACTIONS),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("maelstrom".into(), PowerType::Maelstrom)].into(),
        global_cooldown_spell: 188196,
    }
}

pub(super) fn execution_actions()
-> impl Iterator<Item = super::super::literal_rotation::execution::NativeAction<'static>> {
    ACTIONS
        .iter()
        // Potion provision belongs to the shared native inventory adapter.
        .filter(|(name, _)| *name != "potion")
        .map(
            |&(name, spell)| super::super::literal_rotation::execution::NativeAction {
                name,
                spell,
                // Native book/overrides decide availability, not a talent-ID alias.
                talent: None,
                self_target: matches!(
                    name,
                    "flametongue_weapon"
                        | "lightning_shield"
                        | "thunderstrike_ward"
                        | "stormkeeper"
                        | "spiritwalkers_grace"
                        | "blood_fury"
                        | "berserking"
                        | "fireblood"
                        | "ancestral_call"
                        | "natures_swiftness"
                        | "ancestral_swiftness"
                        | "ascendance"
                ),
            },
        )
}
