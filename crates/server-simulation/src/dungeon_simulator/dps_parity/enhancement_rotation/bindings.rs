//! SimC76f8118 sc_shaman.cpp10339-10420/11203-11330 native spell identities.
use super::super::literal_rotation::execution::NativeAction;
use crate::simc_apl_native::Bindings;

const ACTIONS: &[(&str, u32, bool, Option<u32>)] = &[
    ("windfury_weapon", 33757, true, None),
    ("flametongue_weapon", 318038, true, None),
    ("lightning_shield", 192106, true, None),
    ("bloodlust", 2825, true, None),
    ("stormstrike", 17364, false, None),
    ("windstrike", 115356, false, None),
    ("lava_lash", 60103, false, None),
    ("crash_lightning", 187874, false, None),
    ("flame_shock", 188389, false, None),
    ("lightning_bolt", 188196, false, None),
    ("chain_lightning", 188443, false, None),
    ("tempest", 452201, false, Some(454009)),
    ("voltaic_blaze", 470057, false, Some(470057)),
    ("sundering", 197214, false, Some(197214)),
    ("doom_winds", 384352, true, Some(384352)),
    ("ascendance", 114051, true, Some(114051)),
    ("surging_totem", 444995, false, Some(455630)),
    ("primordial_storm", 1218090, false, Some(1218047)),
    ("berserking", 26297, true, None),
    ("blood_fury", 33697, true, None),
    ("fireblood", 265221, true, None),
    ("ancestral_call", 274738, true, None),
];

const BUFFS: &[(&str, u32)] = &[
    ("ascendance", 114051),
    ("maelstrom_weapon", 344179),
    ("crash_lightning", 187878),
    ("doom_winds", 466772),
    ("primordial_storm", 1218125),
    ("whirling_air", 453409),
    ("whirling_earth", 453406),
    ("whirling_fire", 453405),
    ("hot_hand", 215785),
    ("converging_storms", 198300),
    ("flame_shock", 188389),
    ("lashing_flames", 334168),
    ("lightning_rod", 197209),
    ("chaos_brand", 1490),
    ("hunters_mark", 259556),
];

const TALENTS: &[(&str, u32)] = &[
    ("ascendance", 114051),
    ("doom_winds", 384352),
    ("deeply_rooted_elements", 378270),
    ("elemental_tempo", 1250364),
    ("feral_spirit", 469314),
    ("fire_nova", 1260666),
    ("lashing_flames", 334046),
    ("splitstream", 445035),
    ("storm_unleashed", 1262713),
    ("surging_elements", 382042),
    ("surging_totem", 455630),
    ("thorims_invocation", 384444),
];

pub(super) fn build() -> Bindings {
    let names = |entries: &[(&str, u32)]| -> std::collections::BTreeMap<String, u32> {
        entries
            .iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    let mut actions = names(
        &ACTIONS
            .iter()
            .map(|&(name, id, _, _)| (name, id))
            .collect::<Vec<_>>(),
    );
    // Native458101 summons the actual Lively Totems Searing Totem2523.
    actions.insert("searing_totem".into(), 458101);
    Bindings {
        actions,
        buffs: names(BUFFS),
        talents: names(TALENTS),
        global_cooldown_spell: 17364,
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
