//! Heroic Might keeps pending Heroics separate from the active CS modifier aura.
use super::{combat, selected};
use bevy::prelude::*;
use combat::spell_info::{SpellInfo, SpellStore};

const TALENT: u32 = 1269307;
const HEROIC_MIGHT: u32 = 1292058;
const COLOSSUS_WOUND: u32 = 208086;

#[derive(Component, Default, Debug)]
struct PendingHeroicMight(u8);

pub(super) fn after_heroic(world: &mut World, caster: Entity, spell: &SpellInfo, _: bool) {
    if spell.id != 1269383 || !selected(world, caster, TALENT) {
        return;
    }
    // SimC76f8118 sc_warrior.cpp3571–3573: this is the accumulator,
    // not the active window (both SimC buff objects use native1292058).
    let maximum = world
        .resource::<SpellStore>()
        .get(HEROIC_MIGHT)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native Heroic Might stack metadata")
        .cumulative_aura;
    let maximum = u8::try_from(maximum).expect("native Heroic Might stack cap fits u8");
    assert!(maximum > 0);
    let mut actor = world.entity_mut(caster);
    let mut pending = actor.entry::<PendingHeroicMight>().or_default().into_mut();
    pending.0 = pending.0.saturating_add(1).min(maximum);
}

pub(super) fn before_smash(world: &mut World, caster: Entity, spell: &SpellInfo, _: u8) {
    if spell.id != 167105 || !selected(world, caster, TALENT) {
        return;
    }
    let pending = world
        .get::<PendingHeroicMight>(caster)
        .map_or(0, |value| value.0);
    if pending == 0 {
        return;
    }
    let duration = world
        .resource::<SpellStore>()
        .get(COLOSSUS_WOUND)
        .and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.duration.as_ref())
        .expect("native Colossus Smash wound duration")
        .duration_ms;
    // SimC3761–3764/1729–1730: activate only for the next CS window.
    // Native1292058's flat effect1/effect2 modifiers target label2443 on
    //208086, so both spell and white-swing vulnerabilities retain native values.
    // Activate before wound application; the initial CS strike has no2443 label.
    let values = combat::spell_cast::TriggeredSpellOverrides {
        aura_stacks: Some(pending),
        duration_ms: Some(duration),
        ..Default::default()
    };
    if !combat::spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        HEROIC_MIGHT,
        Some(caster),
        1,
        values,
    ) {
        warn!(?caster, "Native Heroic Might active window refused");
        return;
    }
    assert!(combat::auras::set_self_cast_stacks(
        world,
        caster,
        HEROIC_MIGHT,
        pending
    ));
    world
        .get_mut::<PendingHeroicMight>(caster)
        .expect("pending Heroic Might")
        .0 = 0;
}
