//! Native70448 identities qualified against spell_name and selected trait definitions.
use super::super::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

const ACTIONS: &[(&str, u32)] = &[
    ("arcane_intellect", 1459),
    ("mirror_image", 55342),
    ("fireball", 133),
    ("pyroblast", 11366),
    ("flamestrike", 1254851),
    ("fire_blast", 108853),
    ("combustion", 190319),
    ("meteor", 153561),
    ("scorch", 2948),
    ("ancestral_call", 274738),
    ("berserking", 26297),
    ("blood_fury", 33697),
    ("fireblood", 265221),
];

const BUFFS: &[(&str, u32)] = &[
    ("bloodlust", 2825),
    ("combustion", 190319),
    // Pinned sc_mage.cpp6477; native spell_name431177.
    ("frostfire_empowerment", 431177),
    ("heat_shimmer", 458964),
    ("heating_up", 48107),
    ("hot_streak", 48108),
    ("hyperthermia", 383874),
    ("power_infusion", 10060),
    ("pyroclasm", 269651),
];

const TALENTS: &[(&str, u32)] = &[
    ("blast_zone", 416719),
    ("burnout", 1271177),
    ("firestarter", 205026),
    ("frostfire_bolt", 431044),
    ("fuel_the_fire", 416094),
    ("pyroclasm", 269650),
    ("scald", 450746),
    ("scorch", 2948),
    ("spellfire_spheres", 448601),
    ("spontaneous_combustion", 451875),
    ("sunfury_execution", 449349),
];

pub(super) fn build() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.into(), id))
            .collect()
    };
    Bindings {
        actions: names(ACTIONS),
        buffs: names(BUFFS),
        // The pinned Fire source has no dot.* refresh operands to bind.
        dot_refresh: Default::default(),
        talents: names(TALENTS),
        resources: [("mana".into(), PowerType::Mana)].into(),
        global_cooldown_spell: 133,
    }
}

pub(super) fn execution_actions() -> impl Iterator<Item = NativeAction<'static>> {
    // Native KnownSpells/overrides and cast admission decide usability. No potion
    // inventory or external buff provider is provisioned by this fixture.
    ACTIONS.iter().map(|&(name, spell)| NativeAction {
        name,
        spell,
        talent: None,
        self_target: matches!(
            name,
            "arcane_intellect"
                | "mirror_image"
                | "combustion"
                | "ancestral_call"
                | "berserking"
                | "blood_fury"
                | "fireblood"
        ),
    })
}
