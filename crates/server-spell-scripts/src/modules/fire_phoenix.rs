//! SimC76f8118 sc_mage.cpp1097–1400: authentic Arcane Phoenix actions.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, AuraStatMods};
use combat::class_progression::KnownSpells;
use combat::random::Rng;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::targets::{Relation, relation_matches};
use combat::spell_cast::{self, effects::EffectCtx, queued_triggers};
use combat::spell_info::{SpellInfo, SpellStore};
use combat::stats::CharacterStats;
use shared::components::{Health, Npc, Position};
use std::collections::BTreeSet;

const COMBUSTION: u32 = 190319;
const INVOCATION: u32 = 448658;
const SUMMON: u32 = 448659;
const PHOENIX_ENTRY: u32 = 223453;
const CODEX: u32 = 449382;
const SPHERE: u32 = 448604;
const ASHES: u32 = 1260272;
const CASCADE_TALENT: u32 = 449293;
const CASCADE: u32 = 449314;
const METEORITE: u32 = 449559;
const NORMAL_TRAVEL: u32 = 449560;
const EXCEPTIONAL_TRAVEL: u32 = 456137;
const METEORITE_IMPACT: u32 = 456139;
const BUGGED_IMPACT: u32 = 449569;
const SINGLE_ACTIONS: &[u32] = &[450499, 450461];
const AREA_ACTIONS: &[u32] = &[450499, 450462, METEORITE];
const EXCEPTIONAL_ACTIONS: &[u32] = &[453326, 450421, METEORITE];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::FireAuraRegistration(register),
    SpellScriptHook::BeforeNativeOnCast(before_cast),
    SpellScriptHook::CriticalChance(critical_chance),
    SpellScriptHook::TargetDamageMultiplier(damage_multiplier),
];

#[derive(Component, Clone, Copy)]
struct SelectedPrimary(Option<Entity>);

#[derive(Component, Clone, Copy)]
struct Phoenix {
    owner: Entity,
    primary: Option<Entity>,
    spells_used: u32,
    exceptional_left: u8,
    meteorite_exceptional: bool,
    exceptional_meteorites: u32,
}

fn selected(world: &World, actor: Entity, talent: u32) -> bool {
    world
        .get::<KnownSpells>(actor)
        .is_some_and(|known| known.contains(talent))
}

fn before_cast(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if spell.id == COMBUSTION || (spell.id == SUMMON && depth == 0) {
        let primary = spell
            .cast_primary_target
            .filter(|target| relation_matches(world, caster, *target, Relation::Enemy));
        // A triggered self summon must not overwrite its admitted parent's
        // immutable enemy context. Ordinary448659 already carries the enemy.
        world.entity_mut(caster).insert(SelectedPrimary(primary));
    }
    let Some(phoenix) = world.get::<Phoenix>(caster).copied() else {
        return;
    };
    let exceptional = match spell.id {
        METEORITE => phoenix.meteorite_exceptional,
        453326 | 450421 => true,
        450499 | 450461 | 450462 => false,
        _ => return,
    };
    grant_ashes(world, phoenix.owner, exceptional);
    if spell.id == METEORITE {
        schedule_meteorite_impact(world, caster, spell, exceptional);
    }
}

pub(super) fn invoke(world: &mut World, owner: Entity, combustion: &AuraInstance) {
    if !selected(world, owner, INVOCATION) {
        return;
    }
    // sc_mage.cpp3531–3543: summon the real native creature for this actual
    // Combustion lifetime, rather than inventing a pet or copied damage stream.
    spell_cast::cast_triggered_with_overrides(
        world,
        owner,
        SUMMON,
        Some(owner),
        1,
        spell_cast::TriggeredSpellOverrides {
            duration_ms: Some(combustion.duration_ms as i32),
            ..Default::default()
        },
    );
}

pub(super) fn extend(world: &mut World, owner: Entity, milliseconds: u32) {
    auras::extend_owned_duration(world, owner, owner, &[SUMMON], milliseconds);
    let actors = owned_phoenixes(world, owner);
    for actor in actors {
        if let Some(mut summon) = world.get_mut::<SummonedCreature>(actor) {
            if let Some(expiry) = &mut summon.expires_at {
                *expiry += f64::from(milliseconds) / 1000.0;
            }
        }
    }
}

fn register(scripts: &mut auras::scripts::AuraScripts) {
    scripts.0.insert(
        SUMMON,
        auras::scripts::AuraScript {
            on_apply: Some(initialize),
            on_periodic: Some(pulse),
            on_remove: Some(remove),
            ..Default::default()
        },
    );
}

fn owned_phoenixes(world: &mut World, owner: Entity) -> Vec<Entity> {
    world
        .query::<(Entity, &SummonedCreature, &Npc)>()
        .iter(world)
        .filter(|(_, summon, npc)| {
            summon.owner == owner && summon.spell_id == SUMMON && npc.template_id == PHOENIX_ENTRY
        })
        .map(|(entity, _, _)| entity)
        .collect()
}

fn initialize(world: &mut World, owner: Entity, _aura: &AuraInstance) {
    let primary = world
        .get::<SelectedPrimary>(owner)
        .and_then(|primary| primary.0);
    let exceptional_left = consume_exceptional_budget(world, owner);
    for actor in owned_phoenixes(world, owner) {
        world.entity_mut(actor).insert(Phoenix {
            owner,
            primary,
            spells_used: 0,
            exceptional_left,
            meteorite_exceptional: false,
            exceptional_meteorites: 0,
        });
        // sc_mage.cpp1200–1257: the first spell is immediate; subsequent
        // spells use448659's native effect2 periodic timer, without haste scaling.
        cast_next(world, actor);
    }
}

fn consume_exceptional_budget(world: &mut World, owner: Entity) -> u8 {
    if !selected(world, owner, CODEX) {
        return 0;
    }
    let stacks = world
        .get::<ActiveAuras>(owner)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SPHERE && aura.caster == owner)
        })
        .map_or(0, |aura| aura.stacks);
    // sc_mage.cpp1249–1256: consume before the first immediate action.
    auras::remove_auras(world, owner, auras::RemoveReason::Consumed, |aura| {
        aura.spell_id == SPHERE && aura.caster == owner
    });
    stacks
}

fn pulse(world: &mut World, owner: Entity, _aura: &AuraInstance, index: u8) {
    if index != 1 {
        return;
    }
    for actor in owned_phoenixes(world, owner) {
        cast_next(world, actor);
    }
}

fn remove(world: &mut World, owner: Entity, _aura: &AuraInstance, _reason: auras::RemoveReason) {
    for actor in owned_phoenixes(world, owner) {
        spell_cast::summon::despawn_summon(world, actor);
    }
}

fn opponents(world: &World, state: Phoenix) -> Vec<Entity> {
    state
        .primary
        .into_iter()
        .chain(combat::combat_refs::active_opponents(world, state.owner))
        .filter(|target| {
            world
                .get::<Health>(*target)
                .is_some_and(|health| health.current > 0.0)
                && relation_matches(world, state.owner, *target, Relation::Enemy)
                && world.get::<combat::maps::InMap>(*target)
                    == world.get::<combat::maps::InMap>(state.owner)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn cast_next(world: &mut World, actor: Entity) {
    let Some(state) = world.get::<Phoenix>(actor).copied() else {
        return;
    };
    let targets = opponents(world, state);
    if targets.is_empty() {
        world.get_mut::<Phoenix>(actor).unwrap().spells_used += 1;
        return;
    }
    let (spell, exceptional) = roll_next_action(state, targets.len());
    let target = targets[combat::random::thread_rng().gen_range(0..targets.len())];
    let mut next = world.get_mut::<Phoenix>(actor).unwrap();
    next.spells_used += 1;
    if exceptional {
        next.exceptional_left -= 1;
    }
    next.meteorite_exceptional = exceptional;
    copy_owner_spell_stats(world, actor, state.owner);
    spell_cast::cast_triggered(world, actor, spell, Some(target), 1);
    if exceptional && spell == METEORITE {
        schedule_exceptional_meteorites(world, actor, target);
    }
}

fn roll_next_action(state: Phoenix, target_count: usize) -> (u32, bool) {
    let exceptional = state.spells_used % 2 == 1 && state.exceptional_left > 0;
    let actions = if exceptional {
        EXCEPTIONAL_ACTIONS
    } else if target_count > 1 {
        AREA_ACTIONS
    } else {
        SINGLE_ACTIONS
    };
    // sc_mage.cpp1208–1237: uniform authored action choice.
    let spell = actions[combat::random::thread_rng().gen_range(0..actions.len())];
    (spell, exceptional)
}

fn copy_owner_spell_stats(world: &mut World, actor: Entity, owner: Entity) {
    let owner_stats = *world
        .get::<CharacterStats>(owner)
        .expect("Phoenix owner has native class stats");
    // sc_mage.cpp1200 and pet.cpp211–226: live1.0 owner SP, crit and haste.
    // Native template health/armor remain untouched; there is no melee action.
    world.entity_mut(actor).insert(CharacterStats {
        spell_power: owner_stats.spell_power,
        crit_pct: owner_stats.crit_pct,
        haste_pct: owner_stats.haste_pct,
        versatility_pct: owner_stats.versatility_pct,
        ..Default::default()
    });
    let mut inherited = AuraStatMods::default();
    if let Some(mods) = world.get::<AuraStatMods>(owner) {
        // pet.cpp488–495: pets inherit their owner's critical damage multiplier.
        inherited.critical_damage_mods = mods.critical_damage_mods.clone();
    }
    world.entity_mut(actor).insert(inherited);
}

fn grant_ashes(world: &mut World, owner: Entity, exceptional: bool) {
    if !selected(world, owner, ASHES) || !selected(world, owner, CASCADE_TALENT) {
        return;
    }
    // sc_mage.cpp1115–1122: native effect1 stacks, plus effect2 for an
    // exceptional execution. Only a real Phoenix action earns these stacks.
    let mut stacks = script_effect_value(world, owner, ASHES, 0);
    if exceptional {
        stacks += script_effect_value(world, owner, ASHES, 1);
    }
    spell_cast::cast_triggered_with_overrides(
        world,
        owner,
        CASCADE,
        Some(owner),
        1,
        spell_cast::TriggeredSpellOverrides {
            aura_stacks: Some(stacks as u8),
            ..Default::default()
        },
    );
}

fn schedule_exceptional_meteorites(world: &mut World, actor: Entity, target: Entity) {
    let spell = world
        .resource::<SpellStore>()
        .get(METEORITE)
        .expect("native Phoenix Meteorite")
        .clone();
    let ctx = EffectCtx {
        caster: actor,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    // sc_mage.cpp1364–1370: the exceptional action executes four times,
    // including four distinct Ashes grants, staggered75ms.
    for ordinal in 1..4 {
        queued_triggers::schedule_full_trigger(world, &ctx, target, METEORITE, ordinal * 75);
    }
}

fn schedule_meteorite_impact(
    world: &mut World,
    actor: Entity,
    spell: &SpellInfo,
    exceptional: bool,
) {
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let Some(position) = world.get::<Position>(target).copied() else {
        return;
    };
    let delay_ms = meteorite_delay_ms(world, exceptional);
    let impact = advance_meteorite_impact(world, actor, exceptional);
    let ctx = EffectCtx {
        caster: actor,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    queued_triggers::schedule_destination_trigger(
        world,
        &ctx,
        queued_triggers::DestinationTrigger {
            target,
            spell_id: impact,
            delay_ms,
            destination: [position.x, position.y, position.z],
        },
    );
}

fn meteorite_delay_ms(world: &World, exceptional: bool) -> u32 {
    let travel = if exceptional {
        EXCEPTIONAL_TRAVEL
    } else {
        NORMAL_TRAVEL
    };
    let seconds = world
        .resource::<SpellStore>()
        .get(travel)
        .and_then(|spell| spell.misc.as_ref())
        .expect("native Phoenix Meteorite travel metadata")
        .speed;
    (seconds * 1000.0).round() as u32
}

fn advance_meteorite_impact(world: &mut World, actor: Entity, exceptional: bool) -> u32 {
    let mut state = world.get_mut::<Phoenix>(actor).unwrap();
    // sc_mage.cpp1377–1389: pinned bugs=1 sends the last three exceptional
    // impacts through449569, while normal/first exceptional use456139.
    let impact = if exceptional && state.exceptional_meteorites % 4 != 0 {
        BUGGED_IMPACT
    } else {
        METEORITE_IMPACT
    };
    if exceptional {
        state.exceptional_meteorites += 1;
    }
    impact
}

fn critical_chance(world: &World, actor: Entity, _target: Entity, _spell: u32) -> Option<f32> {
    let phoenix = world.get::<Phoenix>(actor)?;
    let chance = world.get::<CharacterStats>(phoenix.owner)?.crit_pct;
    // sc_mage.cpp1157–1166: Phoenix's actual damaging actions receive
    // native Combustion crit while the owner has that buff.
    let combustion = aura_effect(world, phoenix.owner, COMBUSTION, 0);
    Some(chance + combustion)
}

fn damage_multiplier(world: &World, actor: Entity, _target: Entity, _spell: &SpellInfo) -> f32 {
    let Some(phoenix) = world.get::<Phoenix>(actor) else {
        return 1.0;
    };
    // sc_mage.cpp1128–1134: these Phoenix actions use the owner's actual
    // Combustion and Sphere values. Native guardian531 supplies spec passives.
    let combustion = 1.0 + aura_effect(world, phoenix.owner, COMBUSTION, 2) / 100.0;
    let spheres = 1.0 + aura_effect(world, phoenix.owner, SPHERE, 0) / 100.0;
    let guardian = world
        .get::<AuraStatMods>(phoenix.owner)
        .map_or(1.0, |mods| mods.guardian_damage_mult);
    combustion * spheres * guardian
}

fn aura_effect(world: &World, actor: Entity, spell: u32, index: u8) -> f32 {
    world
        .get::<ActiveAuras>(actor)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == spell))
        .map_or(0.0, |aura| {
            aura.effects
                .iter()
                .find(|effect| effect.index == index)
                .expect("native Phoenix buff effect")
                .amount
                * f32::from(aura.stacks.max(1))
        })
}
