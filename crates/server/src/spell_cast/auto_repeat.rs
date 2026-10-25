//! SPELL_ATTR2_AUTO_REPEAT casts (Auto Shot75): TC a352b1fa
//! Unit::_UpdateAutoRepeatSpell (Unit.cpp3008–3044). A successful cast becomes the
//! unit's repeat; every shot resets the ranged attack timer (Spell::SendSpellCooldown,
//! Spell.cpp4229–4238) and the next one fires through the ordinary checks without a
//! GCD (TRIGGERED_IGNORE_GCD). The idle instant repeat passes CheckMovement
//! (Spell.cpp7175–7197), so moving does not stop it. A failed check skips Auto Shot
//! but cancels other repeats. Cast-in-progress skips, ranged-haste auras and death
//! are not modelled.
use bevy::prelude::*;
use shared::components::WeaponType;
use std::time::Duration;

use super::{SpellStore, attributes, execute_cast};
use crate::stats::CharacterStats;

const SPELL_ATTR2_AUTO_REPEAT: u32 = 0x20;
const AUTO_SHOT: u32 = 75;
/// `BASE_ATTACK_TIME`: RANGED_ATTACK without a ranged main hand (Player.cpp8178).
const BASE_ATTACK_SECS: f32 = 2.0;

/// The unit's current autorepeat spell and its ranged attack timer.
#[derive(Component, Debug)]
pub(crate) struct AutoRepeat {
    spell_id: u32,
    target: Entity,
    ready_in: Duration,
}

impl AutoRepeat {
    /// Follow the attack decision without starting another shot or resetting its timer.
    pub(crate) fn retarget_auto_shot(&mut self, target: Entity) -> bool {
        if self.spell_id != AUTO_SHOT {
            return false;
        }
        self.target = target;
        true
    }
}

/// After a successful cast: an autorepeat spell (re)starts with a full ranged timer.
pub(super) fn on_cast(
    world: &mut World,
    caster: Entity,
    spell: &crate::spell_info::SpellInfo,
    target: Option<Entity>,
) {
    let Some(target) = target else {
        return;
    };
    if attributes(spell, 2) & SPELL_ATTR2_AUTO_REPEAT == 0 {
        return;
    }
    let ready_in = crate::melee::attack_timer_duration(ranged_attack_secs(world, caster));
    world.entity_mut(caster).insert(AutoRepeat {
        spell_id: spell.id,
        target,
        ready_in,
    });
    if spell.id == AUTO_SHOT {
        super::scripts::roll_snakeskin_cobra(world, caster, target);
    }
}

/// Player::GetAttackBySlot (Player.cpp9666–9674): a ranged main hand sets the
/// RANGED_ATTACK base time, divided by haste (Unit::ApplyAttackTimePercentMod).
fn ranged_attack_secs(world: &World, caster: Entity) -> f32 {
    let attack_time = world
        .get::<CharacterStats>(caster)
        .map_or(BASE_ATTACK_SECS, |stats| {
            let base = if crate::auras::shapeshift::attack_timing(world, caster).is_some() {
                crate::auras::shapeshift::FORM_RANGED_ATTACK_SECS
            } else if stats.main_hand.kind == WeaponType::Ranged {
                stats.main_hand.speed
            } else {
                BASE_ATTACK_SECS
            };
            crate::melee::swing_time(base, stats.haste_pct)
        });
    let speed = world
        .get::<crate::auras::AuraStatMods>(caster)
        .map_or(1.0, |mods| mods.melee_ranged_attack_time_mult);
    attack_time * speed
}

/// CancelCast/CastStop also interrupt CURRENT_AUTOREPEAT_SPELL (Unit.cpp1191–1196,
/// 3251–3259).
pub(crate) fn cancel(world: &mut World, unit: Entity) {
    world.entity_mut(unit).remove::<AutoRepeat>();
}

pub(super) fn update(world: &mut World) {
    let dt = world.resource::<Time>().delta();
    let mut repeats = world.query::<(Entity, &mut AutoRepeat)>();
    let due: Vec<(Entity, u32, Entity)> = repeats
        .iter_mut(world)
        .filter_map(|(caster, mut repeat)| {
            repeat.ready_in = repeat.ready_in.saturating_sub(dt);
            repeat
                .ready_in
                .is_zero()
                .then_some((caster, repeat.spell_id, repeat.target))
        })
        .collect();
    for (caster, spell_id, target) in due {
        fire(world, caster, spell_id, target);
    }
}

fn fire(world: &mut World, caster: Entity, spell_id: u32, target: Entity) {
    let spell = world.resource::<SpellStore>().get(spell_id).cloned();
    let (Some(spell), true) = (spell, world.get_entity(target).is_ok()) else {
        cancel(world, caster);
        return;
    };
    // A successful shot re-enters `on_cast`, which resets the timer.
    if let Err(failure) = execute_cast(world, caster, &spell, Some(target)) {
        debug!(?caster, spell_id, ?failure, "Autorepeat shot skipped");
        if spell_id != AUTO_SHOT {
            cancel(world, caster);
        }
    }
}
