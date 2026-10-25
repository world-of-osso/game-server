//! Native70448 identities qualified against spell_name and selected trait definitions.
use super::super::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::Bindings;
use bevy::prelude::*;
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

pub(super) fn build(world: &World, actor: Entity) -> Result<Bindings, String> {
    let traits = world
        .get::<crate::class_progression::TraitLoadout>(actor)
        .ok_or("native Fire selected traits absent")?;
    // SimC76f8118 sc_mage.cpp3888–3891: selected2120, otherwise selected1254851.
    let flamestrike = if traits
        .active_talent_spells()
        .any(|spell| spell.spell_id == 2120)
    {
        2120
    } else {
        1254851
    };
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.into(), id))
            .collect()
    };
    let mut bindings = Bindings {
        actions: names(ACTIONS),
        buffs: names(BUFFS),
        // The pinned Fire source has no dot.* refresh operands to bind.
        dot_refresh: Default::default(),
        talents: names(TALENTS),
        resources: [("mana".into(), PowerType::Mana)].into(),
        global_cooldown_spell: 133,
    };
    bindings.actions.insert("flamestrike".into(), flamestrike);
    Ok(bindings)
}

pub(super) fn execution_actions(
    bindings: &Bindings,
) -> impl Iterator<Item = NativeAction<'static>> + '_ {
    // Native KnownSpells/overrides and cast admission decide usability. No potion
    // inventory or external buff provider is provisioned by this fixture.
    ACTIONS.iter().map(move |&(name, _)| NativeAction {
        name,
        spell: bindings.actions[name],
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
