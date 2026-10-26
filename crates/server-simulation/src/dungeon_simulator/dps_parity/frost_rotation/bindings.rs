//! Pinned SimC identities; native records supply every value and cast permission.
use super::super::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

// SimC76f8118 sc_death_knight.cpp14390–14460 action constructors and
// 15306–15355 buff lookups. Flags identify native caster-targeted actions.
const ACTIONS: &[(&str, u32, bool)] = &[
    ("obliterate", 49020, false),
    ("howling_blast", 49184, false),
    ("frost_strike", 49143, false),
    ("frostscythe", 207230, false),
    ("glacial_advance", 194913, false),
    ("remorseless_winter", 196770, true),
    ("pillar_of_frost", 51271, true),
    ("breath_of_sindragosa", 1249658, true),
    ("reapers_mark", 439843, false),
    ("frostwyrms_fury", 279302, false),
    ("raise_dead", 46585, true),
    // SimC76f8118 sc_death_knight.cpp10168–10179: harmful AoE at the enemy.
    ("empower_rune_weapon", 47568, false),
    ("antimagic_shell", 48707, true),
    ("mind_freeze", 47528, false),
    ("blood_fury", 20572, true),
    ("berserking", 26297, true),
    ("arcane_pulse", 260364, true),
    ("lights_judgment", 255647, false),
    ("ancestral_call", 274738, true),
    ("fireblood", 265221, true),
    ("bag_of_tricks", 312411, false),
];

const BUFFS: &[(&str, u32)] = &[
    ("bonegrinder_frost", 377103),
    ("breath_of_sindragosa", 1249658),
    ("chosen_of_frostbrood_fwf", 1265639),
    ("chosen_of_frostbrood_haste", 1265630),
    ("frostbane", 1229310),
    ("gathering_storm", 211805),
    ("icy_onslaught", 1230273),
    ("killing_machine", 51124),
    ("pillar_of_frost", 51271),
    ("remorseless_winter", 196770),
    ("rime", 59052),
    ("unholy_strength", 53365),
    ("razorice", 51714),
    ("reapers_mark_debuff", 434765),
    ("chains_of_ice_trollbane_slow", 444826),
    ("frost_fever", 55095),
];

const TALENTS: &[(&str, u32)] = &[
    ("apocalypse_now", 444040),
    ("bonegrinder", 377098),
    ("breath_of_sindragosa", 1249658),
    ("chosen_of_frostbrood", 1265637),
    ("deathly_blows", 1265932),
    ("frostbane", 455993),
    ("frostbound_will", 1238680),
    ("gathering_storm", 194912),
    ("icy_onslaught", 1230272),
    ("killing_streak", 1230153),
    ("obliteration", 281238),
    ("pillar_of_frost", 51271),
    ("shattering_blade", 207057),
];

pub(super) fn build() -> Bindings {
    let names = |rows: &[(&str, u32)]| rows.iter().map(|&(name, id)| (name.into(), id)).collect();
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.into(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [
            ("rune".into(), PowerType::Runes),
            ("runic_power".into(), PowerType::RunicPower),
        ]
        .into(),
        global_cooldown_spell: 49020,
        ..Default::default()
    }
}

pub(super) fn execution_actions() -> impl Iterator<Item = NativeAction<'static>> {
    ACTIONS
        .iter()
        .map(|&(name, spell, self_target)| NativeAction {
            name,
            spell,
            self_target,
            talent: None,
        })
}
