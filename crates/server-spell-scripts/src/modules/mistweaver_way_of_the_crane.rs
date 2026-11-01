//! 70448 Way of the Crane: Tiger Palm strikes twice and Spinning Crane Kick hits heal an ally.
use super::combat;
use bevy::prelude::*;
use combat::random::{self as rand, seq::SliceRandom};
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::{
    self,
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    scripts::script_effect_value,
    targets::{self, Relation},
};
use combat::spell_info::SpellStore;
use shared::components::{Health, UnitIsPet};

const TALENT: u32 = 388779;
const TIGER_PALM: u32 = 100780;
const CRANE_KICK_DAMAGE: u32 = 107270;
const HEAL: u32 = 389325;
const CRITICAL_HEAL: u32 = 389328;
/// 389325 has no area; Ancient Teachings' heal names the "nearby" radius Mistweaver uses.
const NEARBY_RADIUS_SPELL: u32 = 388024;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(on_damage)];

fn on_damage(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, crit: bool, damage: f32) {
    if !combat::auras::has_aura(world, ctx.caster, TALENT) {
        return;
    }
    match ctx.spell.id {
        // WCL TQXmJ6YVr71Pdycw fight 8 actor 837: 172 Tiger Palm hits from 86 casts.
        TIGER_PALM if ctx.depth == 0 => strike_again(world, ctx, target),
        CRANE_KICK_DAMAGE if damage > 0.0 => heal_from_kick(world, ctx, crit, damage),
        _ => {}
    }
}

fn strike_again(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let overrides = spell_cast::TriggeredSpellOverrides {
        effect_mask: Some(1 << 0),
        ..Default::default()
    };
    if !spell_cast::cast_triggered_with_overrides(
        world,
        ctx.caster,
        TIGER_PALM,
        Some(target),
        ctx.depth + 1,
        overrides,
    ) {
        warn!(caster = ?ctx.caster, "Way of the Crane second Tiger Palm failed");
    }
}

/// WCL TQXmJ6YVr71Pdycw fight 8: each of 5220 kick hits heals one ally (389325, or
/// 389328 when the hit crits) for about 280% (388779 s1) of the hit's damage.
fn heal_from_kick(world: &mut World, ctx: &EffectCtx<'_>, crit: bool, damage: f32) {
    let caster = ctx.caster;
    let Some(ally) = select_ally(world, caster) else {
        return;
    };
    let spell = world
        .resource::<SpellStore>()
        .get(if crit { CRITICAL_HEAL } else { HEAL })
        .expect("Way of the Crane heal missing")
        .clone();
    let amount = damage * script_effect_value(world, caster, TALENT, 0) / 100.0;
    let heal = EffectCtx {
        caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::apply_heal(world, &heal, ally, amount, crit, false);
}

/// One living party member (no summons or pets) within the nearby radius, injured first,
/// in random order.
fn select_ally(world: &mut World, caster: Entity) -> Option<Entity> {
    let center = targets::position(world, caster)?;
    let radius = targets::radius(
        &world
            .resource::<SpellStore>()
            .get(NEARBY_RADIUS_SPELL)
            .expect("Ancient Teachings heal missing")
            .effects[0],
        1,
    );
    let mut allies = targets::area_units(world, caster, center, radius, Relation::Raid, 0);
    allies.retain(|&unit| {
        world.get::<SummonedCreature>(unit).is_none()
            && !world.get::<UnitIsPet>(unit).is_some_and(|pet| pet.0)
    });
    allies.shuffle(&mut rand::thread_rng());
    allies.sort_by_key(|&unit| {
        world
            .get::<Health>(unit)
            .is_none_or(|health| health.current >= health.max)
    });
    allies.first().copied()
}
