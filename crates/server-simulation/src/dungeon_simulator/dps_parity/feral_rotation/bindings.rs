//! Pinned druid spell/trait identities; native action knowledge owns normalization.
use bevy::prelude::*;
use shared::components::PowerType;

use crate::simc_apl_native::{Bindings, DotRefresh, DotRefreshPolicy, native_dot_spell};

pub(super) const ACTIONS: &[(&str, u32)] = &[
    ("cat_form", 768),
    ("prowl", 5215),
    ("tigers_fury", 5217),
    ("rake", 1822),
    ("chomp", 1244258),
    ("ferocious_bite", 22568),
    ("regrowth", 8936),
    ("shadowmeld", 58984),
    ("moonfire_cat", 155625),
    ("shred", 5221),
    ("swipe_cat", 106785),
    ("primal_wrath", 285381),
    ("rip", 1079),
    ("berserking", 26297),
    ("feral_frenzy", 274837),
    ("frantic_frenzy", 1243807),
    ("convoke_the_spirits", 391528),
    // Native liquid-luster metadata, not a provisioned potion inventory.
    ("potion", 1236994),
];

const BUFFS: &[(&str, u32)] = &[
    ("apex_predators_craving", 391882),
    ("cat_form", 768),
    ("chomp_enabler", 1244393),
    ("clearcasting", 135700),
    ("potion", 1236994),
    ("predatory_swiftness", 69369),
    ("prowl", 5215),
    ("ravage", 441585),
    ("shadowmeld", 58984),
    ("sudden_ambush", 1244483),
    ("tigers_fury", 5217),
    ("rake", 155722),
    ("moonfire_cat", 155625),
    ("rip", 1079),
    // sc_druid.cpp4745: Primal Wrath's dot_name is the actual Rip aura.
    ("primal_wrath", 1079),
    ("bloodseeker_vines", 439531),
];

const TALENTS: &[(&str, u32)] = &[
    ("ashamanes_guidance", 391548),
    ("berserk_heart_of_the_lion", 391174),
    ("convoke_the_spirits", 391528),
    ("doubleclawed_rake", 391700),
    ("fluid_form", 449193),
    ("frantic_frenzy", 1243807),
    ("lunar_inspiration", 155580),
    ("merciless_claws", 231063),
    ("panthers_guile", 1280316),
    ("primal_wrath", 285381),
    ("rampant_ferocity", 391709),
    ("wild_slashes", 390864),
];

pub(super) fn execution_actions(
    bindings: &Bindings,
) -> impl Iterator<Item = super::super::literal_rotation::execution::NativeAction<'static>> + '_ {
    use super::super::literal_rotation::execution::NativeAction;
    let actions = ACTIONS
        .iter()
        .filter(|(name, _)| *name != "potion")
        .map(|&(name, spell)| NativeAction {
            name,
            spell,
            talent: None, // Canonical action knowledge owns selected-trait admission.
            self_target: matches!(
                name,
                "cat_form"
                    | "prowl"
                    | "tigers_fury"
                    | "regrowth"
                    | "shadowmeld"
                    | "berserking"
                    | "convoke_the_spirits"
            ),
        });
    actions.chain(std::iter::once(NativeAction {
        name: "berserk",
        spell: bindings.actions["berserk"],
        talent: None,
        self_target: true,
    }))
}

fn names(entries: &[(&str, u32)]) -> std::collections::BTreeMap<String, u32> {
    entries
        .iter()
        .map(|&(name, id)| (name.to_owned(), id))
        .collect()
}

pub(super) fn build(world: &World, actor: Entity) -> Result<Bindings, String> {
    let berserk = server::spell_cast::knowledge::resolve_action(world, actor, 106951)?
        .ok_or("native Feral reference lacks its selected Berserk/Incarnation action")?;
    let mut bindings = Bindings {
        actions: names(ACTIONS),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [
            ("energy".into(), PowerType::Energy),
            ("combo_points".into(), PowerType::ComboPoints),
        ]
        .into(),
        global_cooldown_spell: 5221,
        ..Bindings::default()
    };
    bindings.actions.insert("berserk".into(), berserk);
    bindings.actions.insert("bs_inc".into(), berserk);
    bindings.buffs.insert("bs_inc".into(), berserk);
    for alias in ["rake", "moonfire_cat"] {
        bindings.dot_refresh.insert(
            alias.into(),
            DotRefresh {
                resolve: native_dot_spell,
                policy: DotRefreshPolicy::Pandemic,
            },
        );
    }
    // Rip's 4000ms/resource duration needs the shared CP-prepared reader.
    // Its absent refresh entry is an explicit NativeFacts error, not a base-duration substitute.
    Ok(bindings)
}
