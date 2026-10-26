//! SimC76f8118 sc_druid.cpp9191–9674: native Cat Convoke child actions.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{ActiveAuras, AuraInstance, RemoveReason};
use combat::random::Rng;
use combat::spell_cast::script_registry::{ContextHitHook, SpellScriptHook};
use combat::spell_cast::{self, effects::EffectCtx, targets};
use combat::spell_info::{CastPolicyContext, SpellInfo};
use shared::components::{ActiveSpec, Health, PowerType};

#[path = "feral_convoke/deck.rs"]
mod deck;
use deck::{Action, Card, ExceptionalDeck};

const CONVOKE: u32 = 391528;
const CAT: u32 = 768;
const GUIDANCE: u32 = 391548;
const BITE: u32 = 22568;
const RAKE: u32 = 1822;
const RAKE_BLEED: u32 = 155722;
const SHRED: u32 = 5221;
const MOONFIRE: u32 = 155625;
const WRATH: u32 = 5176;
const REGROWTH: u32 = 8936;
const REJUVENATION: u32 = 774;
const FERAL_FRENZY: u32 = 274837;
const FRANTIC_FRENZY: u32 = 1243807;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::ContextHit(hit),
];

#[derive(Component)]
struct Channel {
    primary: Entity,
    cards: Vec<Card>,
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        CONVOKE,
        AuraScript {
            on_periodic: Some(tick),
            on_remove: Some(clear_channel),
            ..Default::default()
        },
    );
}

fn hit(spell: u32) -> Option<ContextHitHook> {
    (spell == CONVOKE).then_some(begin)
}

fn selected(world: &World, caster: Entity, spell: u32) -> bool {
    world
        .get::<combat::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == spell)
        })
}

fn begin(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if target != ctx.caster {
        return;
    }
    let feral = world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 103);
    if !feral || !combat::auras::has_aura(world, ctx.caster, CAT) {
        return;
    }
    let Some(primary) = ctx.spell.cast_primary_target else {
        warn!(caster=?ctx.caster, "native Cat Convoke lacks its retained primary");
        return;
    };
    let channel = roll_channel(world, ctx.caster, ctx.spell, primary);
    world.entity_mut(ctx.caster).insert(channel);
}

fn roll_channel(world: &mut World, caster: Entity, spell: &SpellInfo, primary: Entity) -> Channel {
    let ticks = read_tick_budget(world, caster, spell);
    let guidance = selected(world, caster, GUIDANCE);
    let mut exceptional = world
        .entity_mut(caster)
        .take::<ExceptionalDeck>()
        .unwrap_or_default();
    let success = exceptional.draw(guidance);
    world.entity_mut(caster).insert(exceptional);
    let cards = deck::roll_cards(ticks, guidance, success);
    Channel { primary, cards }
}

fn read_tick_budget(world: &World, caster: Entity, spell: &SpellInfo) -> usize {
    let aura = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == CONVOKE && aura.caster == caster)
        })
        .expect("ordinary Convoke applies its actual native channel aura before hit scripts");
    let period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Convoke retains its periodic driver")
        .aura_period_ms;
    assert!(period > 0, "native Convoke requires a positive period");
    (aura.duration_ms / period) as usize
}

fn tick(world: &mut World, unit: Entity, _: &AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    let Some(mut channel) = world.entity_mut(unit).take::<Channel>() else {
        return;
    };
    let index = combat::random::thread_rng().gen_range(0..channel.cards.len());
    let card = channel.cards.swap_remove(index);
    if combat::auras::has_aura(world, unit, CAT) {
        let action = deck::roll_action(card);
        if let Some((spell, target)) = roll_child(world, unit, channel.primary, action) {
            launch_child(world, unit, spell, target);
        }
    }
    if !channel.cards.is_empty() && combat::auras::has_aura(world, unit, CONVOKE) {
        world.entity_mut(unit).insert(channel);
    }
}

fn clear_channel(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    world.entity_mut(unit).remove::<Channel>();
}

fn read_enemies(world: &World, caster: Entity, primary: Entity) -> Vec<Entity> {
    // Native retained primary plus actual encounter opponents; no synthetic range,
    // enemy count, or replacement recipient enters the distance-disabled encounter.
    let mut enemies: Vec<_> = std::iter::once(primary)
        .chain(combat::combat_refs::active_opponents(world, caster))
        .filter(|target| {
            // sc_druid.cpp9643 uses eligible enemies, including neutral attackable
            // native31146/faction7. Hostile reaction alone excludes those targets.
            targets::relation_matches(world, caster, *target, targets::Relation::Enemy)
                && world
                    .get::<Health>(*target)
                    .is_some_and(|health| health.current > 0.0)
        })
        .collect();
    enemies.sort_unstable_by_key(|target| target.to_bits());
    enemies.dedup();
    enemies
}

fn roll_child(
    world: &World,
    caster: Entity,
    primary: Entity,
    action: Action,
) -> Option<(u32, Entity)> {
    let enemies = read_enemies(world, caster, primary);
    if enemies.is_empty() {
        return None;
    }
    // sc_druid.cpp9502–9522: Main/exceptional keep the actual primary;
    // ordinary/off-spec cards select an actual encounter enemy first.
    let target = if matches!(action, Action::Bite | Action::Frenzy) {
        primary
    } else {
        enemies[combat::random::thread_rng().gen_range(0..enemies.len())]
    };
    let spell = roll_spell_id(world, caster, target, &action);
    let target = if matches!(action, Action::Heal) {
        caster
    } else {
        target
    };
    Some((spell, target))
}

fn owns_dot(world: &World, caster: Entity, target: Entity, spell: u32) -> bool {
    let now = spell_cast::now(world);
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.caster == caster
                && aura.spell_id == spell
                && aura.expires_at.is_none_or(|expiry| expiry > now)
        })
    })
}

fn roll_spell_id(world: &World, caster: Entity, target: Entity, action: &Action) -> u32 {
    match action {
        Action::Bite => BITE,
        Action::Frenzy if selected(world, caster, FRANTIC_FRENZY) => FRANTIC_FRENZY,
        Action::Frenzy => FERAL_FRENZY,
        Action::Shred => SHRED,
        // sc_druid.cpp9515–9522: already-ticking dots replace these cards,
        // rather than wasting native casts refreshing the same target's dot.
        Action::Rake if owns_dot(world, caster, target, RAKE_BLEED) => SHRED,
        Action::Rake => RAKE,
        Action::Moonfire if owns_dot(world, caster, target, MOONFIRE) => WRATH,
        Action::Moonfire => MOONFIRE,
        Action::Wrath => WRATH,
        Action::Heal if combat::random::thread_rng().gen_bool(0.5) => REGROWTH,
        Action::Heal => REJUVENATION,
    }
}

fn launch_child(world: &mut World, caster: Entity, id: u32, target: Entity) {
    let Some(mut spell) = spell_cast::load_triggered_spell(world, id, 1) else {
        return;
    };
    if id == BITE {
        // sc_druid.cpp4358–4401/9305: a free secondary Bite has native maximum
        // effective CP/Energy. Retain values; never grant or spend a real pool.
        let (_, maximum) = spell_cast::powers::power_value(world, caster, PowerType::ComboPoints)
            .expect("native Cat Convoke requires its combo-point pool");
        spell.cast_policy_context = Some(Box::new(CastPolicyContext {
            resources: vec![(PowerType::ComboPoints, maximum)],
            free_spender: true,
            targets: vec![target],
        }));
    }
    spell_cast::launch_triggered_spell(world, caster, &spell, Some(target), 1);
}
