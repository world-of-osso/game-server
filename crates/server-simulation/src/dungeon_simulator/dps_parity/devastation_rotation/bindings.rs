//! Native70448 identities cross-checked against pinned76f8118 Evoker bindings.
use crate::simc_apl_native::{Bindings, DotRefresh, DotRefreshPolicy};
use bevy::prelude::*;
use shared::components::PowerType;

const ACTIONS: &[(&str, u32)] = &[
    ("azure_strike", 362969),
    ("azure_sweep", 1265872),
    ("deep_breath", 357210),
    ("disintegrate", 356995),
    ("dragonrage", 375087),
    ("emerald_blossom", 355913),
    ("eternity_surge", 359073),
    ("fire_breath", 357208),
    ("fire_breath_damage", 357209),
    ("hover", 358267),
    ("living_flame", 361469),
    ("pyre", 357211),
    ("quell", 351338),
    ("tip_the_scales", 370553),
    ("unbound_flame", 1292321),
    ("verdant_embrace", 360995),
];

const BUFFS: &[(&str, u32)] = &[
    ("ancient_flame", 375583),
    ("burnout", 375802),
    ("charged_blast", 370454),
    ("dragonrage", 375087),
    ("essence_burst", 359618),
    ("fire_breath_damage", 357209),
    ("hover", 358267),
    ("leaping_flames", 370901),
    ("mass_disintegrate_stacks", 436336),
    ("rising_fury", 1271783),
    ("spymasters_report", 451199),
    ("strafing_run", 1266165),
    ("tip_the_scales", 370553),
];

const TALENTS: &[(&str, u32)] = &[
    ("ancient_flame", 369990),
    ("animosity", 375797),
    ("charged_blast", 370455),
    ("dragonrage", 375087),
    ("eternitys_span", 375757),
    ("feed_the_flames", 369846),
    ("imminent_destruction", 370781),
    ("legacy_of_the_lifebinder", 1264269),
    ("mass_disintegrate", 436335),
    // player.cpp12337–12355 selects the node's first entry, not any Rising Fury rank.
    ("rising_fury", 1271687),
    ("scarlet_adaptation", 372469),
    ("scorching_embers", 370819),
    ("slipstream", 441257),
    ("star_salvo", 1265826),
    ("volatility", 369089),
];

pub(super) fn build() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.into(), id))
            .collect()
    };
    Bindings {
        actions: names(ACTIONS),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("essence".into(), PowerType::Essence)].into(),
        global_cooldown_spell: 361469,
        dot_refresh: [(
            "fire_breath_damage".into(),
            DotRefresh {
                resolve: fire_breath_rank_one,
                policy: DotRefreshPolicy::Pandemic,
            },
        )]
        .into(),
    }
}

pub(super) fn execution_actions()
-> impl Iterator<Item = super::super::literal_rotation::execution::NativeAction<'static>> {
    ACTIONS.iter().map(
        |&(name, spell)| super::super::literal_rotation::execution::NativeAction {
            name,
            spell,
            talent: None,
            self_target: matches!(
                name,
                "fire_breath"
                    | "dragonrage"
                    | "hover"
                    | "tip_the_scales"
                    | "emerald_blossom"
                    | "verdant_embrace"
            ),
        },
    )
}

fn fire_breath_rank_one(
    world: &World,
    caster: Entity,
    _spell: u32,
) -> Result<crate::spell_info::SpellInfo, String> {
    // Generic refreshability creates fresh zero-empower state in dot.cpp458–477.
    Ok(server::test_support::fire_breath_release(world, caster, 1))
}
