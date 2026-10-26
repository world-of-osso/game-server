//! Source-qualified Unseen attacks retain the actual paid Bite snapshot.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::random::Rng;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{ContextHitHook, SpellScriptHook},
    targets,
};
use shared::components::PowerType;

const BITE: u32 = 22568;
const UNSEEN: u32 = 1263657;
const CRAVING_TALENT: u32 = 1263658;
const SLASH: u32 = 1263890;
const SWIPE: u32 = 1263908;
const BLEED: u32 = 1271863;
const CRAVING: u32 = 1263939;
const SWIPE_THRESHOLD: usize = 3;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceContext(&[BITE]),
    SpellScriptHook::ContextHit(hit),
    SpellScriptHook::LaunchDelay(travel_delay),
    SpellScriptHook::ConsumedEffectValues(&[(UNSEEN, 0), (UNSEEN, 1)]),
];

fn hit(spell: u32) -> Option<ContextHitHook> {
    match spell {
        BITE => Some(roll_unseen),
        SLASH | SWIPE => Some(finish_unseen),
        _ => None,
    }
}

fn effect(world: &World, caster: Entity, spell: u32, index: u8) -> Option<f32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)?
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .map(|effect| effect.amount)
}

fn combo_points(ctx: &EffectCtx<'_>) -> Option<f32> {
    ctx.spell
        .cast_policy_context
        .as_ref()?
        .resources
        .iter()
        .find(|(power, _)| *power == PowerType::ComboPoints)
        .map(|(_, points)| *points)
}

// SimC76f8118 sc_druid.cpp4321–4327: native talent chance per frozen CP,
// not a new resource read after the Bite has paid its actual costs.
fn roll_unseen(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if ctx.spell.cast_primary_target != Some(target) {
        return;
    }
    let Some(chance) = effect(world, ctx.caster, UNSEEN, 0) else {
        return;
    };
    let Some(points) = combo_points(ctx) else {
        warn!(caster=?ctx.caster, "Unseen Bite lacks its native launch resource snapshot");
        return;
    };
    if combat::random::thread_rng().gen_range(0.0..100.0) >= chance * points {
        return;
    }
    let energy =
        spell_cast::read_ferocious_bite_extra_energy_modifier(world, ctx.caster, ctx.spell)
            .expect("ordinary native Bite retains its extra-Energy snapshot");
    launch_unseen(world, ctx, points, energy);
}

// SimC76f8118 sc_druid.cpp2776–2804: pick an actual enemy in the sourced
// native talent range, then select Slash versus Swipe from local occupancy.
fn choose_enemy(world: &mut World, caster: Entity) -> Option<Entity> {
    let range = effect(world, caster, UNSEEN, 1).expect("native Unseen range");
    let center = targets::position(world, caster).expect("native Bite caster position");
    let enemies = targets::area_units(world, caster, center, range, targets::Relation::Enemy, 0);
    if enemies.is_empty() {
        return None;
    }
    Some(enemies[combat::random::thread_rng().gen_range(0..enemies.len())])
}

fn read_attack(
    world: &mut World,
    caster: Entity,
    target: Entity,
    depth: u8,
) -> Option<combat::spell_info::SpellInfo> {
    let target_center = targets::position(world, target).expect("native selected Unseen target");
    let swipe = spell_cast::load_triggered_spell(world, SWIPE, depth)?;
    let radius = targets::radius(&swipe.effects[0], 0);
    let local = targets::area_units(
        world,
        caster,
        target_center,
        radius,
        targets::Relation::Enemy,
        0,
    );
    if local.len() > SWIPE_THRESHOLD {
        Some(swipe)
    } else {
        spell_cast::load_triggered_spell(world, SLASH, depth)
    }
}

fn launch_unseen(world: &mut World, ctx: &EffectCtx<'_>, points: f32, energy: f32) {
    let Some(target) = choose_enemy(world, ctx.caster) else {
        return;
    };
    let depth = ctx.depth + 1;
    let Some(mut spell) = read_attack(world, ctx.caster, target, depth) else {
        return;
    };
    let (_, maximum) = spell_cast::powers::power_value(world, ctx.caster, PowerType::ComboPoints)
        .expect("native Unseen caster has a combo-point pool");
    // sc_druid.cpp4919–4929: CP fraction times extra-Energy fraction,
    // not Bite's1+extra*Saber effectiveness. Coefficients stay native.
    spell.cast_damage_multiplier = Some(points / maximum * energy);
    spell.cast_policy_context = ctx.spell.cast_policy_context.clone();
    spell_cast::launch_triggered_spell(world, ctx.caster, &spell, Some(target), depth);
}

fn travel_delay(spell: u32) -> Option<f32> {
    // SimC76f8118 sc_druid.cpp42/4902–4905; actual native missile snapshot
    // retains the chosen recipient and frozen factors for this400–500ms flight.
    matches!(spell, SLASH | SWIPE).then(|| combat::random::thread_rng().gen_range(0.4..0.5))
}

fn finish_unseen(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if ctx.spell.id == SLASH {
        // SimC76f8118 sc_druid.cpp4970–4977 copies the same CP/Energy
        // effectiveness into the native Slash bleed after the delayed hit.
        let Some(mut bleed) = spell_cast::load_triggered_spell(world, BLEED, ctx.depth + 1) else {
            return;
        };
        bleed.cast_damage_multiplier = ctx.spell.cast_damage_multiplier;
        spell_cast::launch_triggered_spell(world, ctx.caster, &bleed, Some(target), ctx.depth + 1);
    }
    if ctx.spell.cast_primary_target == Some(target)
        && effect(world, ctx.caster, CRAVING_TALENT, 0).is_some()
    {
        apply_craving(world, ctx);
    }
}

fn apply_craving(world: &mut World, ctx: &EffectCtx<'_>) {
    // SimC76f8118 sc_druid.cpp4910–4918: one primary impact grants the
    // native buff with duration multiplied by the retained effective points.
    let points = combo_points(ctx).expect("Unseen impact retains its Bite resource snapshot");
    let Some(mut buff) = spell_cast::load_triggered_spell(world, CRAVING, ctx.depth + 1) else {
        return;
    };
    let duration = buff
        .misc
        .as_mut()
        .and_then(|misc| misc.duration.as_mut())
        .expect("native Craving duration");
    duration.duration_ms = (duration.duration_ms as f32 * points).round() as i32;
    spell_cast::launch_triggered_spell(world, ctx.caster, &buff, Some(ctx.caster), ctx.depth + 1);
}
