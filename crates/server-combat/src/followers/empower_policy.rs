//! SimC76f8118 apl_evoker.cpp es/fb_fs/fb_sc rank ordering on native known talents.
use bevy::prelude::*;

const MASS_DISINTEGRATE: u32 = 436335;
const ETERNITYS_SPAN: u32 = 375757;
const STAR_SALVO: u32 = 1265826;
const DRAGONRAGE: u32 = 375087;
const RISING_FURY: u32 = 1271783;

fn knows(world: &World, unit: Entity, spell: u32) -> bool {
    world
        .get::<crate::class_progression::KnownSpells>(unit)
        .is_some_and(|known| known.contains(spell))
}

pub(super) fn selected_stage(
    world: &World,
    unit: Entity,
    spell: u32,
    enemies: usize,
) -> Option<u8> {
    match spell {
        359073 | 382411 => {
            let span = usize::from(knows(world, unit, ETERNITYS_SPAN));
            let salvo = usize::from(knows(world, unit, STAR_SALVO));
            Some(surge_stage(enemies, span, salvo))
        }
        357208 | 382266 => {
            let scalecommander = knows(world, unit, MASS_DISINTEGRATE);
            let burst = crate::auras::has_aura(world, unit, DRAGONRAGE)
                || crate::auras::has_aura(world, unit, RISING_FURY);
            Some(fire_stage(enemies, scalecommander, burst))
        }
        _ => None,
    }
}

fn surge_stage(enemies: usize, span: usize, salvo: usize) -> u8 {
    // Literal APL: without Star Salvo equality, not a rounded target/rank heuristic.
    for rank in (2..=4).rev() {
        let targets = rank * (1 + span);
        let qualifies = if salvo == 0 {
            enemies == targets
        } else {
            enemies >= targets - salvo
        };
        if qualifies {
            return rank as u8;
        }
    }
    1
}

fn fire_stage(enemies: usize, scalecommander: bool, burst: bool) -> u8 {
    if enemies <= 1 || (scalecommander && enemies > 4) {
        return 1;
    }
    if scalecommander {
        return enemies.min(4) as u8;
    }
    // Current native starter kit has no Legacy of the Lifebinder (use_pyre_fs=false).
    // That hero branch and its purchased Volatility ranks remain separately unqualified.
    if enemies == 2 || burst { 2 } else { 3 }
}
