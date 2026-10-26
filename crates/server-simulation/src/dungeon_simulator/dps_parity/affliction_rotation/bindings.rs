//! SimC76f8118 sc_warlock_init.cpp129–238,604–636 native spell identities.
use super::super::literal_rotation::execution::NativeAction;
use super::*;
use crate::simc_apl_native::{DotRefresh, DotRefreshPolicy, native_dot_spell};
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool, Option<u32>)] = &[
    ("agony", 980, false, Some(980)),
    ("corruption", 172, false, None),
    ("wither", 445468, false, Some(445465)),
    ("haunt", 48181, false, Some(48181)),
    ("unstable_affliction", 1259790, false, Some(1259790)),
    ("seed_of_corruption", 27243, false, Some(27243)),
    ("dark_harvest", 1257052, false, Some(1257052)),
    ("summon_darkglare", 205180, true, Some(205180)),
    ("malevolence", 430014, true, Some(430014)),
    ("malefic_grasp", 1261153, false, Some(1261149)),
    ("drain_soul", 198590, false, Some(388667)),
    ("shadow_bolt", 686, false, None),
    ("grimoire_of_sacrifice", 108503, true, Some(108503)),
    ("berserking", 26297, true, None),
    ("blood_fury", 33702, true, None),
    ("fireblood", 265221, true, None),
    ("ancestral_call", 274738, true, None),
];
const BUFFS: &[(&str, u32)] = &[
    ("nightfall", 264571),
    ("shard_instability", 1260269),
    ("cascading_calamity", 1261125),
    ("unstable_empowerment", 1305774),
    ("succulent_soul", 449793),
    ("malevolence", 442726),
    ("agony", 980),
    ("corruption", 146739),
    ("wither", 445474),
    ("unstable_affliction", 1259790),
    ("seed_of_corruption", 27243),
];
const TALENTS: &[(&str, u32)] = &[
    ("grimoire_of_sacrifice", 108503),
    ("sow_the_seeds", 196226),
    ("siphon_life", 452999),
    ("cascading_calamity", 1261124),
    ("summon_darkglare", 205180),
    ("eye_contract", 1279521),
    ("wither", 445465),
];

pub(super) fn build() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.into(), id))
            .collect()
    };
    let mut bindings = Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _, _)| (name.into(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("soul_shard".into(), PowerType::SoulShards)].into(),
        global_cooldown_spell: 686,
        ..Default::default()
    };
    bindings.actions.insert("darkglare".into(), 205180);
    for alias in [
        "agony",
        "corruption",
        "wither",
        "unstable_affliction",
        "seed_of_corruption",
    ] {
        bindings.dot_refresh.insert(
            alias.into(),
            DotRefresh {
                resolve: native_dot_spell,
                policy: DotRefreshPolicy::Pandemic,
            },
        );
    }
    bindings
}

pub(super) fn execution_actions() -> impl Iterator<Item = NativeAction<'static>> {
    ACTIONS
        .iter()
        .map(|&(name, spell, self_target, talent)| NativeAction {
            name,
            spell,
            self_target,
            talent,
        })
}
