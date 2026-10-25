//! SIMC_WW_FLURRY_CHARGE_RELEASE, core only; SimC76f8118 sc_monk.cpp
//! 5148–5176/2059–2066/2256–2271 earns native451021 from actual white/DT impacts;
//! 747–784/1817–1826 consumes it on FoF and schedules native children150ms apart.
use bevy::prelude::*;
use shared::components::WeaponType;

use crate::auras::{self, ActiveAuras, AuraInstance, RemoveReason};
use crate::class_progression::TraitLoadout;
use crate::melee::Hand;
use crate::spell_cast::{self, effects::EffectCtx, queued_triggers};
use crate::spell_info::SpellInfo;
use crate::stats::CharacterStats;

const FLURRY: u32 = 450615;
const CHARGE: u32 = 451021;
const STRIKE: u32 = 450617;
const AREA: u32 = 451250;
const DUAL_THREAT: u32 = 451839;
const ONE_VERSUS_MANY: u32 = 450988;
/// SimC76f8118 sc_monk.cpp779–781: each pair uses i * 150_ms.
const STRIKE_SPACING_MS: u32 = 150;

/// Generic melee callback receives the committed hand/result, not damage-derived crit.
pub(crate) fn after_melee_hit(world: &mut World, caster: Entity, hand: Hand, critical: bool) {
    let Some(count) = earned_charge_count(world, caster, hand, critical) else {
        return;
    };
    // Each native application adds one real stack; the ordinary aura owns its cap.
    for _ in 0..count {
        spell_cast::cast_triggered(world, caster, CHARGE, Some(caster), 1);
    }
}

/// Actual landed Dual Threat damage earns once; its replaced white impact does not.
pub(crate) fn after_direct_damage(world: &mut World, ctx: &EffectCtx, critical: bool) {
    if ctx.spell.id == DUAL_THREAT {
        // SimC2064–2066 attributes Dual Threat to the main-hand weapon group.
        after_melee_hit(world, ctx.caster, Hand::Main, critical);
    }
}

fn selected(world: &World, caster: Entity) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == FLURRY)
    })
}

fn owned_aura(auras: &ActiveAuras, caster: Entity, spell: u32) -> Option<&AuraInstance> {
    auras
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)
}

fn earned_charge_count(world: &World, caster: Entity, hand: Hand, critical: bool) -> Option<u8> {
    if !selected(world, caster) {
        return None;
    }
    let stats = world.get::<CharacterStats>(caster)?;
    let weapon = match hand {
        Hand::Main => stats.main_hand,
        Hand::Off => stats.off_hand?,
    };
    let index = match weapon.kind {
        WeaponType::OneHand | WeaponType::Dagger => 0,
        WeaponType::TwoHand => 1,
        WeaponType::Ranged => return None,
    };
    let auras = world.get::<ActiveAuras>(caster)?;
    let passive = owned_aura(auras, caster, FLURRY)?;
    let base = passive
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("native Flurry retains its weapon-group charge count")
        .amount;
    Some((base * critical_multiplier(auras, caster, critical)) as u8)
}

fn critical_multiplier(auras: &ActiveAuras, caster: Entity, critical: bool) -> f32 {
    if !critical {
        return 1.0;
    }
    owned_aura(auras, caster, ONE_VERSUS_MANY).map_or(1.0, |talent| {
        1.0 + talent
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native One Versus Many retains its critical charge multiplier")
            .amount
    })
}

/// The existing AfterCast seam runs once for the admitted FoF execute.
pub(super) fn release(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !selected(world, caster) {
        return;
    }
    let count = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| owned_aura(auras, caster, CHARGE))
        .map_or(0, |aura| aura.stacks);
    if count == 0 {
        return;
    }
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == CHARGE && aura.caster == caster
    });
    let Some(primary) = spell.cast_primary_target else {
        warn!(
            ?caster,
            "Flurry release lacks the FoF cast's requested primary"
        );
        return;
    };
    schedule_strikes(world, caster, primary, spell, count);
}

fn schedule_strikes(
    world: &mut World,
    caster: Entity,
    primary: Entity,
    spell: &SpellInfo,
    count: u8,
) {
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    for index in 0..count {
        let delay = u32::from(index) * STRIKE_SPACING_MS;
        queued_triggers::schedule_full_trigger(world, &ctx, primary, STRIKE, delay);
        queued_triggers::schedule_full_trigger(world, &ctx, primary, AREA, delay);
    }
}
