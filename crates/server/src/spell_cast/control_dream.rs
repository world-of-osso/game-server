//! Native434249: unused ready time, capped by s1 seconds, reduces the next cooldown.
//! IDs follow the named abilities in local Mainline12.1.0.69933 Spell/SpellName.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use crate::spell_info::SpellInfo;

pub(crate) const CONTROL_OF_THE_DREAM: u32 = 434_249;

#[derive(Component)]
struct DreamLearnedAt(f64);

pub(crate) fn start_ready_credit(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit || world.get::<DreamLearnedAt>(unit).is_some() {
        return;
    }
    let now = super::now(world);
    world.entity_mut(unit).insert(DreamLearnedAt(now));
}

pub(crate) fn stop_ready_credit(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    if aura.caster == unit {
        world.entity_mut(unit).remove::<DreamLearnedAt>();
    }
}

fn affects_spell(spell: &SpellInfo) -> bool {
    matches!(
        spell.id,
        132_158 | 33_891 | 205_636 | 194_223 | 383_410 | 395_022 | 102_560 | 394_013 | 391_528
    )
}

fn native_cap_seconds(world: &World, unit: Entity) -> Option<f64> {
    let aura = world
        .get::<ActiveAuras>(unit)?
        .iter()
        .find(|aura| aura.spell_id == CONTROL_OF_THE_DREAM && aura.caster == unit)?;
    let amount = aura.effects.iter().find(|effect| effect.index == 0)?.amount;
    Some(f64::from(amount))
}

fn fully_ready_at(history: &super::SpellHistory, spell: &SpellInfo, now: f64) -> Option<f64> {
    let category = spell.categories.as_ref().map_or(0, |data| data.category);
    let spell_ready = history
        .spell_ready_at
        .get(&spell.id)
        .copied()
        .unwrap_or(0.0);
    let category_ready = history
        .category_ready_at
        .get(&category)
        .copied()
        .unwrap_or(0.0);
    let mut ready = spell_ready.max(category_ready);
    if let Some(charges) = spell
        .categories
        .as_ref()
        .and_then(|data| history.charges.get(&data.charge_category))
    {
        let mut charges = *charges;
        charges.refresh(now);
        if charges.used > 0 {
            return None;
        }
        ready = ready.max(charges.fully_ready_at);
    }
    (ready <= now).then_some(ready)
}

pub(super) fn ready_credit_ms(world: &World, unit: Entity, spell: &SpellInfo, now: f64) -> u32 {
    if !affects_spell(spell) {
        return 0;
    }
    let Some(learned) = world.get::<DreamLearnedAt>(unit) else {
        return 0;
    };
    let Some(cap) = native_cap_seconds(world, unit) else {
        return 0;
    };
    let ready = world
        .get::<super::SpellHistory>(unit)
        .map_or(Some(learned.0), |history| {
            fully_ready_at(history, spell, now)
        });
    let Some(ready) = ready else {
        return 0;
    };
    let elapsed = (now - ready.max(learned.0)).max(0.0).min(cap);
    (elapsed * 1_000.0).round() as u32
}
