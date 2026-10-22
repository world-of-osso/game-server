//! Native70448 root470057 is Dummy0, with damage moved to1259101.
//! TC a352b1fa spell_shaman.cpp3232–3263:500ms Flame Shock after an actual hit.
//! Current damage child/guaranteed crit: SimC76f8118 sc_shaman.cpp10086–10117.
use bevy::prelude::*;

use crate::spell_cast::effects::EffectCtx;
use crate::spell_info::SpellInfo;

pub(super) const ROOT: u32 = 470057;
pub(super) const DAMAGE: u32 = 1259101;
const FLAME_SHOCK: u32 = 188389;
const SHOCK_DELAY_MS: u32 = 500;

pub(super) fn cast_damage(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    crate::spell_cast::cast_triggered(world, caster, DAMAGE, Some(target), depth + 1);
}

pub(super) fn schedule_flame_shock(world: &mut World, ctx: &EffectCtx, target: Entity) {
    crate::spell_cast::queued_triggers::schedule_full_trigger(
        world,
        ctx,
        target,
        FLAME_SHOCK,
        SHOCK_DELAY_MS,
    );
}

pub(super) fn limit_targets(
    _world: &World,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    let extras = spell
        .effects
        .iter()
        .find(|effect| effect.index == 3)
        .expect("native Voltaic Blaze retains its extra-target count")
        .base_points;
    if let Some(primary) = explicit.and_then(|target| units.iter().position(|unit| *unit == target))
    {
        units.swap(0, primary);
    }
    units.truncate(1 + extras.max(0.0) as usize);
}
