//! TC a352b1fa boss_omor_the_unscarred.cpp:186–264, spell/aura hooks only.
use bevy::prelude::*;
use rand::seq::SliceRandom;
use shared::components::{Npc, ThreatTable};

pub(crate) const ORBITAL_TARGET: u32 = 35642;
pub(crate) const ORBITAL_WHIP: u32 = 32185;
const ORBITAL_KNOCK: u32 = 30637;
const ORBITAL_RADIUS: f32 = 20.0;
const SAY_KNOCK: u8 = 1;

/// Return the cast-local inverse of TC's _canContinue. A lone target remains
/// a spell hit, but its dummy hook does nothing, even outside the 20yd circle.
pub(super) fn filter_targets(world: &World, caster: Entity, units: &mut Vec<Entity>) -> bool {
    if units.len() <= 1 {
        return true;
    }
    let origin = super::targets::position(world, caster).expect("validated Orbital caster");
    units.retain(|&unit| {
        let position = super::targets::position(world, unit).expect("selected Orbital position");
        let offset = position - origin;
        let distance_squared = offset.x * offset.x + offset.z * offset.z;
        distance_squared <= ORBITAL_RADIUS * ORBITAL_RADIUS
    });
    units.shuffle(&mut rand::thread_rng());
    units.truncate(1);
    false
}

pub(super) fn cast_hit(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    super::cast_triggered(world, caster, ORBITAL_WHIP, Some(caster), depth + 1);
    super::cast_triggered(world, caster, ORBITAL_KNOCK, Some(target), depth + 1);
    if world.get::<Npc>(caster).is_some() {
        crate::scripted_ai::creature_text::talk_with_target(world, caster, SAY_KNOCK, Some(target));
    }
    if let Some(mut threat) = world.get_mut::<ThreatTable>(caster) {
        let amount = threat.threat_for(target.to_bits());
        // ModifyThreatByPercent(-100) retains the reference for Whip's iteration.
        threat.add_threat(target.to_bits(), -amount);
    }
}
