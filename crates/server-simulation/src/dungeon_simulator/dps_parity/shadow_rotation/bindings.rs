//! SimC76f8118 sc_priest_shadow.cpp2390–2475/sc_priest.cpp3454 native identities.
use super::super::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::{Bindings, DotRefresh, DotRefreshPolicy, native_dot_spell};
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool, Option<u32>)] = &[
    ("shadowform", 232698, true, None),
    ("mind_blast", 8092, false, None),
    ("mind_flay", 15407, false, None),
    ("mind_flay_insanity", 391403, false, Some(391399)),
    ("shadow_word_pain", 589, false, None),
    ("shadow_word_madness", 335467, false, Some(335467)),
    ("shadow_word_death", 32379, false, Some(32379)),
    ("vampiric_touch", 34914, false, None),
    ("tentacle_slam", 1227280, false, Some(1227280)),
    ("void_torrent", 263165, false, Some(263165)),
    ("void_blast", 450983, false, Some(450405)),
    ("void_volley", 1242173, false, Some(228260)),
    // sc_priest_shadow.cpp1384–1388 fires Volley at the enemy; native root E0 targets6.
    ("voidform", 228260, false, Some(228260)),
    ("power_infusion", 10060, true, Some(10060)),
    ("halo", 120644, true, Some(120644)),
    ("holy_nova", 132157, true, None),
    ("desperate_prayer", 19236, true, Some(19236)),
    ("arcane_torrent", 232633, true, Some(232633)),
    ("fireblood", 265221, true, Some(265221)),
    ("berserking", 26297, true, Some(26297)),
    ("blood_fury", 33702, true, Some(33702)),
    ("ancestral_call", 274738, true, Some(274738)),
];
const TALENTS: &[(&str, u32)] = &[
    ("ancient_madness", 1231346),
    ("deathspeaker", 392507),
    ("devour_matter", 451840),
    ("distorted_reality", 409044),
    ("halo", 120644),
    ("idol_of_yshaarj", 373310),
    ("inescapable_torment", 373427),
    ("invoked_nightmare", 1279350),
    ("lightburst", 1246549),
    ("maddening_tentacles", 1279353),
    ("mind_devourer", 373202),
    ("resonant_energy", 453845),
    ("shadowfiend", 34433),
    ("thought_harvester", 406788),
    ("void_apparitions", 1264096),
    ("void_blast", 450405),
    ("voidform", 228260),
    ("tentacle_slam", 1227280),
];
const BUFFS: &[(&str, u32)] = &[
    ("shadowform", 232698),
    ("voidform", 194249),
    ("power_infusion", 10060),
    ("bloodlust", 2825),
    ("mind_devourer", 373204),
    ("entropic_rift", 459314),
    ("crushing_void", 1279437),
    ("resonant_energy_damage", 453850),
    ("twist_of_fate", 390978),
    ("hex_lords_doom", 1295885),
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
        resources: [
            ("insanity".into(), PowerType::Insanity),
            ("mana".into(), PowerType::Mana),
        ]
        .into(),
        global_cooldown_spell: 8092,
        ..Default::default()
    };
    // Bind summon identities for the shared live pet.* facts, not executable APL rows.
    bindings.actions.extend(
        [
            ("shadowfiend", 34433),
            ("mindbender", 200174),
            ("voidwraith", 451235),
        ]
        .map(|(name, spell)| (name.into(), spell)),
    );
    for name in ["shadow_word_pain", "shadow_word_madness", "vampiric_touch"] {
        bindings.buffs.insert(name.into(), bindings.actions[name]);
        bindings.dot_refresh.insert(
            name.into(),
            DotRefresh {
                resolve: native_dot_spell,
                policy: DotRefreshPolicy::Pandemic,
            },
        );
    }
    bindings
}

pub(super) fn actions() -> impl Iterator<Item = NativeAction<'static>> {
    ACTIONS
        .iter()
        .map(|&(name, spell, self_target, talent)| NativeAction {
            name,
            spell,
            self_target,
            talent,
        })
}
