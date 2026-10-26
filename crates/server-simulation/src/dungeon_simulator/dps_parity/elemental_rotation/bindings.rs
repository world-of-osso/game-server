//! Pinned sc_shaman.cpp spell identities; native70448 supplies all values.
use crate::simc_apl_native::{Bindings, DotRefresh, DotRefreshPolicy, native_dot_spell};
use shared::components::PowerType;

// SimC76f8118 sc_shaman.cpp::create_action / init_spells (10339–10511,11330–11431).
const ACTIONS: &[(&str, u32, bool, Option<u32>)] = &[
    ("lightning_bolt", 188196, false, None),
    ("chain_lightning", 188443, false, None),
    ("lava_burst", 51505, false, None),
    // Native70448 SkillLine924 learns470411; its effect64 casts188389's damage/DoT.
    ("flame_shock", 470411, false, None),
    ("earth_shock", 8042, false, Some(8042)),
    ("elemental_blast", 117014, false, Some(117014)),
    ("earthquake", 61882, false, Some(61882)),
    ("stormkeeper", 191634, true, Some(191634)),
    ("ascendance", 1219480, true, Some(114050)),
    ("ancestral_swiftness", 443454, true, Some(448861)),
    ("natures_swiftness", 378081, true, Some(378081)),
    ("flametongue_weapon", 318038, true, Some(318038)),
    ("thunderstrike_ward", 462757, true, Some(462757)),
    ("lightning_shield", 192106, true, None),
    ("spiritwalkers_grace", 79206, true, Some(79206)),
    ("wind_shear", 57994, false, Some(57994)),
    ("voltaic_blaze", 470057, false, Some(470057)),
    ("tempest", 452201, false, Some(454009)),
    ("frost_shock", 196840, false, Some(196840)),
    ("blood_fury", 33702, true, None),
    ("berserking", 26297, true, None),
    ("fireblood", 265221, true, None),
    ("ancestral_call", 274738, true, None),
    // Native potion inventory is not provisioned; retain its authored row without admission.
    ("potion", 1236616, true, None),
];

// SimC76f8118 sc_shaman.cpp12620–12740,12914–12923.
const BUFFS: &[(&str, u32)] = &[
    ("bloodlust", 2825),
    ("ascendance", 1219480),
    ("stormkeeper", 191634),
    ("lightning_shield", 192106),
    ("master_of_the_elements", 260734),
    ("flame_shock", 188389),
    ("lightning_rod", 197209),
    ("fire_elemental", 188592),
    ("storm_elemental", 157299),
    ("lava_surge", 77762),
    ("wind_gust", 263806),
    ("call_of_the_ancestors", 447244),
    ("purging_flames", 1259491),
    ("tempest", 454015),
    ("elemental_blast_critical_strike", 118522),
    ("elemental_blast_haste", 173183),
    ("elemental_blast_mastery", 173184),
    ("flowing_elements", 1300219),
    ("overcharge_tier", 1300222),
];

const TALENTS: &[(&str, u32)] = &[
    ("flametongue_weapon", 318038),
    ("master_of_the_elements", 16166),
    ("ancestral_swiftness", 448861),
    ("elemental_blast", 117014),
    ("tempest", 454009),
    ("call_of_the_ancestors", 443450),
    ("inferno_arc", 1259047),
    ("purging_flames", 1259471),
    ("eye_of_the_storm", 381708),
    ("molten_wrath", 1258843),
    ("crackling_fury", 1269215),
];

pub(super) fn build() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _, _)| (name.to_owned(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("maelstrom".into(), PowerType::Maelstrom)].into(),
        global_cooldown_spell: 188196,
        // SimC action.cpp4630–4635: ordinary pandemic refresh window.
        dot_refresh: [(
            "flame_shock".into(),
            DotRefresh {
                resolve: native_dot_spell,
                policy: DotRefreshPolicy::Pandemic,
            },
        )]
        .into(),
    }
}

pub(super) fn execution_actions()
-> impl Iterator<Item = super::super::literal_rotation::execution::NativeAction<'static>> {
    ACTIONS
        .iter()
        .filter(|(name, _, _, _)| *name != "potion")
        .map(|&(name, spell, self_target, talent)| {
            super::super::literal_rotation::execution::NativeAction {
                name,
                spell,
                self_target,
                talent,
            }
        })
}
