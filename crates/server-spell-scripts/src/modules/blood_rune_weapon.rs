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
    SpellScriptHook::AfterForegroundSpecial(start_mist),
    SpellScriptHook::AfterForegroundSpecial(copy_weapon_actions),
    SpellScriptHook::EarlyAuraRegistration(register_dance),
    SpellScriptHook::AvoidanceProcHitMask(dance_avoidance_mask),
    SpellScriptHook::AfterForegroundSpecial(consume_dance),
    SpellScriptHook::AfterPayment(spawn_dance_weapon),
    SpellScriptHook::AfterSpellPayment(track_mist_spending),
    SpellScriptHook::AfterPolicyDirectDamage(grant_mist_power),
    SpellScriptHook::AfterCastUpdate(sync_rune_weapon_buff),
    SpellScriptHook::AsynchronousAuras(&[1264407, 1265577]),
];

#[derive(Component, Default)]
struct DanceAttempts(u32);

#[derive(Component)]
struct RegisteredRuneWeapon;

#[derive(Component, Default)]
struct BloodMistState {
    spent: f32,
    hits: u32,
}

// SimC76f8118 sc_death_knight.cpp9138–9160/9532–9533/12483–12497/13198–13199.
fn start_mist(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id == 49028
        && world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250)
        && world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(1263743))
    {
        world.entity_mut(caster).insert(BloodMistState::default());
        spell_cast::cast_triggered(world, caster, 1263729, Some(caster), 1);
    }
}

fn reset_mist_hits(world: &mut World, unit: Entity, _: &AuraInstance, effect: u8) {
    if effect == 0
        && let Some(mut state) = world.get_mut::<BloodMistState>(unit)
    {
        state.hits = 0;
    }
}

fn grant_mist_power(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx<'_>,
    _: Entity,
    _: bool,
) {
    if ctx.spell.id != 1263752 {
        return;
    }
    let cap = world
        .resource::<SpellStore>()
        .get(1263729)
        .expect("Blood Mist")
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .expect("Blood Mist target cap")
        .base_points as u32;
    let Some(mut state) = world.get_mut::<BloodMistState>(ctx.caster) else {
        return;
    };
    if state.hits >= cap {
        return;
    }
    state.hits += 1;
    spell_cast::cast_triggered(world, ctx.caster, 1263774, Some(ctx.caster), ctx.depth + 1);
}

fn track_mist_spending(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    costs: &[(CostPool, f32)],
) {
    let mut spent: f32 = costs
        .iter()
        .filter_map(|(pool, amount)| {
            (*pool == CostPool::Power(PowerType::RunicPower) && *amount > 0.0)
                .then_some(*amount / 10.0)
        })
        .sum();
    // SimC76f8118 sc_death_knight.cpp13183–13199 adds back Improved Death Strike's reduction.
    if spent > 0.0 && spell.id == 49998 && auras::has_aura(world, caster, 374277) {
        let reduction = world
            .resource::<SpellStore>()
            .get(374277)
            .expect("Improved Death Strike")
            .effects
            .iter()
            .find(|effect| effect.index == 4)
            .expect("Blood runic power reduction")
            .base_points;
        spent -= reduction / 10.0;
    }
    if let Some(mut state) = world.get_mut::<BloodMistState>(caster) {
        state.spent += spent;
    }
}

fn finish_mist(world: &mut World, unit: Entity, aura: &AuraInstance, reason: RemoveReason) {
    let Some(state) = world.entity_mut(unit).take::<BloodMistState>() else {
        return;
    };
    if reason != RemoveReason::Expired
        || aura.caster != unit
        || !world
            .get::<KnownSpells>(unit)
            .is_some_and(|known| known.contains(1263781))
    {
        return;
    }
    let percent = world
        .resource::<SpellStore>()
        .get(1263789)
        .expect("Sanguinary Burst modifier")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Sanguinary Burst amount")
        .base_points
        / 100.0;
    spell_cast::cast_triggered_with_overrides(
        world,
        unit,
        1263786,
        Some(unit),
        1,
        TriggeredSpellOverrides {
            damage_multiplier: Some(1.0 + state.spent * percent),
            ..Default::default()
        },
    );
}

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
    if !costs
        .iter()
        .any(|(pool, _)| *pool == CostPool::Power(PowerType::Runes))
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp13072–13075 rolls once per rune resource
    // loss, including a rune cost reduced to zero.
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

// The DRW parry buff follows the longest-lived real weapon, including Dance spawns.
fn sync_rune_weapon_buff(world: &mut World) {
    let now = spell_cast::now(world);
    let weapons: Vec<_> = world
        .query::<(
            Entity,
            &spell_cast::summon::SummonedCreature,
            Option<&RegisteredRuneWeapon>,
        )>()
        .iter(world)
        .filter(|(_, summon, _)| matches!(summon.spell_id, 49028 | 1237128 | 1264353))
        .filter_map(|(entity, summon, registered)| {
            summon
                .expires_at
                .filter(|end| *end > now)
                .map(|end| (entity, summon.owner, end, registered.is_none()))
        })
        .collect();
    // SimC76f8118 sc_death_knight.cpp:4353–4361: both buffs inherit the
    // resolved summon lifetime, without applying its duration modifier again.
    for (weapon, owner, end, is_new) in weapons {
        if is_new {
            world.entity_mut(weapon).insert(RegisteredRuneWeapon);
            if world
                .get::<KnownSpells>(owner)
                .is_some_and(|known| known.contains(1264405))
            {
                spell_cast::cast_triggered_with_overrides(
                    world,
                    owner,
                    1264407,
                    Some(owner),
                    1,
                    TriggeredSpellOverrides {
                        fixed_aura_duration_ms: Some(((end - now) * 1000.0).round() as u32),
                        ..Default::default()
                    },
                );
            }
        }
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
                    fixed_aura_duration_ms: Some(((end - now) * 1000.0).round() as u32),
                    ..Default::default()
                },
            );
        }
    }
}

fn register_dance(scripts: &mut combat::auras::scripts::AuraScripts) {
    let mist = scripts.0.entry(1263729).or_default();
    mist.on_periodic = Some(reset_mist_hits);
    mist.on_remove = Some(finish_mist);
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
    if !matches!(spell.id, 195292 | 195182 | 49998 | 206930 | 433895 | 50842) {
        return;
    }
    let target = spell.cast_primary_target;
    if spell.id != 50842 && target.is_none() {
        return;
    }
    let weapons: Vec<Entity> = world
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(world)
        .filter(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == caster.to_bits())
        .map(|(entity, _, _)| entity)
        .collect();
    for weapon in weapons {
        // SimC76f8118 sc_death_knight.cpp13856–13883: each owner
        // Vampiric Strike grants an independent Transfusion stack to each weapon.
        if spell.id == 433895
            && world
                .get::<KnownSpells>(caster)
                .is_some_and(|known| known.contains(1265574))
        {
            spell_cast::cast_triggered_with_overrides(
                world,
                weapon,
                1265577,
                Some(weapon),
                1,
                TriggeredSpellOverrides {
                    effect_mask: Some(1),
                    ..Default::default()
                },
            );
        }
        match spell.id {
            49998 | 50842 => {
                let (copy, mask) = match spell.id {
                    49998 => (49998, 1),
                    50842 => (50842, 1),
                    _ => unreachable!(),
                };
                spell_cast::cast_triggered_with_overrides(
                    world,
                    weapon,
                    copy,
                    target,
                    1,
                    TriggeredSpellOverrides {
                        effect_mask: Some(mask),
                        ..Default::default()
                    },
                );
            }
            206930 => {
                let Some(mut copy) = spell_cast::load_triggered_spell(world, 228645, 1) else {
                    continue;
                };
                let base = copy
                    .effects
                    .iter()
                    .find(|effect| effect.index == 1)
                    .expect("rune weapon Heart Strike")
                    .chain_targets;
                let targets = auras::spell_mods::modify(
                    world,
                    caster,
                    &copy,
                    auras::SpellModOp::ChainTargets,
                    base as f32,
                ) as i32;
                copy.effects.retain(|effect| effect.index == 1);
                copy.effects[0].chain_targets = targets;
                spell_cast::launch_triggered_spell(world, weapon, &copy, target, 1);
            }
            433895 => {
                let Some(mut copy) = spell_cast::load_triggered_spell(world, 433895, 1) else {
                    continue;
                };
                copy.effects.retain(|effect| effect.index == 4);
                copy.effects[0].implicit_targets = [6, 0];
                spell_cast::launch_triggered_spell(world, weapon, &copy, target, 1);
            }
            _ => {
                spell_cast::cast_triggered(world, weapon, spell.id, target, 1);
            }
        }
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
