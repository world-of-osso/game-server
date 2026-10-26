//! SimC76f8118 sc_hunter.cpp7695–7790/7864–7893 native spell identities.
use super::super::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool)] = &[
    ("kill_command", 259489, false),
    ("raptor_strike", 186270, false),
    ("wildfire_bomb", 259495, false),
    ("takedown", 1250646, true),
    ("boomstick", 1261193, false),
    ("moonlight_chakram", 1264949, false),
    ("muzzle", 187707, false),
    ("aspect_of_the_eagle", 186289, true),
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
            ("takedown", 1250646),
            ("tip_of_the_spear", 260286),
            ("raptor_swipe", 1273155),
            ("sentinels_mark", 1253601),
            ("moonlight_chakram", 1264946),
            ("power_infusion", 10060),
            ("howl_wyvern", 471878),
            ("howl_boar", 472324),
            ("howl_bear", 472325),
            ("howl_of_the_pack_leader_wyvern", 471878),
            ("howl_of_the_pack_leader_boar", 472324),
            ("howl_of_the_pack_leader_bear", 472325),
        ]
        .map(|(name, id)| (name.into(), id))
        .into(),
        talents: [
            ("howl_of_the_pack_leader", 471876),
            ("twin_fangs", 1272139),
            ("lethal_calibration", 1262409),
            ("takedown", 1250646),
        ]
        .map(|(name, id)| (name.into(), id))
        .into(),
        resources: [("focus".into(), PowerType::Focus)].into(),
        global_cooldown_spell: 186270,
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
