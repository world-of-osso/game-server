//! TC a352b1fa spell_druid.cpp271–295/319/1336/1934/2263/2821.
//! Native BeforeCast: switch to the script's form only when not already in it.
use bevy::prelude::*;

use super::CastFailure;
use crate::auras::ActiveAuras;
use crate::auras::shapeshift::current_form;
use crate::spell_info::{SpellInfo, SpellStore};

const BERSERK: u32 = 50334;
const RESET_COOLDOWNS: [u32; 3] = [33917, 77758, 6795];

const CAT: u32 = 768;
const BEAR: u32 = 5487;

// Foreground SimC druid_spell_base_t actions; form actions and Heart of the
// Wild explicitly opt out. The numbers after each action are sc_druid.cpp lines.
const DRUID_SWING_RESET_SPELLS: &[u32] = &[
    204066, // Lunar Beam: 5356
    145205, // Efflorescence: 6101
    102342, // Ironbark: 6179
    33763,  // Lifebloom: 6192
    88423,  // Nature's Cure: 6260
    132158, // Nature's Swiftness: 6266
    8936,   // Regrowth: 6300
    774, 155777, // Rejuvenation and Germination: 6448
    2782,   // Remove Corruption: 6521
    2908,   // Soothe: 6527
    18562,  // Swiftmend: 6535
    740,    // Tranquility: 6576
    48438,  // Wild Growth: 6611
    22842,  // Frenzied Regeneration: 6685
    22812,  // Barkskin: 7048
    194223, 102560, // Celestial Alignment, Incarnation: 7097–7133
    1850, 252216, // Dash, Tiger Dash: 7140
    1233272, 1233346, // Eclipse: 7182–7253
    339,     // Entangling Roots: 7267
    205636,  // Force of Nature: 7297
    202770,  // Fury of Elune: 7350
    33891,   // Incarnation: Tree of Life: 7448
    29166,   // Innervate: 7466
    274281, 274282, 274283,  // New, Half, Full Moon: 7607–7642
    8921,    // Moonfire: 7786
    5215,    // Prowl: 7952
    1252871, // Red Moon: 7998
    106839, 78675,   // Skull Bash and Solar Beam: 8174, 8186
    106898,  // Stampeding Roar: 8221
    191034,  // Starfall: 8258
    194153,  // Starfire: 8480
    78674,   // Starsurge: 8580, 8633
    93402,   // Sunfire: 8730
    202347,  // Stellar Flare: 8812
    102401,  // Wild Charge: 8842
    88747,   // Wild Mushroom: 8882
    1269658, // Wild Guardian: 9004
    5176, 190984, // Wrath: 9024
    391528, // Convoke the Spirits: 9193
];

pub(super) fn reset_swing_for_spell(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let druid = world
        .get::<shared::components::Player>(caster)
        .is_some_and(|player| player.class == 11);
    let gcd = spell
        .cooldowns
        .as_ref()
        .is_some_and(|cooldowns| cooldowns.start_recovery_ms > 0);
    if druid && gcd && DRUID_SWING_RESET_SPELLS.contains(&spell.id) {
        // SimC76f8118 sc_druid.cpp2518: "reset swing timer on execute (as most cast time spells do)".
        crate::combat::reset_player_main_hand_swing(world, caster);
    }
}

fn required_form(spell_id: u32) -> Option<(i32, u32)> {
    match spell_id {
        5215 | 252216 => Some((1, CAT)),
        99 | 50334 | 106898 => Some((5, BEAR)),
        _ => None,
    }
}

pub(super) fn before_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    depth: u8,
) -> Result<(), CastFailure> {
    let Some((form, child)) = required_form(spell.id) else {
        return Ok(());
    };
    if spell.id == BERSERK {
        let store = world.resource::<SpellStore>();
        for id in RESET_COOLDOWNS {
            if store.get(id).is_none() {
                return Err(CastFailure::Unsupported(format!(
                    "native Berserk reset child {id} is missing"
                )));
            }
        }
    }
    let current = world
        .get::<ActiveAuras>(caster)
        .and_then(current_form)
        .map(|(_, form)| form);
    if current == Some(form) {
        return Ok(());
    }
    if !super::try_cast_triggered(world, caster, child, Some(caster), depth + 1) {
        return Err(CastFailure::Unsupported(format!(
            "native Druid form child {child} could not be cast for {}",
            spell.id
        )));
    }
    Ok(())
}

/// TC spell_druid.cpp:330–345, SpellHistory.cpp:647–671/1091–1094.
/// Reset existing spell/category cooldowns, not charges or global cooldowns.
/// The native default update=false does not send an invented reset packet.
pub(super) fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != BERSERK {
        return;
    }
    let now = super::history_now(world);
    let categories: Vec<_> = {
        let store = world.resource::<SpellStore>();
        RESET_COOLDOWNS
            .into_iter()
            .map(|id| {
                let spell = store.get(id).expect("Berserk validated reset child");
                (
                    id,
                    spell
                        .categories
                        .as_ref()
                        .map_or(0, |category| category.category),
                )
            })
            .collect()
    };
    let Some(mut history) = world.get_mut::<super::SpellHistory>(caster) else {
        return;
    };
    for (id, category) in categories {
        if !history
            .spell_ready_at
            .get(&id)
            .is_some_and(|ready| *ready > now)
        {
            continue;
        }
        history.spell_ready_at.remove(&id);
        history.category_ready_at.remove(&category);
        history.category_spell_ids.remove(&category);
        history.item_cooldown_sources.remove(&id);
    }
}
