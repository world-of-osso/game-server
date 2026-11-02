//! Native 70448 Void Apparitions 1264107; SimC76f8118 shadow1874–1876/2878–2920.
use super::{combat, shadow_apparitions, shadow_cthun, shadow_yogg};
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource, RemoveReason};
use combat::random::seq::SliceRandom;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, summon::SummonedCreature,
};

const TALENT: u32 = 1264107;
const APEX: u32 = 1264096;
const SHADOWFIEND: u32 = 1280172;
const YSHAARJ_BUFF: u32 = 373316;
const HORRIFIC: u32 = 1243105;
const VISION: u32 = 1243106;
const HORRIFIC_BUFF: u32 = 1243113;
const VISION_BUFF: u32 = 1243114;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SummonRemoved(remove_fiend_buff),
    SpellScriptHook::MeleeResidual(shadowfiend_insanity),
    SpellScriptHook::ConsumedEffectValues(&[(TALENT, 0)]),
];

#[derive(Clone, Copy)]
enum Idol {
    Yshaarj,
    Horrific,
    Vision,
    Yogg,
    Cthun,
}

#[derive(Component, Default)]
struct IdolDeck(Vec<Idol>);

fn draw_idol(world: &mut World, caster: Entity) -> Idol {
    let mut deck = world
        .entity_mut(caster)
        .take::<IdolDeck>()
        .unwrap_or_default();
    if deck.0.is_empty() {
        // INFERRED: SimC76f8118 shadow2315–2320 shuffled 3/4/1/2/7 deck;
        // native1264107 specifies the outcomes but not their distribution.
        deck.0.extend([Idol::Yshaarj; 3]);
        deck.0.extend([Idol::Horrific; 4]);
        deck.0.push(Idol::Vision);
        deck.0.extend([Idol::Yogg; 2]);
        deck.0.extend([Idol::Cthun; 7]);
        deck.0.shuffle(&mut combat::random::thread_rng());
    }
    let idol = deck.0.pop().expect("refilled Void Apparitions deck");
    world.entity_mut(caster).insert(deck);
    idol
}

pub(super) fn trigger(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if !auras::has_aura(world, ctx.caster, TALENT) {
        return;
    }
    let chance = spell_cast::scripts::script_effect_value(world, ctx.caster, TALENT, 0);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    match draw_idol(world, ctx.caster) {
        Idol::Cthun => {
            shadow_cthun::spawn_idol(world, ctx, target);
            return;
        }
        Idol::Yshaarj => {
            if !spell_cast::try_cast_triggered(
                world,
                ctx.caster,
                SHADOWFIEND,
                Some(target),
                ctx.depth + 1,
            ) {
                warn!(caster = ?ctx.caster, "Void Apparitions Shadowfiend summon rejected");
                return;
            }
            spell_cast::cast_triggered(
                world,
                ctx.caster,
                YSHAARJ_BUFF,
                Some(ctx.caster),
                ctx.depth + 1,
            );
        }
        Idol::Horrific => cast_vision(world, ctx, target, HORRIFIC, HORRIFIC_BUFF),
        Idol::Vision => cast_vision(world, ctx, target, VISION, VISION_BUFF),
        Idol::Yogg => shadow_yogg::spawn_thing(world, ctx),
    }
    if auras::has_aura(world, ctx.caster, APEX) {
        shadow_apparitions::spawn_apparitions(world, ctx, target);
    }
}

fn cast_vision(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, damage: u32, buff: u32) {
    spell_cast::cast_triggered(world, ctx.caster, damage, Some(target), ctx.depth + 1);
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: ctx.caster,
            target: ctx.caster,
            spell_id: buff,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn shadowfiend_insanity(world: &mut World, actor: Entity, _: Entity, _: f32) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    let owner = summon.owner;
    if summon.spell_id != SHADOWFIEND
        || !world
            .get::<combat::class_progression::KnownSpells>(owner)
            .is_some_and(|known| known.contains(34433))
        || !world
            .get::<shared::components::ActiveSpec>(owner)
            .is_some_and(|spec| spec.0 == 258)
    {
        return;
    }
    spell_cast::cast_triggered(world, owner, 262485, Some(owner), 1);
}

fn remove_fiend_buff(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    if !matches!(summon.spell_id, SHADOWFIEND | 123040 | 200174 | 451235) {
        return;
    }
    let owner = summon.owner;
    let now = spell_cast::now(world);
    let other = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .any(|(entity, pet)| {
            entity != actor
                && pet.owner == owner
                && matches!(pet.spell_id, SHADOWFIEND | 123040 | 200174 | 451235)
                && pet.expires_at.is_none_or(|expiry| expiry > now)
                && spell_cast::targets::is_alive(world, entity)
        });
    if !other {
        auras::remove_auras(world, owner, RemoveReason::Expired, |aura| {
            aura.spell_id == YSHAARJ_BUFF && aura.caster == owner
        });
    }
}
