//! One-hit absorbs tied to a particular damage source.
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct SourceAbsorbs(Vec<Entry>);

struct Entry {
    source: Entity,
    spell: u32,
    amount: f32,
    expires_at: f64,
}

pub fn grant(
    world: &mut World,
    target: Entity,
    source: Entity,
    spell: u32,
    amount: f32,
    duration_ms: u32,
) {
    let expires_at = super::aura_now(world) + f64::from(duration_ms) / 1000.0;
    let mut entity = world.entity_mut(target);
    let mut absorbs = entity.get_mut::<SourceAbsorbs>();
    if let Some(absorbs) = absorbs.as_mut() {
        absorbs
            .0
            .retain(|entry| entry.source != source || entry.spell != spell);
        absorbs.0.push(Entry {
            source,
            spell,
            amount,
            expires_at,
        });
    } else {
        entity.insert(SourceAbsorbs(vec![Entry {
            source,
            spell,
            amount,
            expires_at,
        }]));
    }
}

pub fn consume(
    absorbs: Option<&mut SourceAbsorbs>,
    source: Entity,
    now: f64,
    damage: f32,
) -> Option<(u32, f32)> {
    if damage <= 0.0 {
        return None;
    }
    let absorbs = absorbs?;
    absorbs.0.retain(|entry| entry.expires_at > now);
    let index = absorbs.0.iter().position(|entry| entry.source == source)?;
    let entry = absorbs.0.remove(index);
    Some((entry.spell, damage.min(entry.amount)))
}

pub fn absorb(world: &mut World, target: Entity, source: Entity, damage: f32) -> (f32, f32) {
    let now = super::aura_now(world);
    let result = consume(
        world.get_mut::<SourceAbsorbs>(target).as_deref_mut(),
        source,
        now,
        damage,
    );
    if let Some((spell, absorbed)) = result {
        finish(world, target, source, spell, absorbed);
        (damage - absorbed, absorbed)
    } else {
        (damage, 0.0)
    }
}

pub fn finish(world: &mut World, target: Entity, source: Entity, spell: u32, amount: f32) {
    super::remove_auras(world, source, super::RemoveReason::Consumed, |aura| {
        aura.spell_id == spell && aura.caster == target
    });
    #[cfg(any(test, feature = "test-support"))]
    crate::test_support::healer_intake::record_absorb(world, target, target, spell, amount);
    crate::damage_meter::record_absorb(world, target, amount);
}
