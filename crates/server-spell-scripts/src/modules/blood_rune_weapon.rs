//! Insatiable Blade's Bone Shield stacks on Dancing Rune Weapon.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, AuraApplication, AuraInstance, AuraSource, RemoveReason, procs::AuraEvent,
};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    self, CostPool, TriggeredSpellOverrides, script_registry::SpellScriptHook,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, Npc, PowerType, UnitSummonedBy};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterForegroundSpecial(grant_bones),
    SpellScriptHook::AfterForegroundSpecial(copy_weapon_actions),
    SpellScriptHook::EarlyAuraRegistration(register_dance),
    SpellScriptHook::AvoidanceProcHitMask(dance_avoidance_mask),
    SpellScriptHook::AfterForegroundSpecial(consume_dance),
    SpellScriptHook::AfterPayment(spawn_dance_weapon),
    SpellScriptHook::AfterCastUpdate(sync_rune_weapon_buff),
];

#[derive(Component, Default)]
struct DanceAttempts(u32);

// SimC76f8118 sc_death_knight.cpp13063–13075/14820–14822: 12.5%
// accumulated chance on rune spending; the source explicitly says it is not in data.
fn spawn_dance_weapon(world: &mut World, caster: Entity, costs: &[(CostPool, f32)]) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 250)
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(1264506))
    {
        return;
    }
    let runes = costs
        .iter()
        .filter_map(|(pool, amount)| {
            (*pool == CostPool::Power(PowerType::Runes)).then_some(*amount)
        })
        .sum::<f32>();
    if runes <= 0.0 {
        return;
    }
    for _ in 0..runes as u32 {
        let mut entity = world.entity_mut(caster);
        let mut state = entity.entry::<DanceAttempts>().or_default();
        let mut state = state.get_mut();
        state.0 += 1;
        let chance = combat::random::accumulated_proc_constant(0.125) * f64::from(state.0);
        if combat::random::random::<f64>() < chance {
            state.0 = 0;
            drop(state);
            drop(entity);
            spell_cast::cast_triggered(world, caster, 1264353, Some(caster), 1);
        }
    }
}

// The DRW parry buff follows the longest-lived real weapon, including Dance spawns.
fn sync_rune_weapon_buff(world: &mut World) {
    let now = spell_cast::now(world);
    let weapons: Vec<_> = world
        .query::<&spell_cast::summon::SummonedCreature>()
        .iter(world)
        .filter(|summon| matches!(summon.spell_id, 49028 | 1237128 | 1264353))
        .filter_map(|summon| {
            summon
                .expires_at
                .filter(|end| *end > now)
                .map(|end| (summon.owner, end))
        })
        .collect();
    for (owner, end) in weapons {
        let remaining = world
            .get::<auras::ActiveAuras>(owner)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 81256 && aura.caster == owner)
                    .and_then(|aura| aura.expires_at)
            })
            .unwrap_or(now);
        if end > remaining + 0.001 {
            spell_cast::cast_triggered_with_overrides(
                world,
                owner,
                81256,
                Some(owner),
                1,
                TriggeredSpellOverrides {
                    duration_ms: Some(((end - now) * 1000.0).round() as i32),
                    ..Default::default()
                },
            );
        }
    }
}

fn register_dance(scripts: &mut combat::auras::scripts::AuraScripts) {
    let dance = scripts.0.entry(1264351).or_default();
    dance.check_proc = Some(dance_parry);
    dance.on_proc = Some(grant_dance);
}

fn dance_avoidance_mask(spell: u32) -> Option<u32> {
    (spell == 1264351).then_some(combat::auras::procs::PROC_HIT_PARRY)
}

fn dance_parry(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.hit_mask & combat::auras::procs::PROC_HIT_PARRY != 0
        && auras::has_aura(world, aura.caster, 81256)
}

fn grant_dance(world: &mut World, caster: Entity, _: &AuraInstance, _: Option<Entity>) {
    spell_cast::cast_triggered(world, caster, 1264568, Some(caster), 1);
}

fn consume_dance(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if matches!(spell.id, 206930 | 433895) {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 1264568 && aura.caster == caster
        });
    }
}

// SimC76f8118 sc_death_knight.cpp9523–9527/13955–13965 summons the
// selected second weapon and mirrors the owner's Bone Shield generators.
fn copy_weapon_actions(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 250)
    {
        return;
    }
    if spell.id == 49028 {
        if world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(377668))
        {
            spell_cast::cast_triggered(world, caster, 1237128, Some(caster), 1);
        }
        return;
    }
    if !matches!(spell.id, 195292 | 195182) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let weapons: Vec<Entity> = world
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(world)
        .filter(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == caster.to_bits())
        .map(|(entity, _, _)| entity)
        .collect();
    for weapon in weapons {
        spell_cast::cast_triggered(world, weapon, spell.id, Some(target), 1);
    }
}

// SimC76f8118 sc_death_knight.cpp9500–9521 grants Insatiable Blade's
// authored effect2 stacks when Dancing Rune Weapon is cast.
fn grant_bones(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 49028
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250)
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(377637))
    {
        return;
    }
    let stacks = world
        .resource::<SpellStore>()
        .get(377637)
        .expect("Insatiable Blade talent")
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Insatiable Blade stacks")
        .base_points as u32;
    for _ in 0..stacks {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: 195181,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}
