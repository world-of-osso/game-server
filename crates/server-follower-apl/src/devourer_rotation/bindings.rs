//! SimC76f8118 sc_demon_hunter.cpp Devourer spell lookups; native70448 owns values.
use crate::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

pub const ACTIONS: &[(&str, u32, bool)] = &[
    ("arcane_torrent", 202719, true),
    ("soul_immolation", 1241937, true),
    ("consume", 473662, false),
    ("devour", 1217610, false),
    ("metamorphosis", 1217605, true),
    ("the_hunt", 1246167, false),
    ("vengeful_retreat", 198793, true),
    ("voidblade", 1245412, false),
    ("void_ray", 473728, false),
    ("collapsing_star", 1221150, false),
    ("reap", 1226019, false),
    ("cull", 1245453, false),
    ("eradicate", 1225826, false),
    ("hungering_slash", 1239123, false),
    ("reapers_toll", 1245470, false),
    ("pierce_the_veil", 1245483, false),
    ("predators_wake", 1259431, false),
];
pub const BUFFS: &[(&str, u32)] = &[
    ("metamorphosis", 1217607),
    ("void_metamorphosis_stack", 1225789),
    ("collapsing_star", 1221171),
    ("collapsing_star_stacking", 1227702),
    ("eradicate", 1239524),
    ("hungering_slash", 1239525),
    ("moment_of_craving", 1238495),
    ("soulburst", 1297433),
    ("voidstep", 1223157),
    ("voidfall_spending", 1256302),
];
pub const TALENTS: &[(&str, u32)] = &[
    ("eradicate", 1226033),
    ("devourers_bite", 1240201),
    ("scythes_embrace", 1246558),
    ("voidrush", 1272422),
    ("the_hunt", 1246167),
];

pub fn build() -> Bindings {
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
        global_cooldown_spell: 473662,
        ..Default::default()
    }
}

pub fn execution_actions() -> impl Iterator<Item = NativeAction<'static>> {
    ACTIONS
        .iter()
        .map(|&(name, spell, self_target)| NativeAction {
            name,
            spell,
            self_target,
            talent: None,
        })
}
