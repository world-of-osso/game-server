//! SimC76f8118 sc_hunter.cpp spell lookups, with native70448 identities/values.
use super::super::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool)] = &[
    ("auto_shot", 75, false),
    ("aimed_shot", 19434, false),
    ("arcane_shot", 185358, false),
    ("rapid_fire", 257044, false),
    ("explosive_shot", 212431, false),
    ("volley", 260243, false),
    ("trueshot", 288613, true),
    ("multishot", 257620, false),
    ("kill_shot", 53351, false),
    ("steady_shot", 56641, false),
    ("moonlight_chakram", 1264949, false),
    ("black_arrow", 466930, false),
    ("wailing_arrow", 392060, false),
    ("berserking", 26297, true),
    ("blood_fury", 20572, true),
    ("ancestral_call", 274738, true),
    ("fireblood", 265221, true),
    ("lights_judgment", 255647, false),
    ("summon_pet", 883, true),
];

pub(super) fn build() -> Bindings {
    Bindings {
        actions: ACTIONS.iter().map(|&(n, id, _)| (n.into(), id)).collect(),
        buffs: [
            ("trueshot", 288613),
            ("bullseye", 204090),
            ("bloodlust", 2825),
            ("lock_and_load", 194594),
            ("precise_shots", 260242),
            ("trick_shots", 257622),
            ("moonlight_chakram", 1264946),
            ("bulletstorm", 389020),
            ("sentinels_mark", 1253601),
            ("spotters_mark", 466872),
            ("explosive_shot", 212431),
        ]
        .map(|(n, id)| (n.into(), id))
        .into(),
        talents: [
            ("unbreakable_bond", 1223323),
            ("bullseye", 204089),
            ("calling_the_shots", 260404),
            ("trick_shots", 257621),
            ("tactical_reload", 1301406),
            ("aspect_of_the_hydra", 470945),
            ("no_scope", 473385),
        ]
        .map(|(n, id)| (n.into(), id))
        .into(),
        resources: [("focus".into(), PowerType::Focus)].into(),
        global_cooldown_spell: 19434,
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
            talent: (name == "summon_pet").then_some(1223323),
        })
}
