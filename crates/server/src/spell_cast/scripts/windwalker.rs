//! SimC76f8118 sc_monk.cpp358–373,870–871,6359–6365; native70448 values.
use bevy::prelude::*;

use crate::auras::{self, ActiveAuras, RemoveReason};
use crate::random as rand;
use crate::spell_cast::{CostPool, applicable_costs};
use crate::spell_info::SpellInfo;
use shared::components::PowerType;

// Source may_combo_strike actions, shared with the published starter policy.
const COMBO_ACTIONS: &[u32] = &[
    100780, 100784, 107428, 113656, 101546, 392983, 322109, 152175, 1249625, 1272696, 468179,
];
const COMBO_STRIKES: u32 = 115636;
const MASTERY_DAMAGE: &[u32] = &[
    100780, 100784, 185099, 117418, 107270, 395519, 395521, 322109, 468179, 451767, 158221,
    1272696, 1248815,
];

#[derive(Component)]
struct ComboStrikes {
    active: bool,
    last_action: u32,
}

pub fn last_combo_action(world: &World, caster: Entity) -> Option<u32> {
    world
        .get::<ComboStrikes>(caster)
        .map(|state| state.last_action)
}

/// SimC1160/3918: these background damage actions also advance combo identity.
pub(super) fn scripted_combo_cast(world: &mut World, caster: Entity, spell_id: u32, depth: u8) {
    if depth == 0 || !matches!(spell_id, 468179 | 1272696) {
        return;
    }
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .expect("admitted native combo child")
        .clone();
    ordinary_combo_cast(world, caster, &spell);
}

const HIT_COMBO: u32 = 196740;
const HIT_COMBO_BUFF: u32 = 196741;
const COMBO_BREAKER: u32 = 137384;
const COMBO_BREAKER_BUFF: u32 = 116768;
pub(crate) use crate::spell_info::DANCE_OF_CHIJI;
const DANCE_BUFF: u32 = 325202;
const SPINNING_CRANE_KICK: u32 = 101546;
const MYSTIC_TOUCH_PASSIVE: u32 = 8647;
const MYSTIC_TOUCH_DEBUFF: u32 = 113746;
const RUSHING_WIND_KICK_TALENT: u32 = 1250566;
const RUSHING_WIND_KICK_OVERRIDE: u32 = 1250554;
pub(super) const RUSHING_WIND_KICK: u32 = 467307;
const RISING_SUN_KICK: u32 = 107428;
pub(super) const FISTS_OF_FURY: u32 = 113656;
pub(super) const WHIRLING_DRAGON_PUNCH: u32 = 152175;
const WDP_ENABLER: u32 = 196742;
const WDP_SINGLE_DAMAGE: u32 = 451767;
const WDP_AREA_DAMAGE: u32 = 158221;

/// SIMC_WW_WDP_DAMAGE_CHILDREN: SimC76f8118 sc_monk.cpp1888–1906/5825–5826.
/// SIMC_WDP_TICK_SCHEDULE_DIFFERS: actual aura226 ticks, not SimC's manual timer.
/// Native child geometry, caps/reduced-AoE and damage calculations remain untouched.
pub(crate) fn register_wdp_relationship(scripts: &mut auras::scripts::AuraScripts) {
    scripts.0.insert(
        WHIRLING_DRAGON_PUNCH,
        auras::scripts::AuraScript {
            on_periodic: Some(cast_wdp_area_tick),
            ..Default::default()
        },
    );
}

fn cast_wdp_area_tick(world: &mut World, unit: Entity, aura: &auras::AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    crate::spell_cast::cast_triggered(world, aura.caster, WDP_AREA_DAMAGE, Some(unit), 1);
}

/// The root hits its own aura unit; the single-target child uses the request's
/// retained primary instead. The ordinary native root has one self hit.
pub(super) fn cast_wdp_single_hit(
    world: &mut World,
    ctx: &crate::spell_cast::effects::EffectCtx,
    hit: Entity,
) {
    if hit != ctx.caster {
        return;
    }
    let Some(primary) = ctx.primary_target() else {
        warn!(?ctx.caster, "WDP single-target relationship lacks its requested primary");
        return;
    };
    crate::spell_cast::cast_triggered(
        world,
        ctx.caster,
        WDP_SINGLE_DAMAGE,
        Some(primary),
        ctx.depth + 1,
    );
}

/// MONK_MYSTIC_TOUCH_SIMC_PRODUCER: SimC76f8118 sc_monk.cpp438–455.
/// Native113746 supplies the physical vulnerability; no multiplier is invented.
pub(crate) fn apply_mystic_touch_after_hit(world: &mut World, event: &auras::procs::AuraEvent) {
    if event.damage <= 0.0 || event.periodic {
        return;
    }
    let learned = world
        .get::<crate::class_progression::KnownSpells>(event.actor)
        .is_some_and(|known| known.contains(MYSTIC_TOUCH_PASSIVE));
    if !learned || !mystic_touch_action_event(world, event) {
        return;
    }
    if let Some(target) = event.target {
        crate::spell_cast::cast_triggered(world, event.actor, MYSTIC_TOUCH_DEBUFF, Some(target), 1);
    }
}

fn mystic_touch_action_event(world: &World, event: &auras::procs::AuraEvent) -> bool {
    let Some(id) = event.spell_id else {
        return event.actor_mask[0] & auras::procs::PROC_DEAL_MELEE_SWING != 0;
    };
    // The source hook belongs to monk_action_t, not every item/proc owned by a Monk.
    let store = world.resource::<crate::spell_info::SpellStore>();
    let family = store
        .get(MYSTIC_TOUCH_PASSIVE)
        .and_then(|spell| spell.class_options.as_ref())
        .map(|options| options.spell_class_set);
    store
        .get(id)
        .and_then(|spell| spell.class_options.as_ref())
        .is_some_and(|options| Some(options.spell_class_set) == family)
}

fn ordinary_cast_event(event: &auras::procs::AuraEvent) -> bool {
    event.can_proc && event.actor_mask[1] & auras::procs::PROC2_CAST_SUCCESSFUL != 0
}

/// SimC383–392: current Chi resource qualifies, even when native cost is discounted.
/// Check the root action rather than losing its resource identity on a damage child.
pub(crate) fn check_dance_proc(
    world: &World,
    aura: &auras::AuraInstance,
    event: &auras::procs::AuraEvent,
) -> bool {
    if !ordinary_cast_event(event) {
        return false;
    }
    let Some(spell) = event
        .spell_id
        .and_then(|id| world.resource::<crate::spell_info::SpellStore>().get(id))
    else {
        return false;
    };
    applicable_costs(world, aura.caster, spell).is_ok_and(|costs| {
        costs
            .iter()
            .any(|(pool, _, _)| *pool == CostPool::Power(PowerType::Chi))
    })
}

/// SimC1664–1666: the generic Chi-action roll occurs before this SCK use is spent.
pub(crate) fn after_dance_cast_event(world: &mut World, event: &auras::procs::AuraEvent) {
    if !ordinary_cast_event(event) || event.spell_id != Some(SPINNING_CRANE_KICK) {
        return;
    }
    let caster = event.actor;
    let instance = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == DANCE_BUFF && aura.caster == caster)
        })
        .map(|aura| aura.instance_id);
    if let Some(instance) = instance {
        auras::spend_stack(world, caster, instance);
    }
}

/// Source may_combo_strike actions; unrelated triggered children do not advance it.
pub(crate) fn ordinary_combo_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !COMBO_ACTIONS.contains(&spell.id) {
        return;
    }
    let previous = last_combo_action(world, caster);
    let active = previous != Some(spell.id);
    if auras::has_aura(world, caster, COMBO_STRIKES) {
        world.entity_mut(caster).insert(ComboStrikes {
            active,
            last_action: spell.id,
        });
    }
    if !auras::has_aura(world, caster, HIT_COMBO) {
        return;
    }
    if !active {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == HIT_COMBO_BUFF && aura.caster == caster
        });
    } else {
        crate::spell_cast::cast_triggered(world, caster, HIT_COMBO_BUFF, Some(caster), 1);
    }
}

/// WW_COMBO_STRIKES_SIMC_POLICY: SimC76f8118 sc_monk.cpp142–156/358–373.
/// TC has no115636 script; DB2 owns the coefficient, not SimC's numeric value.
/// Eligibility covers the published kit's damage actions/children only. Utility,
/// white hits and unrelated procs neither consume the state nor gain mastery.
pub(crate) fn combo_strikes_multiplier(world: &World, caster: Entity, spell: u32) -> f32 {
    let active = world
        .get::<ComboStrikes>(caster)
        .is_some_and(|state| state.active);
    if !active || !MASTERY_DAMAGE.contains(&spell) || !auras::has_aura(world, caster, COMBO_STRIKES)
    {
        return 1.0;
    }
    let stats = world
        .get::<crate::stats::CharacterStats>(caster)
        .expect("native Combo Strikes caster retains class stats");
    let mastery = world
        .resource::<crate::spell_info::SpellStore>()
        .get(COMBO_STRIKES)
        .expect("native Combo Strikes aura retains its definition");
    let coefficient = mastery
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Combo Strikes effect0")
        .bonus_coefficient;
    (1.0 + stats.mastery_points * coefficient / 100.0)
        * sunfire_spiral_multiplier(world, caster, spell)
}

/// WW_SUNFIRE_SPIRAL_SIMC_POLICY: SimC76f8118 sc_monk.cpp945–953.
/// TC has no script/binding and native1272415 has no class mask. The caller
/// already proved active Combo Strikes; DB2 owns this selected talent's percent.
fn sunfire_spiral_multiplier(world: &World, caster: Entity, spell: u32) -> f32 {
    if !matches!(spell, 185099 | 468179) {
        return 1.0;
    }
    let Some(aura) = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 1272415))
    else {
        return 1.0;
    };
    let effect = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Sunfire Spiral retains its effect0 percentage");
    1.0 + effect.amount / 100.0
}

/// SIMC_WW_WDP_ENABLER_PRODUCER: SimC76f8118 sc_monk.cpp1138/1826/4552–4572.
/// TC/TDB have no producer. SIMC_WW_WDP_ENABLER_DURATION overrides DB2's1000ms
/// with the source's minimum remaining cooldown plus hasted1500ms; refresh NONE.
pub(super) fn trigger_wdp_enabler_on_hit(
    world: &mut World,
    caster: Entity,
    _target: Entity,
    depth: u8,
) {
    let selected = world
        .get::<crate::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == WHIRLING_DRAGON_PUNCH)
        });
    if !selected || wdp_enabler_is_active(world, caster) {
        return;
    }
    let Some(duration_ms) = read_wdp_duration_ms(world, caster) else {
        return;
    };
    crate::spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        WDP_ENABLER,
        Some(caster),
        depth + 1,
        crate::spell_cast::TriggeredSpellOverrides {
            duration_ms: Some(duration_ms),
            ..Default::default()
        },
    );
}

fn wdp_enabler_is_active(world: &World, caster: Entity) -> bool {
    let now = crate::spell_cast::now(world);
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            let owned = aura.spell_id == WDP_ENABLER && aura.caster == caster;
            let active = aura.expires_at.is_none_or(|expiry| expiry > now);
            owned && active
        })
    })
}

fn read_wdp_duration_ms(world: &World, caster: Entity) -> Option<i32> {
    const GRACE_MS: f64 = 1500.0;
    let store = world.resource::<crate::spell_info::SpellStore>();
    let overlap = [RISING_SUN_KICK, FISTS_OF_FURY]
        .into_iter()
        .map(|id| {
            let spell = store
                .get(id)
                .expect("native WDP prerequisite spell retained");
            crate::spell_cast::cooldown_query::cooldown_value(world, caster, spell, "remains")
                .expect("native WDP prerequisite cooldown readable")
        })
        .reduce(f64::min)
        .expect("two native WDP cooldowns");
    let overlap_ms = (overlap * 1000.0).round() as i32;
    if overlap_ms <= 0 {
        return None;
    }
    let stats = world
        .get::<crate::stats::CharacterStats>(caster)
        .expect("native WDP owner retains derived haste");
    // SimC timespan.hpp132–135 truncates the scaled grace before adding cooldowns.
    let grace_ms = (GRACE_MS / (1.0 + f64::from(stats.haste_pct) / 100.0)) as i32;
    Some(overlap_ms + grace_ms)
}

/// SIMC_WW_RWK_BOK_LANDED_PRODUCER: SimC76f8118 sc_monk.cpp1408–1414.
/// TC a352b1fa has no binding. DB2 owns the selected talent's chance and override.
/// Native instant BOK hits before modifier-use consumption; misses never run OnHit.
pub(super) fn blackout_kick_hit(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    if !auras::has_aura(world, caster, COMBO_BREAKER_BUFF) {
        return;
    }
    let Some(chance) = read_rushing_wind_kick_chance(world, caster) else {
        return;
    };
    if rand::random::<f32>() * 100.0 >= chance {
        return;
    }
    reset_rising_sun_kick(world, caster);
    crate::spell_cast::cast_triggered(
        world,
        caster,
        RUSHING_WIND_KICK_OVERRIDE,
        Some(caster),
        depth + 1,
    );
}

fn selected_rushing_wind_kick_owner(world: &World, caster: Entity) -> bool {
    world
        .get::<crate::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == RUSHING_WIND_KICK_TALENT)
        })
}

/// Same named policy, SimC76f8118 sc_monk.cpp1199–1203.
/// Aura332 is not a native generic trigger type; the source action expires it.
pub(super) fn consume_rushing_wind_kick(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !selected_rushing_wind_kick_owner(world, caster) {
        return;
    }
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == RUSHING_WIND_KICK_OVERRIDE && aura.caster == caster
    });
}

fn read_rushing_wind_kick_chance(world: &World, caster: Entity) -> Option<f32> {
    if !selected_rushing_wind_kick_owner(world, caster) {
        return None;
    }
    let talent = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == RUSHING_WIND_KICK_TALENT && aura.caster == caster)?;
    Some(
        talent
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native selected Rushing Wind Kick retains effect0 chance")
            .amount,
    )
}

pub(super) fn reset_rising_sun_kick(world: &mut World, caster: Entity) {
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(RISING_SUN_KICK)
        .expect("native selected RWK retains Rising Sun Kick");
    let remaining = crate::spell_cast::cooldown_query::cooldown_value(
        world,
        caster,
        spell,
        "full_recharge_time",
    )
    .expect("native Rising Sun Kick full recharge is readable");
    // Reduce the actual complete native timer, including its shared charge category.
    // The existing gameplay helper notifies the owner and never changes the GCD.
    super::reduce_cooldown(world, caster, RISING_SUN_KICK, remaining);
}

/// Tiger Palm rolls its learned passive's native effect0 chance on a landed hit.
pub(super) fn tiger_palm_hit(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    let chance = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == COMBO_BREAKER))
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    if rand::random::<f32>() * 100.0 < chance {
        crate::spell_cast::cast_triggered(
            world,
            caster,
            COMBO_BREAKER_BUFF,
            Some(caster),
            depth + 1,
        );
    }
}
