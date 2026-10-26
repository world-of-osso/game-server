//! Native Flurry charge release and selected hero producers; SimC76f8118 sc_monk.cpp
//! 5148–5176/2059–2066/2256–2271 earns native451021 from actual white/DT impacts;
//! 747–784/1817–1826 consumes it on FoF and schedules native children150ms apart.
use bevy::prelude::*;
use shared::components::WeaponType;

use crate::auras::scripts::{AuraScript, AuraScripts};
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
const WISDOM: u32 = 1272821;
const STAND_READY: u32 = 1262603;
const STAND_READY_BUFF: u32 = 1237196;
const ZENITH: u32 = 1249625;

#[derive(Component)]
struct WisdomReadyAt(f64);

pub(crate) fn register_hero_flurry(scripts: &mut AuraScripts) {
    scripts.0.insert(
        STAND_READY_BUFF,
        AuraScript {
            on_proc: Some(release_stand_ready),
            ..Default::default()
        },
    );
}

pub(super) fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC3995/1136/1672: Zenith arms Stand Ready; actual RSK/SCK
    // executes may release Wisdom while Zenith is active, independently of FoF.
    match spell.id {
        ZENITH if selected_talent(world, caster, STAND_READY) => {
            spell_cast::cast_triggered(world, caster, STAND_READY_BUFF, Some(caster), 1);
        }
        107428 | 101546
            if selected_talent(world, caster, WISDOM) && auras::has_aura(world, caster, ZENITH) =>
        {
            release_wisdom(world, caster, spell);
        }
        _ => {}
    }
}

fn release_wisdom(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let now = spell_cast::now(world);
    if world
        .get::<WisdomReadyAt>(caster)
        .is_some_and(|ready| ready.0 > now)
    {
        return;
    }
    let Some(primary) = spell.cast_primary_target else {
        warn!(?caster, "Wisdom release lacks its attack's primary");
        return;
    };
    // SimC767–770: use the talent's real internal cooldown and count.
    let cooldown_ms = world
        .resource::<crate::spell_info::SpellStore>()
        .get(WISDOM)
        .expect("native Wisdom talent")
        .aura_options
        .as_ref()
        .expect("native Wisdom proc metadata")
        .proc_category_recovery_ms;
    let count = super::script_effect_value(world, caster, WISDOM, 0) as u8;
    world
        .entity_mut(caster)
        .insert(WisdomReadyAt(now + f64::from(cooldown_ms) / 1000.0));
    schedule_strikes(world, caster, primary, spell, count, 1.0);
}

fn release_stand_ready(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    other: Option<Entity>,
) {
    // SimC6856–6865: the native buff's eligible proc consumes the entire
    // armed batch before any delayed child can trigger another callback.
    let Some(primary) = other else {
        warn!(?unit, "Stand Ready proc lacks its native attack target");
        return;
    };
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(aura.spell_id)
        .expect("native Stand Ready buff")
        .clone();
    let count = super::script_effect_value(world, unit, STAND_READY, 0) as u8;
    let efficiency = super::script_effect_value(world, unit, STAND_READY, 1) / 100.0;
    auras::remove_auras(world, unit, RemoveReason::Consumed, |candidate| {
        candidate.spell_id == STAND_READY_BUFF && candidate.caster == aura.caster
    });
    // SimC735–738 scales only the primary strike, never the AREA child.
    schedule_strikes(world, unit, primary, &spell, count, efficiency);
}
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

fn selected_talent(world: &World, caster: Entity, talent: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == talent)
    })
}

fn owned_aura(auras: &ActiveAuras, caster: Entity, spell: u32) -> Option<&AuraInstance> {
    auras
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)
}

fn earned_charge_count(world: &World, caster: Entity, hand: Hand, critical: bool) -> Option<u8> {
    if !selected_talent(world, caster, FLURRY) {
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
    if !selected_talent(world, caster, FLURRY) {
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
    schedule_strikes(world, caster, primary, spell, count, 1.0);
}

fn schedule_strikes(
    world: &mut World,
    caster: Entity,
    primary: Entity,
    spell: &SpellInfo,
    count: u8,
    primary_multiplier: f32,
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
        queued_triggers::schedule_scaled_full_trigger(
            world,
            &ctx,
            queued_triggers::ScaledTrigger {
                target: primary,
                spell_id: STRIKE,
                delay_ms: delay,
                multiplier: primary_multiplier,
            },
        );
        queued_triggers::schedule_full_trigger(world, &ctx, primary, AREA, delay);
    }
}
