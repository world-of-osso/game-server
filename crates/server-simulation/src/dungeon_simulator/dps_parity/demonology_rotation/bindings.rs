//! SimC76f8118 sc_warlock_init.cpp Demonology/Diabolist find_spell identities.
use super::*;

const ACTIONS: &[(&str, u32, bool, Option<u32>)] = &[
    ("shadow_bolt", 686, false, None),
    ("demonbolt", 264178, false, None),
    ("hand_of_guldan", 105174, false, None),
    ("call_dreadstalkers", 104316, false, Some(104316)),
    ("summon_demonic_tyrant", 265187, true, Some(265187)),
    ("power_siphon", 264130, true, Some(264130)),
    ("implosion", 196277, false, Some(196277)),
    // Native roots retain their enemy Dummy marker (target6), even though
    // their summon effects use caster-relative destinations.
    ("grimoire_imp_lord", 1276452, false, Some(1276452)),
    ("grimoire_fel_ravager", 1276467, false, Some(1276467)),
    ("summon_doomguard", 1276672, false, Some(1276672)),
    ("ruination", 434635, false, Some(428522)),
    ("infernal_bolt", 434506, false, Some(428518)),
    ("berserking", 26297, true, None),
    ("blood_fury", 33702, true, None),
    ("fireblood", 265221, true, None),
    ("ancestral_call", 274738, true, None),
];
const BUFFS: &[(&str, u32)] = &[
    ("demonic_core", 264173),
    ("power_siphon", 334581),
    ("dominion_of_argus", 1276166),
    ("infernal_bolt", 433891),
    ("ruination", 433885),
    ("doom", 460553),
];
const TALENTS: &[(&str, u32)] = &[
    ("ruination", 428522),
    ("grimoire_imp_lord", 1276452),
    ("summon_doomguard", 1276672),
    ("diabolic_ritual", 428514),
    ("demonic_soul", 449614),
    ("reign_of_tyranny", 1276748),
    ("to_hell_and_back", 1281511),
    ("doom", 460551),
    ("inner_demons", 267216),
    ("summon_demonic_tyrant", 265187),
];
pub(super) fn build() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.into(), id))
            .collect()
    };
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _, _)| (name.into(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("soul_shard".into(), PowerType::SoulShards)].into(),
        global_cooldown_spell: 686,
        ..Default::default()
    }
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
