//! Spell scripts (TrinityCore `SpellScript`) keyed by spell ID: behaviour that the
//! spell's DB2 effects do not carry.

use bevy::prelude::*;

use super::targets::{Relation, relation_matches};
use crate::auras::mount::{FLIGHT_STYLE_STEADY, FlightStyle, set_flight_style};
use crate::spell_info::SpellInfo;

/// Frostbolt: its effects are TRIGGER_MISSILE 32 to 228597 (SCHOOL_DAMAGE) only.
const FROSTBOLT: u32 = 116;
/// Chilled: APPLY_AURA MOD_DECREASE_SPEED 33 (-50%, 8 s).
const CHILLED: u32 = 205_708;

/// Chaos Strike (Demon Hunter, no spec): DUMMY 0 only.
const CHAOS_STRIKE_INITIAL: u32 = 344_862;
/// Chaos Strike: DUMMY 0, TRIGGER_SPELL 1 (222031) and 2 (199547), the weapon hits.
const CHAOS_STRIKE: u32 = 162_794;
/// Living Flame (Red): DUMMY 0 only.
const LIVING_FLAME: u32 = 361_469;
/// Living Flame damage: SCHOOL_DAMAGE 0.
const LIVING_FLAME_DAMAGE: u32 = 361_500;
/// Living Flame heal.
const LIVING_FLAME_HEAL: u32 = 361_509;
/// Switch Flight Style: 436854 toggles, 460002 to Skyriding, 460003 to Steady.
const SWITCH_FLIGHT_STYLE: u32 = 436_854;
const SWITCH_TO_SKYRIDING: u32 = 460_002;
const SWITCH_TO_STEADY: u32 = 460_003;
/// Azure Strike (Blue): DUMMY 0 (the target count), SCHOOL_DAMAGE 1 in an area.
const AZURE_STRIKE: u32 = 362_969;
const FORTIFYING_BREW: u32 = 115_203;
const FORTIFYING_BREW_BUFF: u32 = 120_954;
const SHIELD_BLOCK: u32 = 2_565;
const SHIELD_BLOCK_AURA: u32 = 132_404;
const BLOODTHIRST: u32 = 23_881;
const BLOODTHIRST_HEAL: u32 = 117_313;
const RAMPAGE: u32 = 184_367;
const ENRAGE: u32 = 184_362;
const HOLY_SHOCK: u32 = 20_473;
const HOLY_SHOCK_DAMAGE: u32 = 25_912;
const HOLY_SHOCK_HEAL: u32 = 25_914;
const PENANCE: u32 = 47_540;
const PENANCE_DAMAGE_CHANNEL: u32 = 47_758;
const PENANCE_HEAL_CHANNEL: u32 = 47_757;
const COBRA_SHOT: u32 = 193_455;
const KILL_COMMAND: u32 = 34_026;
const BLACKOUT_KICK: u32 = 100_784;
const BLACKOUT_KICK_RANK_TWO: u32 = 261_917;
const RISING_SUN_KICK: u32 = 107_428;
const FISTS_OF_FURY: u32 = 113_656;
const BACKSTAB: u32 = 53;
const PYRE_IMPACT: u32 = 393_568;
const PYRE_DAMAGE: u32 = 357_212;
const DRAIN_SOUL: u32 = 198_590;
const FIRE_BLAST: u32 = 108_853;
const DEATH_COIL: u32 = 47_541;
const DEATH_COIL_DAMAGE: u32 = 47_632;
const DEATH_COIL_HEAL: u32 = 47_633;
const LICHBORNE: u32 = 49_039;
const OUTBREAK: u32 = 77_575;
const DREAD_PLAGUE: u32 = 1_240_996;
const VIRULENT_PLAGUE: u32 = 191_587;
const CHAIN_HEAL: u32 = 1_064;

/// `SpellEffectFn` on `OnEffectHitTarget`: runs per unit the effect selects,
/// after the effect's handler. A DUMMY effect with a hook selects targets.
pub type EffectHitHook = fn(&mut World, Entity, Entity, u8);

pub fn on_effect_hit(spell_id: u32, effect_index: u8) -> Option<EffectHitHook> {
    match (spell_id, effect_index) {
        (FORTIFYING_BREW, 0) => Some(fortifying_brew),
        (SHIELD_BLOCK, 0) => Some(shield_block),
        (BLOODTHIRST, 0) => Some(bloodthirst),
        (RAMPAGE, 0) => Some(rampage),
        (HOLY_SHOCK, 0) => Some(holy_shock),
        (PENANCE, 0) => Some(penance),
        (COBRA_SHOT, 1) => Some(cobra_shot),
        (BLACKOUT_KICK, 0) => Some(blackout_kick),
        (PYRE_IMPACT, 0) => Some(pyre),
        (DEATH_COIL, 0) => Some(death_coil),
        (CHAOS_STRIKE_INITIAL, 0) => Some(chaos_strike_initial),
        (LIVING_FLAME, 0) => Some(living_flame),
        (SWITCH_FLIGHT_STYLE, 0) => Some(switch_flight_style),
        (SWITCH_TO_SKYRIDING, 0) => Some(switch_to_skyriding),
        (SWITCH_TO_STEADY, 0) => Some(switch_to_steady),
        _ => None,
    }
}

/// TC a352b1fa spell_mage.cpp:804-815 overrides Fire Blast's crit chance
/// to 100%; Retail108853 tooltip explicitly renders `${$s1*2}`.
pub fn crit_chance(spell_id: u32) -> Option<f32> {
    match spell_id {
        FIRE_BLAST => Some(100.0),
        _ => None,
    }
}

pub type DamageAmountHook = fn(&World, Entity, Entity, f32) -> f32;

pub fn damage_amount(spell_id: u32) -> Option<DamageAmountHook> {
    match spell_id {
        BACKSTAB => Some(backstab_damage),
        DRAIN_SOUL => Some(drain_soul_damage),
        _ => None,
    }
}

/// TC a352b1fa spell_warlock.cpp:840-849: evaluate the execute threshold
/// each tick, not at application. Retail $s2=100% and $s3=20%.
fn drain_soul_damage(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    let Some(health) = world.get::<shared::components::Health>(target) else {
        return amount;
    };
    let threshold = script_effect_value(world, caster, DRAIN_SOUL, 2);
    if health.current < health.max * threshold / 100.0 {
        let percent = script_effect_value(world, caster, DRAIN_SOUL, 1);
        amount * (1.0 + percent / 100.0)
    } else {
        amount
    }
}

/// TC a352b1fa spell_rogue.cpp:225-251, spell_rog_backstab: effect 3
/// increases damage only in the target's rear half-plane. Retail $s4=20%.
fn backstab_damage(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    let (Some(from), Some(to)) = (
        super::targets::position(world, target),
        super::targets::position(world, caster),
    ) else {
        return amount;
    };
    let angle = super::targets::orientation(world, target);
    let forward = Vec3::new(angle.cos(), 0.0, -angle.sin());
    let behind = forward.dot(to - from) < 0.0;
    if behind {
        let percent = script_effect_value(world, caster, BACKSTAB, 3);
        amount * (1.0 + percent / 100.0)
    } else {
        amount
    }
}

/// `SpellCastFn` on `AfterCast`: runs once on the caster after the spell resolved.
pub type AfterCastHook = fn(&mut World, Entity, &SpellInfo);

pub fn after_cast(spell_id: u32) -> Option<AfterCastHook> {
    shared::skyriding::ability_impulse(spell_id).map(|_| crate::skyriding::flap as AfterCastHook)
}

/// `SpellObjectAreaTargetSelectFn`: narrows the units an effect selected.
/// Receives the spell and its explicit target.
pub type AreaTargetFilter = fn(&SpellInfo, Option<Entity>, &mut Vec<Entity>);

pub fn area_target_filter(spell_id: u32, effect_index: u8) -> Option<AreaTargetFilter> {
    match (spell_id, effect_index) {
        (AZURE_STRIKE, 1) => Some(azure_strike_targets),
        _ => None,
    }
}

pub(super) type TargetPlanHook =
    fn(&mut World, Entity, &SpellInfo, Option<Entity>) -> super::TargetPlan;

/// Script-owned target plans still use normal validation, hit and effect paths.
pub(super) fn target_plan(spell_id: u32) -> Option<(Relation, TargetPlanHook)> {
    match spell_id {
        CHAIN_HEAL => Some((Relation::Ally, chain_heal_plan)),
        _ => None,
    }
}

/// Retail1064 $s2/$s3 supply falloff and jump radius; effect0 supplies four
/// total targets. TC a352b1fa Spell.cpp:2273-2322 chooses greatest HP deficit,
/// then moves the next jump's source. Its old 12.5yd default is NOT Retail's20yd.
fn chain_heal_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> super::TargetPlan {
    let primary = explicit.expect("Chain Heal has a validated ally target");
    let jumps = spell.effects[0].chain_targets.saturating_sub(1) as usize;
    let radius = script_effect_value(world, caster, CHAIN_HEAL, 2);
    let amplitude = 1.0 - script_effect_value(world, caster, CHAIN_HEAL, 1) / 100.0;
    let mut units = vec![primary];
    units.extend(chain_heal_jumps(world, caster, primary, radius, jumps));
    let chain_multipliers =
        std::iter::successors(Some(1.0), |multiplier| Some(multiplier * amplitude))
            .zip(&units)
            .map(|(multiplier, &unit)| (unit, multiplier))
            .collect();
    vec![super::PlannedEffect {
        index: 0,
        units,
        dest: None,
        chain_multipliers,
    }]
}

fn chain_heal_jumps(
    world: &mut World,
    caster: Entity,
    primary: Entity,
    radius: f32,
    jumps: usize,
) -> Vec<Entity> {
    let center = super::targets::position(world, primary).expect("validated target position");
    let mut candidates = super::targets::area_units(
        world,
        caster,
        center,
        radius * jumps as f32,
        Relation::Ally,
        0,
    );
    candidates.retain(|&unit| unit != primary);
    let mut source = primary;
    let mut chain = Vec::new();
    while chain.len() < jumps {
        let from = super::targets::position(world, source).expect("chain source position");
        let Some(slot) = most_injured_within(world, &candidates, from, radius) else {
            break;
        };
        source = candidates.remove(slot);
        chain.push(source);
    }
    chain
}

fn most_injured_within(
    world: &World,
    candidates: &[Entity],
    from: Vec3,
    radius: f32,
) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .filter_map(|(slot, &unit)| {
            let distance = super::targets::position(world, unit)?.distance(from);
            let health = world.get::<shared::components::Health>(unit)?;
            (distance <= radius).then_some((slot, health.max - health.current))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(slot, _)| slot)
}

/// Validate before spending costs: Retail47541 heals only an Undead ally.
/// Lichborne49039 explicitly makes its bearer Undead; undead NPC type is 6.
pub fn check_target(
    world: &World,
    caster: Entity,
    spell_id: u32,
    target: Entity,
) -> Result<(), super::CastFailure> {
    if spell_id != DEATH_COIL {
        return Ok(());
    }
    let enemy = relation_matches(world, caster, target, Relation::Enemy);
    let undead_ally =
        relation_matches(world, caster, target, Relation::Ally) && is_undead(world, target);
    if enemy || undead_ally {
        Ok(())
    } else {
        Err(super::CastFailure::InvalidTarget)
    }
}

fn is_undead(world: &World, target: Entity) -> bool {
    if crate::auras::has_aura(world, target, LICHBORNE) {
        return true;
    }
    world
        .get::<shared::components::Npc>(target)
        .and_then(|npc| {
            world
                .get_resource::<crate::world_data::WorldData>()?
                .templates
                .get(&npc.template_id)
        })
        .is_some_and(|template| template.creature_type == 6)
}

/// TC a352b1fa spell_dk.cpp:589-609 names the 47632 damage bridge.
/// Retail47541 additionally names47633 healing and $77575s3 plague extension.
fn death_coil(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let child = if relation_matches(world, caster, target, Relation::Ally) {
        DEATH_COIL_HEAL
    } else {
        DEATH_COIL_DAMAGE
    };
    super::cast_triggered(world, caster, child, Some(target), depth + 1);
    let milliseconds = script_effect_value(world, caster, OUTBREAK, 2)
        .max(0.0)
        .round() as u32;
    crate::auras::extend_owned_duration(
        world,
        caster,
        target,
        &[DREAD_PLAGUE, VIRULENT_PLAGUE],
        milliseconds,
    );
}

/// TC a352b1fa spell_evoker.cpp:601-618 spell_evo_pyre casts 357212 at
/// the hit unit's position. Its real radius/coefficients define the explosion;
/// target 87 is a scripted destination, not a generic unit-target substitution.
fn pyre(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if depth >= super::effects::MAX_TRIGGER_DEPTH {
        warn!("Pyre trigger depth exhausted");
        return;
    }
    let Some(center) = super::targets::position(world, target) else {
        return;
    };
    let child = world
        .resource::<crate::spell_info::SpellStore>()
        .get(PYRE_DAMAGE)
        .expect("Pyre requires catalog damage 357212")
        .clone();
    let effect = &child.effects[0];
    let radius = super::targets::radius(effect, 1);
    let units = super::targets::area_units(
        world,
        caster,
        center,
        radius,
        Relation::Enemy,
        super::max_targets(&child),
    );
    let mut plan = vec![super::PlannedEffect {
        index: 0,
        units,
        dest: Some(super::targets::Dest {
            position: center,
            orientation: super::targets::orientation(world, caster),
        }),
        chain_multipliers: Vec::new(),
    }];
    super::hit::remove_defended_targets(world, caster, &child, &mut plan);
    super::missiles::launch(world, caster, &child, plan, depth + 1);
}

/// Retail 100784/261917: rank two reduces Rising Sun Kick and Fists of Fury
/// by $100784m3/1000 seconds. Effect 2 stores milliseconds, not a proc chance.
fn blackout_kick(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    if crate::auras::has_aura(world, caster, BLACKOUT_KICK_RANK_TWO) {
        let milliseconds = script_effect_value(world, caster, BLACKOUT_KICK, 2);
        for spell in [RISING_SUN_KICK, FISTS_OF_FURY] {
            reduce_cooldown(world, caster, spell, f64::from(milliseconds) / 1000.0);
        }
    }
}

/// Retail 193455: "Reduces the cooldown of Kill Command by $s3 sec."
/// Effect 2 supplies that duration (1 second); no proc/PPM roll is authored.
/// Run once on the damage hit, not on each Dummy metadata effect.
fn cobra_shot(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    let seconds = script_effect_value(world, caster, COBRA_SHOT, 2);
    reduce_cooldown(world, caster, KILL_COMMAND, f64::from(seconds));
}

fn script_effect_value(world: &World, caster: Entity, spell_id: u32, index: u8) -> f32 {
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .expect("script spell must exist in SpellStore");
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("script requires its documented catalog effect");
    super::effects::modded_base_points(world, caster, spell, effect)
}

/// Reduce the currently running recharge or spell cooldown; never start one.
fn reduce_cooldown(world: &mut World, caster: Entity, spell_id: u32, seconds: f64) {
    let now = super::now(world);
    let charge_category = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .and_then(|spell| spell.categories.as_ref())
        .map_or(0, |category| category.charge_category);
    let Some(mut history) = world.get_mut::<super::SpellHistory>(caster) else {
        return;
    };
    let charges = history
        .charges
        .get_mut(&charge_category)
        .filter(|charges| charges.used > 0)
        .map(|charges| {
            charges.next_ready_at -= seconds;
            charges.refresh(now);
            *charges
        });
    let remaining_ms = history
        .spell_ready_at
        .get_mut(&spell_id)
        .filter(|ready| **ready > now)
        .map(|ready| {
            *ready = (*ready - seconds).max(now);
            ((*ready - now) * 1000.0).round() as u32
        });
    if let Some(charges) = charges {
        super::send_charges(world, caster, &charges, now);
    }
    if let Some(remaining_ms) = remaining_ms {
        super::send_to_owner(
            world,
            caster,
            shared::protocol::SpellCooldownUpdate {
                spell_id,
                category: 0,
                duration_ms: remaining_ms,
                remaining_ms,
                is_gcd: false,
            },
        );
    }
}

/// TC a352b1fa spell_priest.cpp:3070-3125,5546, spell_pri_penance:
/// DUMMY 0 starts the 47757 friendly or 47758 hostile channel. Unlike full-mask
/// generic triggers, TC preserves channel state. Both children have no costs;
/// their real periodic triggers supply bolt timing and TARGET_UNIT_CHANNEL_TARGET.
fn penance(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let child = if relation_matches(world, caster, target, Relation::Ally) {
        PENANCE_HEAL_CHANNEL
    } else {
        PENANCE_DAMAGE_CHANNEL
    };
    let Some(spell) = world
        .resource::<crate::spell_info::SpellStore>()
        .get(child)
        .cloned()
    else {
        warn!("Penance requires channel spell {child}");
        return;
    };
    if depth >= super::effects::MAX_TRIGGER_DEPTH {
        warn!("Penance channel skipped: trigger depth {depth} exhausted");
        return;
    }
    super::start_channel(world, caster, &spell, Some(target));
    super::cast_triggered(world, caster, child, Some(target), depth + 1);
}

/// TC a352b1fa spell_paladin.cpp:1261-1311, spell_pal_holy_shock:
/// DUMMY 0 casts 25914 on a friendly target, otherwise 25912. The Retail
/// 20473 tooltip references each child's $s1; its ENERGIZE 1 remains generic.
fn holy_shock(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let spell = if relation_matches(world, caster, target, Relation::Ally) {
        HOLY_SHOCK_HEAL
    } else {
        HOLY_SHOCK_DAMAGE
    };
    super::cast_triggered(world, caster, spell, Some(target), depth + 1);
}

/// TC a352b1fa spell_warrior.cpp:1754-1769, spell_warr_shield_block:
/// DUMMY 0 casts 132404. Its catalog aura supplies +100 block chance for 6 s
/// and the Shield Slam damage modifier; 2565's tooltip refers to those fields.
fn shield_block(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    super::cast_triggered(world, caster, SHIELD_BLOCK_AURA, Some(caster), depth + 1);
}

/// TC a352b1fa spell_warrior.cpp:320-411, spell_warr_bloodthirst and
/// spell_warr_bloodthirst_enrage: on the damage hit, heal via 117313 and roll
/// effect 2's chance (not SpellAuraOptions/PPM) for 184362. Catalog 23881's
/// $117313s1 and $s3 resolve to 3% max health and 30% chance. Fresh Meat's
/// optional first-target guarantee remains unsupported.
fn bloodthirst(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    let store = world.resource::<crate::spell_info::SpellStore>();
    let spell = store
        .get(BLOODTHIRST)
        .expect("resolving Bloodthirst is in SpellStore");
    let Some(effect) = spell.effects.iter().find(|effect| effect.index == 2) else {
        warn!("Bloodthirst requires effect 2's Enrage chance");
        return;
    };
    let chance = super::effects::modded_base_points(world, caster, spell, effect);
    super::cast_triggered(world, caster, BLOODTHIRST_HEAL, Some(caster), depth + 1);
    if rand::random::<f32>() * 100.0 < chance {
        super::cast_triggered(world, caster, ENRAGE, Some(caster), depth + 1);
    }
}

/// Catalog 184367's tooltip explicitly says "Enrages you". Effects 1-4
/// already trigger the four attack spells; DUMMY 0 describes their count.
/// Enrage's identity is TC a352b1fa spell_warrior.cpp's SPELL_WARRIOR_ENRAGE.
fn rampage(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    super::cast_triggered(world, caster, ENRAGE, Some(caster), depth + 1);
}

/// `spell_dh_chaos_strike_initial::HandleHit` (spell_dh.cpp:368-388): the caster
/// casts Chaos Strike 162794 at the hit unit, triggered.
fn chaos_strike_initial(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    super::cast_triggered(world, caster, CHAOS_STRIKE, Some(target), depth + 1);
}

/// Fortifying Brew's DUMMY cast applies 120954 with custom health and damage
/// amounts from 115203 (local catalog: +20%, -20%). Retail snapshots a fixed
/// health gain at cast time and removes that same gain when the buff expires:
/// simulationcraft/simc, midnight, buffs::fortifying_brew_t::trigger/expire_override
/// https://github.com/simulationcraft/simc/blob/midnight/engine/class_modules/monk/sc_monk.cpp
/// The 115203 -> 120954 DUMMY bridge also exists in the TrinityCore-derived
/// RoleplayCore spell_monk_fortifying_brew::HandleDummy:
/// https://github.com/KamiliaBlow/RoleplayCore/blob/master/src/server/scripts/Spells/spell_monk.cpp
fn fortifying_brew(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    let Some(buff) = customize_fortifying_brew(world, caster) else {
        warn!(
            "Fortifying Brew requires caster health and spells 115203/120954 with health/damage effects"
        );
        return;
    };
    // Only the two requested defensive effects; stagger, talents and cosmetic
    // size effects are not implemented by this bounded bridge.
    crate::auras::apply_aura_with_spell(
        world,
        &crate::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: FORTIFYING_BREW_BUFF,
            effect_mask: 0b11,
            source: crate::auras::AuraSource::Cast,
        },
        &buff,
    );
}

fn customize_fortifying_brew(world: &World, caster: Entity) -> Option<SpellInfo> {
    let store = world.resource::<crate::spell_info::SpellStore>();
    let cast = store.get(FORTIFYING_BREW)?;
    let mut buff = store.get(FORTIFYING_BREW_BUFF)?.clone();
    let health_percent = cast
        .effects
        .iter()
        .find(|effect| effect.index == 0)?
        .base_points;
    let damage_percent = cast
        .effects
        .iter()
        .find(|effect| effect.index == 1)?
        .base_points;
    let health_bonus = snapshot_brew_health_bonus(world, caster, health_percent)?;
    for effect in &mut buff.effects {
        match effect.index {
            0 => effect.base_points = health_bonus,
            1 => effect.base_points = damage_percent,
            _ => {}
        }
    }
    Some(buff)
}

fn snapshot_brew_health_bonus(world: &World, caster: Entity, percent: f32) -> Option<f32> {
    let health = world.get::<shared::components::Health>(caster)?;
    let existing = world
        .get::<crate::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == FORTIFYING_BREW_BUFF)
        });
    let previous_bonus = existing
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    Some(((health.max - previous_bonus) * percent / 100.0).floor())
}

/// Switch Flight Style 436854 (the Riding skill line's ability, spelldesc
/// `$?A404468[Steady Flight][Skyriding]`): DUMMY 0 on the caster swaps Steady and
/// Skyriding (`auras::mount::set_flight_style`). TrinityCore scripts a DUMMY effect with an
/// `OnEffectHitTarget` `SpellEffectFn`; it has no script for this spell.
fn switch_flight_style(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    let style = if crate::auras::has_aura(world, target, FLIGHT_STYLE_STEADY) {
        FlightStyle::Skyriding
    } else {
        FlightStyle::Steady
    };
    set_flight_style(world, target, style);
}

/// Switch Flight Style 460002 (icon ability_dragonriding_dynamicflight01): DUMMY 0 on
/// the caster puts it on Skyriding.
fn switch_to_skyriding(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    set_flight_style(world, target, FlightStyle::Skyriding);
}

/// Switch Flight Style 460003 (icon ability_dragonriding_staticflight01): DUMMY 0 on
/// the caster puts it on Steady.
fn switch_to_steady(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    set_flight_style(world, target, FlightStyle::Steady);
}

/// Energizing Flame (Evoker passive): DUMMY 0, the percent of Living Flame's mana
/// cost refunded when it hits an enemy.
const ENERGIZING_FLAME: u32 = 400_006;

/// `spell_evo_living_flame::HandleHitTarget` (spell_evoker.cpp:505-530): heal a
/// unit the caster can assist, damage any other; `HandleLaunchTarget` refunds
/// Energizing Flame's percent of the mana cost on an enemy.
fn living_flame(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if relation_matches(world, caster, target, Relation::Ally) {
        super::cast_triggered(world, caster, LIVING_FLAME_HEAL, Some(target), depth + 1);
        return;
    }
    refund_energizing_flame(world, caster);
    super::cast_triggered(world, caster, LIVING_FLAME_DAMAGE, Some(target), depth + 1);
}

fn refund_energizing_flame(world: &mut World, caster: Entity) {
    if !crate::auras::has_aura(world, caster, ENERGIZING_FLAME) {
        return;
    }
    let store = world.resource::<crate::spell_info::SpellStore>();
    let (Some(living_flame), Some(energizing)) =
        (store.get(LIVING_FLAME), store.get(ENERGIZING_FLAME))
    else {
        return;
    };
    let percent = energizing
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map_or(0.0, |effect| effect.base_points);
    let living_flame = living_flame.clone();
    let mana_cost: f32 = super::applicable_costs(world, caster, &living_flame)
        .unwrap_or_default()
        .into_iter()
        .filter(|(pool, ..)| *pool == super::CostPool::Power(shared::components::PowerType::Mana))
        .map(|(_, amount, _)| amount)
        .sum();
    if mana_cost > 0.0 {
        super::powers::change_power(
            world,
            caster,
            shared::components::PowerType::Mana,
            mana_cost * percent / 100.0,
        );
    }
}

/// `spell_evo_azure_strike::FilterTargets` (spell_evoker.cpp:94-107): the explicit
/// target plus effect 0's value minus one others picked at random
/// (`Trinity::Containers::RandomResize`).
fn azure_strike_targets(spell: &SpellInfo, explicit: Option<Entity>, units: &mut Vec<Entity>) {
    pick_azure_strike_targets(spell, explicit, units, &mut rand::thread_rng());
}

fn pick_azure_strike_targets(
    spell: &SpellInfo,
    explicit: Option<Entity>,
    units: &mut Vec<Entity>,
    rng: &mut impl rand::Rng,
) {
    use rand::seq::SliceRandom;
    let count = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map_or(1, |effect| effect.base_points.max(1.0) as usize);
    units.retain(|&unit| Some(unit) != explicit);
    let others: Vec<Entity> = units.choose_multiple(rng, count - 1).copied().collect();
    *units = others;
    units.extend(explicit);
}

/// `SpellHitFn`: runs once per unit the spell lands on, after its effects resolved there.
pub type HitHook = fn(&mut World, Entity, Entity, u8);

pub fn on_hit(spell_id: u32) -> Option<HitHook> {
    match spell_id {
        FROSTBOLT => Some(frostbolt_chilled),
        _ => None,
    }
}

/// `spell_mage_frostbolt::HandleChilled` (spell_mage.cpp:978-995): the caster casts
/// Chilled at the hit unit, triggered.
fn frostbolt_chilled(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    super::cast_triggered(world, caster, CHILLED, Some(target), depth + 1);
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;
    use crate::spell_info::SpellEffect;

    /// Azure Strike's second target is random among the other enemies in the area.
    #[test]
    fn azure_strike_keeps_the_target_and_one_random_other() {
        let mut spell = SpellInfo::default();
        spell.effects.push(SpellEffect {
            index: 0,
            base_points: 2.0,
            ..Default::default()
        });
        let mut world = World::new();
        let target = world.spawn_empty().id();
        let others: Vec<Entity> = (0..4).map(|_| world.spawn_empty().id()).collect();
        let mut picked = std::collections::HashSet::new();
        for seed in 0..32 {
            let mut units = others.clone();
            units.insert(1, target);
            pick_azure_strike_targets(
                &spell,
                Some(target),
                &mut units,
                &mut StdRng::seed_from_u64(seed),
            );
            assert_eq!(units.len(), 2);
            assert_eq!(units[1], target);
            picked.insert(units[0]);
        }
        assert!(picked.len() > 1, "always the same other: {picked:?}");
    }
}
