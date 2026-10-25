//! Pinned Mage identities; all amounts and admission remain native70448.
use super::super::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

// SimC76f8118 sc_mage.cpp5712–5779: class action names.
const ACTIONS: &[(&str, u32, bool)] = &[
    ("arcane_intellect", 1459, true),
    ("summon_water_elemental", 31687, true),
    ("blizzard", 190356, false),
    ("glacial_spike", 199786, false),
    ("frostbolt", 116, false),
    ("ray_of_frost", 205021, false),
    ("flurry", 44614, false),
    ("frozen_orb", 84714, true),
    ("comet_storm", 153595, false),
    ("ice_lance", 30455, false),
    ("ice_nova", 157997, false),
    ("cone_of_cold", 120, true),
    ("any_blink", 1953, true),
    ("blood_fury", 33702, true),
    ("berserking", 26297, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
];

// SimC76f8118 sc_mage.cpp5651/6450–6477: aura identities differ from casts.
pub(super) const BUFFS: &[(&str, u32)] = &[
    ("brain_freeze", 190446),
    ("fingers_of_frost", 44544),
    ("freezing_rain", 270232),
    ("glacial_spike", 1222865),
    ("icicles", 205473),
    ("rapid_refreezing", 1310248),
    ("thermal_void", 1247730),
    ("frostfire_empowerment", 431177),
    ("power_infusion", 10060),
    ("freezing", 1221389),
];

// sc_mage.cpp6195–6220: native specialization traits; Frostfire is a hero trait.
const TALENTS: &[(&str, u32)] = &[
    ("frostfire_bolt", 431044),
    ("freezing_winds", 1216953),
    ("freezing_rain", 270233),
    ("wintertide", 378406),
    // player.cpp12334–12352/trait_data.cpp101–133: the pinned fourth-name
    // lookup exceeds its three entries and resolves the first apex trait.
    ("hand_of_frost_4", 1262935),
    ("crystalline_refraction", 1247544),
    ("cone_of_frost", 1247090),
];

pub(super) fn build() -> Bindings {
    let names = |rows: &[(&str, u32)]| rows.iter().map(|&(name, id)| (name.into(), id)).collect();
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.into(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("mana".into(), PowerType::Mana)].into(),
        global_cooldown_spell: 116,
        ..Default::default()
    }
}

pub(super) fn actions() -> impl Iterator<Item = NativeAction<'static>> {
    ACTIONS
        .iter()
        .map(|&(name, spell, self_target)| NativeAction {
            name,
            spell,
            self_target,
            talent: None,
        })
}
