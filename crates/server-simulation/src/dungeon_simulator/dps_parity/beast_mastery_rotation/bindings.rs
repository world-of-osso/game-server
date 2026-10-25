//! SimC76f8118 sc_hunter.cpp spell lookups; native70448 supplies all values.
use super::super::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool)] = &[
    ("auto_shot", 75, false),
    ("barbed_shot", 217200, false),
    ("bestial_wrath", 19574, true),
    ("kill_command", 34026, false),
    ("cobra_shot", 193455, false),
    ("wild_thrash", 1264359, false),
    ("black_arrow", 466930, false),
    ("wailing_arrow", 392060, false),
    ("berserking", 26297, true),
    ("blood_fury", 20572, true),
    ("ancestral_call", 274738, true),
    ("fireblood", 265221, true),
    ("summon_pet", 883, true),
];

pub(super) fn build() -> Bindings {
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.into(), id))
            .collect(),
        buffs: [
            ("barbed_shot", 217200),
            ("bestial_wrath", 19574),
            ("beast_cleave", 268877),
            ("natures_ally", 1276720),
            ("cobra_fang", 1299389),
            ("withering_fire", 466991),
            ("howl_of_the_pack_leader_cooldown", 471877),
            ("howl_wyvern", 471878),
            ("howl_boar", 472324),
            ("howl_bear", 472325),
        ]
        .map(|(name, id)| (name.into(), id))
        .into(),
        talents: [
            ("black_arrow", 466930),
            ("beast_cleave", 115939),
            ("wild_thrash", 1264359),
            ("master_handler", 424558),
            ("bloody_frenzy", 407412),
            ("snakeskin_quiver", 468695),
            ("jagged_wounds", 1265044),
            ("killer_cobra", 199532),
            ("serpentine_strikes", 468701),
            ("natures_ally_3", 1273126),
        ]
        .map(|(name, id)| (name.into(), id))
        .into(),
        resources: [("focus".into(), PowerType::Focus)].into(),
        global_cooldown_spell: 217200,
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
