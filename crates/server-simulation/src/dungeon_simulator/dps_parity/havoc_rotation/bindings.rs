//! SimC76f8118 sc_demon_hunter.cpp spell lookups; native70448 owns all values.
use super::super::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool)] = &[
    ("immolation_aura", 258920, true),
    ("metamorphosis", 191427, false),
    ("the_hunt", 370965, false),
    ("vengeful_retreat", 198793, true),
    ("reavers_glaive", 442294, false),
    ("annihilation", 201427, false),
    ("essence_break", 258860, false),
    ("death_sweep", 210152, true),
    ("throw_glaive", 185123, false),
    ("abyssal_gaze", 452497, false),
    ("eye_beam", 198013, false),
    ("felblade", 232893, false),
    ("blade_dance", 188499, true),
    ("chaos_strike", 162794, false),
    ("arcane_torrent", 202719, true),
    ("potion", 1236994, true),
];
const BUFFS: &[(&str, u32)] = &[
    ("burning_wound", 391191),
    ("reavers_glaive", 444686),
    ("rending_strike", 442442),
    ("glaive_flurry", 442435),
    ("reavers_mark", 442624),
    ("essence_break", 320338),
    ("initiative", 391215),
    ("eternal_hunt", 1271092),
    ("inertia_trigger", 1215159),
    ("demonsurge", 452416),
    ("metamorphosis", 162264),
];
const TALENTS: &[(&str, u32)] = &[
    ("burning_wound", 391189),
    ("violent_transformation", 452409),
    ("a_fire_inside", 427775),
    ("chaotic_transformation", 388112),
    ("eternal_hunt", 1270898),
    ("screaming_brutality", 1220506),
    ("initiative", 388108),
];

pub(super) fn build() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.to_owned(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("fury".into(), PowerType::Fury)].into(),
        global_cooldown_spell: 162794,
        ..Default::default()
    }
}

pub(super) fn execution_actions() -> impl Iterator<Item = NativeAction<'static>> {
    // Prepared injects the actual profile potion independently of class spells;
    // physical soul-fragment pickups remain unavailable.
    ACTIONS
        .iter()
        .filter(|(name, _, _)| *name != "potion")
        .map(|&(name, spell, self_target)| NativeAction {
            name,
            spell,
            self_target,
            talent: None,
        })
}
