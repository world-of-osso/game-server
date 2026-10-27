//! Native Between the Eyes stack duration and earned Gravedigger consumption.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::PowerType;

const BTE: u32 = 315341;
const GRAVEDIGGER_ONE: u32 = 1265861;
const GRAVEDIGGER: u32 = 1265935;
const PALMED: u32 = 1265931;
const ENERGIZE: u32 = 1279356;

#[derive(Component)]
struct PendingGravedigger;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceOnlyContext(&[BTE]),
    SpellScriptHook::AsynchronousAuras(&[BTE]),
    SpellScriptHook::BeforeNativeOnCast(snapshot_gravedigger),
    SpellScriptHook::PreparePolicySpell(prepare_duration),
    SpellScriptHook::AfterResolvedHits(after_bte),
];

fn snapshot_gravedigger(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if spell.id != BTE || depth != 0 {
        return;
    }
    if auras::has_aura(world, caster, GRAVEDIGGER) {
        world.entity_mut(caster).insert(PendingGravedigger);
    } else {
        world.entity_mut(caster).remove::<PendingGravedigger>();
    }
}

fn prepare_duration(_: &World, _: Entity, spell: &mut SpellInfo) {
    if spell.id != BTE {
        return;
    }
    let points = spell
        .cast_policy_context
        .as_ref()
        .and_then(|context| {
            context
                .resources
                .iter()
                .find_map(|(power, amount)| (*power == PowerType::ComboPoints).then_some(*amount))
        })
        .expect("Between the Eyes retains its effective combo points");
    let duration = spell
        .misc
        .as_mut()
        .and_then(|misc| misc.duration.as_mut())
        .expect("native Between the Eyes duration");
    // SimC76f8118 sc_rogue.cpp3844–3845: base duration plus one native
    // duration increment per effective combo point, capped by native data.
    duration.duration_ms = (duration.duration_ms
        + (duration.per_resource_ms as f32 * points) as i32)
        .min(duration.max_duration_ms);
}

fn after_bte(world: &mut World, ctx: &EffectCtx) {
    if ctx.spell.id != BTE || ctx.depth != 0 {
        return;
    }
    let landed = spell_cast::resolving_damage(world, ctx)
        .is_some_and(|hits| hits.iter().any(|(_, amount)| *amount > 0.0));
    if !landed {
        return;
    }
    earn_extra_stack(world, ctx);
    consume_gravedigger(world, ctx.caster);
}

fn earn_extra_stack(world: &mut World, ctx: &EffectCtx) {
    if !auras::has_aura(world, ctx.caster, GRAVEDIGGER_ONE) {
        return;
    }
    let chance = spell_cast::scripts::script_effect_value(world, ctx.caster, GRAVEDIGGER_ONE, 0);
    if combat::random::thread_rng().gen_range(0.0..100.0) >= chance {
        return;
    }
    // sc_rogue.cpp3847–3850: a separate asynchronous stack uses the same duration.
    auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: ctx.caster,
            target: ctx.caster,
            spell_id: BTE,
            effect_mask: auras::aura_effect_mask(ctx.spell),
            source: AuraSource::Cast,
        },
        ctx.spell,
    );
}

fn consume_gravedigger(world: &mut World, caster: Entity) {
    if world
        .entity_mut(caster)
        .take::<PendingGravedigger>()
        .is_none()
    {
        return;
    }
    // sc_rogue.cpp3862–3872: the earned use returns native points, resets
    // Between the Eyes and consumes the native Palmed threshold.
    spell_cast::cast_triggered(world, caster, ENERGIZE, Some(caster), 1);
    let spell = world
        .resource::<SpellStore>()
        .get(BTE)
        .expect("native Between the Eyes")
        .clone();
    let now = spell_cast::now(world);
    let remaining = world
        .get::<spell_cast::SpellHistory>(caster)
        .map_or(0, |history| history.remaining_cooldown_ms(&spell, now));
    spell_cast::scripts::reduce_cooldown(world, caster, BTE, remaining as f64 / 1000.0);
    let threshold = world
        .resource::<SpellStore>()
        .get(PALMED)
        .expect("native Palmed Bullets")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Palmed threshold")
        .misc_values[1];
    if let Some(instance) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == PALMED && aura.caster == caster)
            .map(|aura| aura.instance_id)
    }) {
        for _ in 0..threshold {
            auras::spend_stack(world, caster, instance);
        }
    }
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == GRAVEDIGGER && aura.caster == caster
    });
}
