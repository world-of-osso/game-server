//! SimC76f8118 sc_warlock_init.cpp386–509,538–579,604–636 native identities.
use super::super::literal_rotation::execution::NativeAction;
use super::*;
use crate::simc_apl_native::{DotRefresh, DotRefreshPolicy, native_dot_spell};
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool, Option<u32>)] = &[
    ("incinerate", 29722, false, None),
    ("immolate", 348, false, None),
    ("wither", 445468, false, Some(445465)),
    ("chaos_bolt", 116858, false, Some(116858)),
    ("conflagrate", 17962, false, Some(17962)),
    ("soul_fire", 6353, false, Some(6353)),
    ("shadowburn", 17877, false, Some(17877)),
    ("summon_infernal", 1122, false, Some(1122)),
    ("malevolence", 430014, true, Some(430014)),
    ("havoc", 80240, false, Some(80240)),
    ("rain_of_fire", 5740, false, Some(5740)),
    ("cataclysm", 152108, false, Some(152108)),
    ("grimoire_of_sacrifice", 108503, true, Some(108503)),
    ("ruination", 434635, false, Some(428522)),
    ("infernal_bolt", 434506, false, Some(428518)),
    ("berserking", 26297, true, None),
    ("blood_fury", 33702, true, None),
    ("fireblood", 265221, true, None),
    ("ancestral_call", 274738, true, None),
];
const BUFFS: &[(&str, u32)] = &[
    ("backdraft", 117828),
    ("malevolence", 442726),
    ("fiendish_cruelty", 1245664),
    ("alythesss_ire", 1244947),
    ("infernal_bolt", 433891),
    ("ruination", 433885),
    ("diabolic_ritual_overlord", 431944),
    ("diabolic_ritual_mother_of_chaos", 432815),
    ("diabolic_ritual_pit_lord", 432816),
    ("demonic_art_overlord", 428524),
    ("demonic_art_mother_of_chaos", 432794),
    ("demonic_art_pit_lord", 432795),
    ("immolate", 157736),
    ("wither", 445474),
    ("havoc", 80240),
];
const TALENTS: &[(&str, u32)] = &[
    ("wither", 445465),
    ("diabolic_ritual", 428514),
    ("soul_fire", 6353),
    ("cataclysm", 152108),
    ("roaring_blaze", 1244310),
    ("backdraft", 196406),
    ("summon_infernal", 1122),
    ("malevolence", 430014),
    ("conflagration_of_chaos", 387108),
    ("internal_combustion", 266134),
    ("lake_of_fire", 1244877),
    ("destructive_rapidity", 1244928),
    ("avatar_of_destruction", 1245089),
    ("fire_and_brimstone", 196408),
    ("grimoire_of_sacrifice", 108503),
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
        global_cooldown_spell: 29722,
        ..Default::default()
    };
    // Native1122 queues the actual111685 guardian summon.
    bindings.actions.insert("infernal".into(), 111685);
    for alias in ["immolate", "wither"] {
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
