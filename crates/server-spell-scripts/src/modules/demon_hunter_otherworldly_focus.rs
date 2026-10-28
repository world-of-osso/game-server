//! Annihilator Otherworldly Focus scales qualifying area damage by recipient count.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, targets::Relation,
};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const FOCUS: u32 = 1253817;
pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ActualDamageMultiplier(actual)];
fn actual(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) -> f32 {
    multiplier(world, ctx.caster, target, ctx.spell)
}
fn multiplier(world: &mut World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if !world
        .get::<combat::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == FOCUS)
        })
    {
        return 1.0;
    }
    let spec = world.get::<ActiveSpec>(caster).map(|spec| spec.0);
    let value_index = match (spec, spell.id) {
        (Some(581), 247455 | 1256306 | 1256617 | 1264130) => 0,
        (Some(1480), 1221162 | 1256305 | 1256619 | 1264129) => 2,
        _ => return 1.0,
    };
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native area damage effect");
    let radius = effect.radius[1]
        .or(effect.radius[0])
        .expect("native area damage radius")
        .radius_max;
    let center = if effect.implicit_targets[0] == 87 {
        Vec3::from_array(
            spell
                .cast_destination
                .expect("native area damage destination"),
        )
    } else {
        spell_cast::targets::position(world, caster).expect("native area damage caster position")
    };
    let targets =
        spell_cast::targets::area_units(world, caster, center, radius, Relation::Enemy, 0).len();
    let base = spell_cast::scripts::script_effect_value(world, caster, FOCUS, value_index);
    let reduction = spell_cast::scripts::script_effect_value(world, caster, FOCUS, 1);
    // SimC76f8118 sc_demon_hunter.cpp3353–3388 uses the actual area count,
    // up to ten additional recipients, with the native spec-specific percentage.
    1.0 + (base - reduction * targets.saturating_sub(1).min(10) as f32).max(0.0) / 100.0
}
