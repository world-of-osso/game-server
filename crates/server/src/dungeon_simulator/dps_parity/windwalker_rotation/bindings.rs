//! Native identities: pinned generated trait/racial tables and monk.cpp find_spell.
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

pub(super) const ACTIONS: &[(&str, u32)] = &[
    ("tiger_palm", 100780),
    ("blackout_kick", 100784),
    ("rising_sun_kick", 107428),
    ("fists_of_fury", 113656),
    ("spinning_crane_kick", 101546),
    ("strike_of_the_windlord", 392983),
    ("touch_of_death", 322109),
    ("touch_of_karma", 122470),
    ("whirling_dragon_punch", 152175),
    ("rushing_wind_kick", 467307),
    ("zenith", 1249625),
    ("zenith_stomp", 1272696),
    ("celestial_conduit", 443028),
    ("invoke_xuen_the_white_tiger", 123904),
    ("slicing_winds", 1217413),
    ("berserking", 26297),
    ("ancestral_call", 274738),
    ("blood_fury", 33697),
    ("fireblood", 265221),
    ("arcane_torrent", 129597),
    ("thorn_bloom", 1237885),
    ("haymaker", 287712),
    ("bag_of_tricks", 312411),
    ("arcane_pulse", 260364),
    ("rocket_barrage", 69041),
    ("lights_judgment", 255647),
    // Native ItemEffect211397/211403, spell recovery300000ms; inventory admission
    // is separate. An unconsumed native potion has no active cooldown.
    ("potion", 1236994),
];

pub(super) const TALENT_ACTIONS: &[(&str, u32)] = &[
    ("rising_sun_kick", 107428),
    ("fists_of_fury", 113656),
    ("strike_of_the_windlord", 392983),
    ("whirling_dragon_punch", 152175),
    ("rushing_wind_kick", 1250566),
    ("zenith", 1249625),
    ("zenith_stomp", 1272694),
    ("celestial_conduit", 1248989),
    ("invoke_xuen_the_white_tiger", 123904),
    ("slicing_winds", 1217413),
];

const BUFFS: &[(&str, u32)] = &[
    ("bloodlust", 2825),
    ("combo_breaker", 116768),
    ("dance_of_chiji", 325202),
    ("heart_of_the_jade_serpent", 443421),
    ("heart_of_the_jade_serpent_unity_within", 443616),
    ("heart_of_the_jade_serpent_yulons_avatar", 1238904),
    ("invoke_xuen_the_white_tiger", 123904),
    ("power_infusion", 10060),
    ("rushing_wind_kick", 1250554),
    ("tigereye_brew_1", 1261724),
    ("unbroken_rhythm", 1297033),
    ("whirling_dragon_punch", 196742),
    ("zenith", 1249625),
    ("zenith_stomp", 1291484),
];

const TALENTS: &[(&str, u32)] = &[
    ("zenith_stomp", 1272694),
    ("ascension", 115396),
    ("airborne_rhythm", 1248833),
    ("obsidian_spiral", 1249832),
    ("drinking_horn_cover", 391370),
    ("spiritual_focus", 280197),
    ("energy_burst", 451498),
    ("sequenced_strikes", 451515),
    ("meridian_strikes", 391330),
    ("shadowboxing_treads", 392982),
    ("revolving_whirl", 451524),
    ("echo_technique", 1250042),
    ("strike_of_the_windlord", 392983),
    ("whirling_dragon_punch", 152175),
    ("harmonic_combo", 1250041),
    ("rushing_wind_kick", 1250566),
    ("invoke_xuen_the_white_tiger", 123904),
    ("flurry_strikes", 450615),
    ("celestial_conduit", 1248989),
    ("restore_balance", 442719),
];

pub(super) const COMBO_HISTORY: &[u32] = &[
    100780, 100784, 107428, 113656, 101546, 392983, 322109, 152175, 467307, 1249625, 1272696,
    443028, 1217413,
];

pub(super) fn build() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    Bindings {
        actions: names(ACTIONS),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [
            ("chi".into(), PowerType::Chi),
            ("energy".into(), PowerType::Energy),
        ]
        .into(),
        global_cooldown_spell: 100780,
    }
}

pub(super) fn execution_actions()
-> impl Iterator<Item = super::super::literal_rotation::execution::NativeAction<'static>> {
    // No native potion inventory is provisioned; the WW report owns that gap.
    ACTIONS
        .iter()
        .filter(|(name, _)| *name != "potion")
        .map(
            |&(name, spell)| super::super::literal_rotation::execution::NativeAction {
                name,
                spell,
                self_target: self_target(name),
                talent: TALENT_ACTIONS
                    .iter()
                    .find(|(action, _)| *action == name)
                    .map(|(_, id)| *id),
            },
        )
}

fn self_target(name: &str) -> bool {
    matches!(
        name,
        "zenith"
            | "touch_of_karma"
            | "spinning_crane_kick"
            | "invoke_xuen_the_white_tiger"
            | "berserking"
            | "ancestral_call"
            | "blood_fury"
            | "fireblood"
            | "arcane_torrent"
    )
}
