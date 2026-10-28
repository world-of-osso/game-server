//! Brewmaster Stagger defers damage into the native rolling periodic aura.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health, Level, UnitLevel, UnitStats};
use std::cell::Cell;

use super::{ActiveAuras, has_aura};
use crate::class_progression::TraitLoadout;
use crate::level_scaling::ArmorConstants;
use crate::spell_info::SpellStore;
use crate::stats::CharacterStats;

const STAGGER: u32 = 115069;
const STAGGER_DAMAGE: u32 = 124255;
const SHUFFLE: u32 = 215479;

pub(crate) struct Snapshot {
    rating: f32,
    armor_constant: f32,
    magic_fraction: f32,
    health: Health,
    current_pool: Cell<f32>,
    ox_threshold: Option<f32>,
    ox_available: Cell<bool>,
}

fn selected(world: &World, actor: Entity, spell: u32) -> bool {
    world.get::<TraitLoadout>(actor).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == spell)
    })
}

fn effect(store: &SpellStore, spell: u32, index: u8) -> f32 {
    store
        .get(spell)
        .expect("native Stagger spell")
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("native Stagger effect")
        .base_points
}

/// Snapshot the actual target before its incoming damage is resolved.
pub(crate) fn snapshot(world: &mut World, target: Entity) -> Option<Snapshot> {
    if !world
        .get::<ActiveSpec>(target)
        .is_some_and(|spec| spec.0 == 268)
        || !selected(world, target, STAGGER)
    {
        return None;
    }
    settle_due_ticks(world, target);
    let health = *world.get::<Health>(target).expect("Stagger target health");
    let agility = world
        .get::<CharacterStats>(target)
        .map(|stats| stats.primary.agility)
        .or_else(|| world.get::<UnitStats>(target).map(|stats| stats.agility))
        .expect("Stagger target agility");
    let level = world
        .get::<Level>(target)
        .map(|level| level.0)
        .or_else(|| world.get::<UnitLevel>(target).map(|level| level.0))
        .expect("Stagger target level");
    // Pinned SimC expected_stat.inc has no MYTHIC (16) rows: its product is 1.
    let armor_constant = world
        .resource::<ArmorConstants>()
        .at(level)
        .expect("Stagger armor constant");
    let store = world.resource::<SpellStore>();
    let mut rating = agility * effect(store, STAGGER, 0) / 100.0;
    if has_aura(world, target, SHUFFLE) {
        rating *= 1.0 + effect(store, SHUFFLE, 0) / 100.0;
    }
    if selected(world, target, 322960) && has_aura(world, target, 120954) {
        rating *= 1.0 + effect(store, 120954, 5) / 100.0;
    }
    if selected(world, target, 1241136) {
        // sc_monk.cpp6226–6228: Zen State's 1.3 multiplier is not in spell data.
        rating *= 1.0 + (1.0 - health.current / health.max).max(0.0) * 1.3;
    }
    let ox_threshold = (selected(world, target, 455068) && has_aura(world, target, 455071))
        .then(|| effect(store, 455068, 0) / 100.0);
    let current_pool = pool_size(world, target);
    Some(Snapshot {
        rating,
        armor_constant,
        magic_fraction: effect(store, STAGGER, 4) / 100.0,
        health,
        current_pool: Cell::new(current_pool),
        ox_threshold,
        ox_available: Cell::new(true),
    })
}

impl Snapshot {
    /// sc_monk.cpp6208–6239 and sc_stagger.hpp430–475.
    pub(crate) fn deferred(&self, amount: f32, school: u32) -> (f32, bool) {
        let ox = self.ox_available.get()
            && self
                .ox_threshold
                .is_some_and(|threshold| amount > self.health.current * threshold);
        let rating = self.rating * if ox { 1.4 } else { 1.0 };
        let mut fraction = rating / (rating + self.armor_constant);
        if school != 0 && school != 1 {
            fraction *= self.magic_fraction;
        }
        // sc_monk.cpp6195–6201: the last Stagger level caps the pool at 10× health.
        let available = (self.health.max * 10.0 - self.current_pool.get()).max(0.0);
        let deferred = (amount * fraction.min(0.99)).min(available);
        self.current_pool.set(self.current_pool.get() + deferred);
        if ox {
            self.ox_available.set(false);
        }
        (deferred, ox)
    }
}

/// Actual unpaid Stagger damage, using the scheduler's remaining ticks.
pub fn pool_size(world: &World, target: Entity) -> f32 {
    world
        .get::<ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == STAGGER_DAMAGE && aura.caster == target)
        })
        .map_or(0.0, |aura| {
            let ticks = super::periodic::count_remaining_ticks(aura).expect("finite Stagger aura");
            aura.effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("Stagger periodic effect")
                .amount
                * ticks as f32
        })
}

fn settle_due_ticks(world: &mut World, target: Entity) {
    let instance = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == STAGGER_DAMAGE && aura.caster == target)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        let now = crate::spell_cast::now(world);
        super::periodic::run_ticks_until(world, target, instance, now);
    }
}

/// Deposit deferred damage without restarting the existing periodic phase.
pub(crate) fn deposit(world: &mut World, target: Entity, amount: f32, ox: bool) {
    if ox {
        let instance = world.get::<ActiveAuras>(target).and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 455071 && aura.caster == target)
                .map(|aura| aura.instance_id)
        });
        if let Some(instance) = instance {
            super::spend_stack(world, target, instance);
        }
    }
    if amount <= 0.0 {
        return;
    }
    let store = world.resource::<SpellStore>();
    let spell = store
        .get(STAGGER_DAMAGE)
        .expect("native Stagger damage")
        .clone();
    let duration = store
        .get(124273)
        .and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.duration)
        .expect("native heavy Stagger duration")
        .duration_ms;
    let extra = if selected(world, target, 280515) {
        effect(store, 280515, 0) * 100.0
    } else {
        0.0
    };
    let duration = (duration as f32 + extra) as u32;
    let application = super::AuraApplication {
        caster: target,
        target,
        spell_id: STAGGER_DAMAGE,
        effect_mask: 1,
        source: super::AuraSource::Cast,
    };
    super::apply_rolling_periodic_deposit(world, &application, &spell, duration, amount);
}

/// Clear the source-specified fraction or minimum from an active Stagger pool.
pub fn purify(world: &mut World, target: Entity) -> f32 {
    settle_due_ticks(world, target);
    let pool = pool_size(world, target);
    if pool <= 0.0 {
        return 0.0;
    }
    let max_health = world
        .get::<Health>(target)
        .expect("Stagger target health")
        .max;
    let percent = crate::spell_cast::scripts::script_effect_value(world, target, 119582, 0);
    let minimum = crate::spell_cast::scripts::script_effect_value(world, target, 119582, 1);
    let clear = (pool * percent / 100.0)
        .max(max_health * minimum / 100.0)
        .min(pool);
    if clear >= pool {
        super::remove_auras(world, target, super::RemoveReason::Consumed, |aura| {
            aura.spell_id == STAGGER_DAMAGE && aura.caster == target
        });
    } else {
        let mut auras = world
            .get_mut::<ActiveAuras>(target)
            .expect("active Stagger auras");
        let aura = auras
            .auras
            .iter_mut()
            .find(|aura| aura.spell_id == STAGGER_DAMAGE && aura.caster == target)
            .expect("active Stagger aura");
        let ticks = super::periodic::count_remaining_ticks(aura).expect("finite Stagger aura");
        let effect = aura
            .effects
            .iter_mut()
            .find(|effect| effect.index == 0)
            .expect("Stagger periodic effect");
        effect.amount = (pool - clear) / ticks as f32;
        auras.dirty = true;
    }
    clear
}
