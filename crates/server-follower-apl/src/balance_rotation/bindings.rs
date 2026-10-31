//! Pinned sc_druid.cpp Balance identities; native action knowledge owns normalization.
use bevy::prelude::*;
use shared::components::PowerType;

use crate::simc_apl_native::{Bindings, DotRefresh, DotRefreshPolicy, native_dot_spell};

/// SimC76f8118 sc_druid.cpp::create_action (10149–10254); spec spells from
/// class_spells.inc / trait_data.inc for spec102.
pub const ACTIONS: &[(&str, u32, bool)] = &[
    ("moonkin_form", 24858, true),
    ("wrath", 190984, false),
    ("starfire", 194153, false),
    ("moonfire", 8921, false),
    ("sunfire", 93402, false),
    ("starsurge", 78674, false),
    ("starfall", 191034, true),
    ("convoke_the_spirits", 391528, true),
    ("force_of_nature", 205636, false),
    ("fury_of_elune", 202770, false),
    ("new_moon", 274281, false),
    ("half_moon", 274282, false),
    ("full_moon", 274283, false),
    ("wild_mushroom", 88747, false),
    ("berserking", 26297, true),
    // Native potion inventory is not provisioned; retain its authored row without admission.
    ("potion", 1236616, true),
];

/// SimC76f8118 sc_druid.cpp::create_buffs (11039–11610).
pub const BUFFS: &[(&str, u32)] = &[
    ("bloodlust", 2825),
    ("moonkin_form", 24858),
    ("eclipse_lunar", 48518),
    ("eclipse_solar", 48517),
    ("lunar_eclipse_override", 1233353),
    ("celestial_alignment", 194223),
    ("incarnation_chosen_of_elune", 102560),
    ("harmony_of_the_grove", 428735),
    ("starweavers_warp", 393942),
    ("starweavers_weft", 393944),
    ("touch_the_cosmos", 450360),
    ("orbit_breaker", 1303480),
    ("balance_of_all_things_arcane", 394050),
    ("balance_of_all_things_nature", 394049),
    ("ascendant_stars", 1263382),
    ("ascendant_fires", 1263363),
    ("atmospheric_exposure", 430589),
    ("moonfire", 164812),
    ("sunfire", 164815),
];

/// SimC76f8118 trait_data.inc spec102 talent spells.
pub const TALENTS: &[(&str, u32)] = &[
    ("aetherial_kindling", 327541),
    ("celestial_alignment", 395022),
    ("convoke_the_spirits", 391528),
    ("dream_surge", 433831),
    ("force_of_nature", 205636),
    ("fury_of_elune", 202770),
    ("incarnation_chosen_of_elune", 394013),
    ("lunation", 429539),
    ("meteor_storm", 1240262),
    ("meteorites", 1240907),
    ("power_of_goldrinn", 394046),
    ("radiant_moonlight", 394121),
    ("rattle_the_stars", 393954),
    ("starweaver", 393940),
    ("stellar_amplification", 450212),
];

pub fn names(entries: &[(&str, u32)]) -> std::collections::BTreeMap<String, u32> {
    entries
        .iter()
        .map(|&(name, id)| (name.to_owned(), id))
        .collect()
}

pub fn build(world: &World, actor: Entity) -> Result<Bindings, String> {
    let mut bindings = Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.to_owned(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [
            ("astral_power".into(), PowerType::LunarPower),
            ("mana".into(), PowerType::Mana),
        ]
        .into(),
        global_cooldown_spell: 190984,
        ..Bindings::default()
    };
    // SimC76f8118 sc_druid.cpp10149–10160/13486–13491: ca_inc is the
    // selected Incarnation or Celestial Alignment.
    let ca_inc =
        server::spell_cast::knowledge::resolve_action(world, actor, 194223)?.unwrap_or(194223);
    for alias in ["celestial_alignment", "ca_inc"] {
        bindings.actions.insert(alias.into(), ca_inc);
    }
    bindings.buffs.insert("ca_inc".into(), ca_inc);
    // SimC76f8118 sc_druid.cpp7255–7257: Lunar Calling casts Lunar Eclipse.
    let lunar_calling = world
        .get::<crate::class_progression::KnownSpells>(actor)
        .is_some_and(|known| known.contains(429523));
    let eclipse = if lunar_calling { 1233272 } else { 1233346 };
    bindings.actions.insert("eclipse".into(), eclipse);
    // SimC action.cpp4630–4635: ordinary pandemic refresh window.
    for alias in ["moonfire", "sunfire"] {
        bindings.dot_refresh.insert(
            alias.into(),
            DotRefresh {
                resolve: native_dot_spell,
                policy: DotRefreshPolicy::Pandemic,
            },
        );
    }
    Ok(bindings)
}

pub fn execution_actions(
    bindings: &Bindings,
) -> impl Iterator<Item = crate::literal_rotation::execution::NativeAction<'static>> + '_ {
    use crate::literal_rotation::execution::NativeAction;
    let actions = ACTIONS.iter().filter(|(name, _, _)| *name != "potion").map(
        |&(name, spell, self_target)| NativeAction {
            name,
            spell,
            self_target,
            talent: None, // Canonical action knowledge owns selected-trait admission.
        },
    );
    let selected = ["celestial_alignment", "eclipse"].map(|name| NativeAction {
        name,
        spell: bindings.actions[name],
        self_target: true,
        talent: None,
    });
    actions.chain(selected)
}
