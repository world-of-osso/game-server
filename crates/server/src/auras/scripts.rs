//! Script hooks for script-only aura types (DUMMY 4, PERIODIC_DUMMY 226),
//! keyed by spell ID. Class registrations below run on
//! apply and remove of that spell's auras, on its PERIODIC_DUMMY ticks and on
//! its procs. Every hook receives the unit carrying the aura.

use std::collections::HashMap;

use bevy::prelude::*;

use super::AuraInstance;

pub type AuraHook = fn(&mut World, Entity, &AuraInstance);
pub type RemoveHook = fn(&mut World, Entity, &AuraInstance, super::RemoveReason);
/// Also receives the ticking effect index.
pub type PeriodicHook = fn(&mut World, Entity, &AuraInstance, u8);
/// Also receives the other party of the proc event.
pub type ProcHook = fn(&mut World, Entity, &AuraInstance, Option<Entity>);
pub type ProcCheckHook = fn(&World, &AuraInstance, &super::procs::AuraEvent) -> bool;
pub type DamageProcHook = fn(&mut World, Entity, &AuraInstance, &super::procs::AuraEvent);

#[derive(Debug, Clone, Copy, Default)]
pub struct AuraScript {
    pub on_apply: Option<AuraHook>,
    pub on_remove: Option<RemoveHook>,
    pub on_periodic: Option<PeriodicHook>,
    pub on_proc: Option<ProcHook>,
    pub check_proc: Option<ProcCheckHook>,
    pub on_damage_proc: Option<DamageProcHook>,
}

#[derive(Resource, Debug, Default)]
pub struct AuraScripts(pub HashMap<u32, AuraScript>);

fn script(world: &World, spell_id: u32) -> Option<AuraScript> {
    world
        .get_resource::<AuraScripts>()
        .and_then(|scripts| scripts.0.get(&spell_id).copied())
}

pub(super) fn run_apply(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if let Some(run) = script(world, aura.spell_id).and_then(|script| script.on_apply) {
        run(world, unit, aura);
    }
}

pub(super) fn run_remove(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    reason: super::RemoveReason,
) {
    if let Some(run) = script(world, aura.spell_id).and_then(|script| script.on_remove) {
        run(world, unit, aura, reason);
    }
}

pub(super) fn has_proc_script(world: &World, spell_id: u32) -> bool {
    script(world, spell_id)
        .is_some_and(|script| script.on_proc.is_some() || script.on_damage_proc.is_some())
}

pub(super) fn run_proc(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    other: Option<Entity>,
    event: &super::procs::AuraEvent,
) {
    if let Some(run) = script(world, aura.spell_id).and_then(|script| script.on_proc) {
        run(world, unit, aura, other);
    }
    if let Some(run) = script(world, aura.spell_id).and_then(|script| script.on_damage_proc) {
        run(world, unit, aura, event);
    }
}

pub(super) fn check_proc(
    world: &World,
    aura: &AuraInstance,
    event: &super::procs::AuraEvent,
) -> bool {
    script(world, aura.spell_id)
        .and_then(|script| script.check_proc)
        .is_none_or(|check| check(world, aura, event))
}

pub(super) fn run_periodic(world: &mut World, unit: Entity, aura: &AuraInstance, effect_index: u8) {
    if let Some(run) = script(world, aura.spell_id).and_then(|script| script.on_periodic) {
        run(world, unit, aura, effect_index);
    }
}

const ASPECT_OF_THE_TURTLE: u32 = 186_265;
const TURTLE_PACIFY: u32 = 205_769;
const BARKSKIN: u32 = 22_812;
const BRAMBLES: u32 = 203_953;
const BRAMBLES_DAMAGE: u32 = 213_709;
const DRAIN_SOUL: u32 = 198_590;
const DRAIN_SOUL_ENERGIZE: u32 = 205_292;
const SURVIVAL_INSTINCTS: u32 = 61336;
const SURVIVAL_INSTINCTS_BUFF: u32 = 50322;
const BREATH_OF_FIRE_BURN: u32 = 123725;
const LASHING_FLAMES: u32 = 334168;
const RAKE_BLEED: u32 = 155722;
const UNSTABLE_AFFLICTION: u32 = 30108;
const UNSTABLE_AFFLICTION_BLEED: u32 = 233490;
const PROTECTIVE_GROWTH: u32 = 433_748;
const REGROWTH: u32 = 8_936;
const LINGERING_HEALING: u32 = 231_040;
const FOUNT_OF_STRENGTH: u32 = 441_675;
const FRENZIED_REGENERATION: u32 = 22_842;
const YSERAS_GIFT: u32 = 145_108;
const YSERAS_GIFT_SELF: u32 = 145_109;
const YSERAS_GIFT_GROUP: u32 = 145_110;
const POWER_STRIKE: u32 = 121_817;
const POWER_STRIKE_PROC: u32 = 129_914;
const POWER_STRIKE_ENERGIZE: u32 = 121_283;
const CRITICAL_CHAOS: u32 = 320_413;
const KNOW_YOUR_ENEMY: u32 = 388_118;
const PENETRATING_SHOTS: u32 = 459_783;
const TIGER_DASH: u32 = 252_216;
const BLOODSURGE: u32 = 384_361;
const MONSTER_RISING: u32 = 452_414;
const MONSTER_RISING_AGILITY: u32 = 452_550;
const METAMORPHOSIS_HAVOC: u32 = 162_264;
const METAMORPHOSIS_VENGEANCE: u32 = 187_827;
const METAMORPHOSIS_DEVOURER: u32 = 1_217_607;
const ENDURING_TORMENT: u32 = 452_410;
const ENDURING_TORMENT_BUFF: u32 = 453_314;
const BLOODSURGE_ENERGIZE: u32 = 384_362;
const REND_AURA: u32 = 388_539;
/// `Cell::VisitAllObjects(caster, worker, 50.0f)`.
const BLOODSURGE_RADIUS: f32 = 50.0;
const ROGUE_STEALTH: u32 = 1_784;
const ROGUE_SANCTUARY: u32 = 98_877;
const ROGUE_STEALTH_AURA: u32 = 158_185;
const ROGUE_STEALTH_SHAPESHIFT: u32 = 158_188;

/// Native441675s3: Frenzied Regeneration also increases your maximum health.
pub(super) fn fount_max_health_pct(unit: Entity, auras: &super::ActiveAuras) -> f32 {
    let regenerating = auras
        .iter()
        .any(|aura| aura.spell_id == FRENZIED_REGENERATION && aura.caster == unit);
    if !regenerating {
        return 0.0;
    }
    auras
        .iter()
        .find(|aura| aura.spell_id == FOUNT_OF_STRENGTH && aura.caster == unit)
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 2))
        .map_or(0.0, |effect| effect.amount)
}

/// Native231040s2 extends Regrowth only "when cast on yourself"; s1 is the
/// independently masked Rejuvenation duration modifier handled by spell_mods.
pub(super) fn self_regrowth_duration_bonus(
    world: &World,
    application: &super::AuraApplication,
) -> f32 {
    if application.spell_id != REGROWTH || application.caster != application.target {
        return 0.0;
    }
    world
        .get::<super::ActiveAuras>(application.caster)
        .and_then(|auras| {
            auras.iter().find(|aura| {
                aura.spell_id == LINGERING_HEALING && aura.caster == application.caster
            })
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 1))
        .map_or(0.0, |effect| effect.amount)
}

/// Native433748s1: "Your Regrowth protects you ... while your Regrowth is on you."
/// Fold into the recipient's shared school multipliers so spells and melee agree.
pub(super) fn protective_growth_multiplier(unit: Entity, auras: &super::ActiveAuras) -> f32 {
    let own_regrowth = auras
        .iter()
        .any(|aura| aura.spell_id == REGROWTH && aura.caster == unit);
    if !own_regrowth {
        return 1.0;
    }
    let talent = auras
        .iter()
        .find(|aura| aura.spell_id == PROTECTIVE_GROWTH && aura.caster == unit);
    let amount = talent
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    1.0 - amount / 100.0
}

///60103/334046/334168 explicitly limit this vulnerability to Flame Shock.
///Other271 masks do not establish a universal filter:208086's description
///instead grants all owner damage. Keep that existing base contract intact.
pub(super) fn affects_incoming_spell(
    aura: &AuraInstance,
    effect: &super::AuraEffectState,
    spell: &crate::spell_info::SpellInfo,
) -> bool {
    aura.spell_id != LASHING_FLAMES || super::spell_mods::class_mask_affects(aura, effect, spell)
}

///115181 names123725s2 as reduced damage dealt to the casting Monk.
///Consume that named field for this spell only; do not redefine generic269.
pub(super) fn outgoing_to_owner_multiplier(world: &World, attacker: Entity, victim: Entity) -> f32 {
    world
        .get::<super::ActiveAuras>(attacker)
        .map_or(1.0, |auras| {
            auras
                .iter()
                .filter(|aura| aura.spell_id == BREATH_OF_FIRE_BURN && aura.caster == victim)
                .flat_map(|aura| &aura.effects)
                .filter(|effect| effect.index == 1)
                .map(|effect| 1.0 + effect.amount / 100.0)
                .product()
        })
}

pub(super) fn consumes_named_field(spell_id: u32, effect_index: u8) -> bool {
    super::augmentation::consumes_field(spell_id, effect_index)
        || super::ebon_might::consumes_field(spell_id, effect_index)
        || (spell_id == super::agony::SUDDEN_ONSET && effect_index == 1)
        || (spell_id == crate::spell_cast::blooming_infusion::BLOOMING_INFUSION
            && effect_index == 0)
        || (spell_id == crate::spell_cast::control_dream::CONTROL_OF_THE_DREAM && effect_index == 0)
        || (spell_id == PROTECTIVE_GROWTH && effect_index == 0)
        || (spell_id == LINGERING_HEALING && effect_index == 1)
        || (spell_id == FOUNT_OF_STRENGTH && effect_index == 2)
        || (spell_id == 431442 && matches!(effect_index, 0..=2))
        || (spell_id == 1260647 && effect_index == 0)
        || (spell_id == BREATH_OF_FIRE_BURN && effect_index == 1)
        || (spell_id == 740 && effect_index == 5)
        || (spell_id == 190456 && matches!(effect_index, 1 | 3))
        || (spell_id == 31850 && effect_index == 1)
        || (spell_id == 47788 && effect_index == 2)
        || (spell_id == crate::spell_cast::scripts::prayer_of_mending::WARD && effect_index == 0)
        || (spell_id == 288733 && effect_index == 1)
}

/// Mainline740s6 supplies the caster's Tranquility shield cap.
pub(super) fn absorb_amount(
    world: &World,
    caster: Entity,
    spell: &crate::spell_info::SpellInfo,
    effect: &crate::spell_info::SpellEffect,
) -> Option<f32> {
    //190456's local tooltip explicitly includes (1 + $@versadmg) in its capacity.
    if (spell.id, effect.index) == (190456, 0) {
        let amount = crate::spell_cast::effects::scaled_amount(world, caster, spell, effect);
        return Some(amount * crate::spell_cast::effects::versatility_multiplier(world, caster));
    }
    let percent_index = match (spell.id, effect.index) {
        (740, 4) => 5,
        //122470s3: shield capacity is50% max health. Redirect s4 remains separate.
        (122470, 1) => 2,
        _ => return None,
    };
    let percent = spell
        .effects
        .iter()
        .find(|e| e.index == percent_index)
        .expect("named shield percentage")
        .base_points;
    let health = world
        .get::<shared::components::Health>(caster)
        .expect("Tranquility caster health");
    Some(health.max * percent / 100.0)
}

///1822's stealth bonus also applies to its named155722 bleed, frozen on apply.
///Do not re-read stealth at each tick or modify arbitrary periodic auras.
pub(super) fn snapshot_periodic_amount(
    world: &World,
    caster: Entity,
    spell: &crate::spell_info::SpellInfo,
    effect: &crate::spell_info::SpellEffect,
    amount: f32,
) -> f32 {
    if spell.id == RAKE_BLEED && effect.index == 0 {
        crate::spell_cast::scripts::rake_bleed_snapshot(world, caster, amount)
    } else {
        let amount =
            crate::spell_cast::scripts::dispersion_tick(world, caster, spell, effect.index, amount);
        crate::spell_cast::scripts::snapshot_combustion(world, caster, spell, effect.index, amount)
    }
}

///30108s3 grants one owner-specific vulnerability, not one multiplier per stack.
pub(super) fn unstable_affliction_multiplier(
    world: &World,
    attacker: Entity,
    target: Entity,
) -> f32 {
    let owned = world
        .get::<super::ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == UNSTABLE_AFFLICTION_BLEED && aura.caster == attacker)
        });
    if owned {
        1.0 + crate::spell_cast::scripts::script_effect_value(
            world,
            attacker,
            UNSTABLE_AFFLICTION,
            2,
        ) / 100.0
    } else {
        1.0
    }
}

/// Tranquility's channel owns its aura lifetime, not its shield capacity.
/// Removing the whole aura on depletion would also stop its healing ticks.
pub(super) fn absorb_ends_aura(spell_id: u32) -> bool {
    spell_id != 740
}

///321068 explicitly gates187827s4's extra Shear/Fracture Fury modifier.
pub(super) fn effect_enabled(
    world: &World,
    caster: Entity,
    spell_id: u32,
    effect_index: u8,
) -> bool {
    spell_id != 187827 || effect_index != 3 || super::has_aura(world, caster, 321068)
}

pub(super) fn register_class_scripts(mut scripts: ResMut<AuraScripts>) {
    crate::spell_cast::scripts::prayer_of_mending::register(&mut scripts);
    super::earthen_rage::register(&mut scripts);
    super::spirit_wolf::register(&mut scripts);
    scripts.0.insert(
        super::fel_devastation::FEL_DEVASTATION,
        AuraScript {
            on_periodic: Some(super::fel_devastation::cast_periodic_heal),
            ..Default::default()
        },
    );
    scripts.0.insert(
        crate::spell_cast::blooming_infusion::BLOOMING_INFUSION,
        AuraScript {
            on_apply: Some(crate::spell_cast::blooming_infusion::start_cast_count),
            on_remove: Some(crate::spell_cast::blooming_infusion::stop_cast_count),
            ..Default::default()
        },
    );
    scripts.0.insert(
        crate::spell_cast::control_dream::CONTROL_OF_THE_DREAM,
        AuraScript {
            on_apply: Some(crate::spell_cast::control_dream::start_ready_credit),
            on_remove: Some(crate::spell_cast::control_dream::stop_ready_credit),
            ..Default::default()
        },
    );
    scripts.0.insert(
        80240,
        AuraScript {
            on_apply: Some(crate::spell_cast::scripts::replace_havoc),
            ..Default::default()
        },
    );
    scripts.0.insert(
        crate::spell_cast::scripts::LIFEBLOOM,
        AuraScript {
            on_apply: Some(crate::spell_cast::scripts::replace_lifebloom),
            on_remove: Some(crate::spell_cast::scripts::bloom_lifebloom),
            ..Default::default()
        },
    );
    scripts.0.insert(
        crate::spell_cast::scripts::CENARION_WARD,
        AuraScript {
            on_proc: Some(crate::spell_cast::scripts::heal_cenarion_ward),
            ..Default::default()
        },
    );
    scripts.0.insert(
        53563,
        AuraScript {
            on_apply: Some(crate::spell_cast::scripts::replace_beacon),
            ..Default::default()
        },
    );
    scripts.0.insert(
        SURVIVAL_INSTINCTS,
        AuraScript {
            on_apply: Some(apply_survival_instincts),
            on_remove: Some(remove_survival_instincts),
            ..Default::default()
        },
    );
    scripts.0.insert(
        DRAIN_SOUL,
        AuraScript {
            on_remove: Some(drain_soul_shard_on_death),
            ..Default::default()
        },
    );
    scripts.0.insert(
        BARKSKIN,
        AuraScript {
            on_periodic: Some(barkskin_brambles),
            ..Default::default()
        },
    );
    scripts.0.insert(
        BLOODSURGE,
        AuraScript {
            on_periodic: Some(roll_bloodsurge),
            ..Default::default()
        },
    );
    scripts.0.insert(
        ENDURING_TORMENT,
        AuraScript {
            on_periodic: Some(toggle_enduring_torment),
            ..Default::default()
        },
    );
    scripts.0.insert(
        super::charred_warblades::CHARRED_WARBLADES,
        AuraScript {
            check_proc: Some(super::charred_warblades::check_fire_damage),
            on_damage_proc: Some(super::charred_warblades::accumulate_heal),
            on_periodic: Some(super::charred_warblades::heal_on_tick),
            on_remove: Some(super::charred_warblades::discard_heal),
            ..Default::default()
        },
    );
    scripts.0.insert(
        YSERAS_GIFT,
        AuraScript {
            on_periodic: Some(heal_yseras_gift),
            ..Default::default()
        },
    );
    scripts.0.insert(
        POWER_STRIKE_PROC,
        AuraScript {
            on_proc: Some(energize_power_strike),
            ..Default::default()
        },
    );
    scripts.0.insert(
        POWER_STRIKE,
        AuraScript {
            on_periodic: Some(cast_power_strike_proc),
            ..Default::default()
        },
    );
    for spell in [CRITICAL_CHAOS, KNOW_YOUR_ENEMY, PENETRATING_SHOTS] {
        scripts.0.insert(
            spell,
            AuraScript {
                on_apply: Some(update_crit_scaled_bonus),
                on_periodic: Some(recalculate_crit_scaled_bonus),
                ..Default::default()
            },
        );
    }
    scripts.0.insert(
        MONSTER_RISING,
        AuraScript {
            on_periodic: Some(toggle_monster_rising),
            ..Default::default()
        },
    );
    scripts.0.insert(
        super::ebon_might::SELF_BUFF,
        AuraScript {
            on_periodic: Some(super::ebon_might::refresh_owned),
            ..Default::default()
        },
    );
    scripts.0.insert(
        super::augmentation::SHIFTING_SANDS,
        AuraScript {
            on_periodic: Some(super::augmentation::refresh_sands),
            ..Default::default()
        },
    );
    scripts.0.insert(
        TIGER_DASH,
        AuraScript {
            on_periodic: Some(decay_tiger_dash),
            ..Default::default()
        },
    );
    scripts.0.insert(
        crate::spell_cast::scripts::RAPID_FIRE,
        AuraScript {
            on_periodic: Some(crate::spell_cast::scripts::cast_rapid_fire_tick),
            ..Default::default()
        },
    );
    scripts.0.insert(
        crate::spell_cast::scripts::HAUNT,
        AuraScript {
            on_remove: Some(crate::spell_cast::scripts::reset_haunt_on_death),
            ..Default::default()
        },
    );
    scripts.0.insert(
        ASPECT_OF_THE_TURTLE,
        AuraScript {
            on_apply: Some(apply_turtle_pacify),
            on_remove: Some(remove_turtle_pacify),
            ..Default::default()
        },
    );
    scripts.0.insert(
        crate::pvp::arena_cage::REACTION_TRIGGER,
        AuraScript {
            on_proc: Some(crate::pvp::arena_cage::reaction_dead),
            ..Default::default()
        },
    );
    scripts.0.insert(
        crate::pvp::arena_cage::WARNING,
        AuraScript {
            on_remove: Some(crate::pvp::arena_cage::warning_removed),
            ..Default::default()
        },
    );
    scripts.0.insert(
        ROGUE_STEALTH,
        AuraScript {
            on_apply: Some(apply_rogue_stealth),
            on_remove: Some(remove_rogue_stealth),
            ..Default::default()
        },
    );
}

/// TC spell_rogue.cpp spell_rog_stealth: 1784's own rows carry no MOD_STEALTH;
/// its effect1 dummy casts Sanctuary and the 158185/158188 stealth children.
fn apply_rogue_stealth(world: &mut World, unit: Entity, _aura: &AuraInstance) {
    for spell_id in [
        ROGUE_SANCTUARY,
        ROGUE_STEALTH_AURA,
        ROGUE_STEALTH_SHAPESHIFT,
    ] {
        crate::spell_cast::cast_triggered(world, unit, spell_id, Some(unit), 1);
    }
}

fn remove_rogue_stealth(
    world: &mut World,
    unit: Entity,
    _aura: &AuraInstance,
    reason: super::RemoveReason,
) {
    super::remove_auras(world, unit, reason, |aura| {
        matches!(aura.spell_id, ROGUE_STEALTH_AURA | ROGUE_STEALTH_SHAPESHIFT)
    });
}

/// TC a352b1fa spell_druid.cpp:297-316, spell_dru_barkskin. Effect 2's
/// PERIODIC_DUMMY ticks only deal damage when Brambles is present. Catalog
/// 203953's tooltip references 213709s1 and 22812t3; the child supplies the
/// AoE targeting and 0.075 AP coefficient, the parent the 1000ms cadence.
fn barkskin_brambles(world: &mut World, unit: Entity, _aura: &AuraInstance, effect_index: u8) {
    if effect_index == 2 && super::has_aura(world, unit, BRAMBLES) {
        crate::spell_cast::cast_triggered(world, unit, BRAMBLES_DAMAGE, Some(unit), 1);
    }
}

/// TC a352b1fa spell_druid.cpp:2827-2843, spell_dru_tiger_dash_aura: each effect1
/// PERIODIC_DUMMY tick lowers effect0's run-speed amount by effect1's amount.
fn decay_tiger_dash(world: &mut World, unit: Entity, aura: &AuraInstance, effect_index: u8) {
    if effect_index != 1 {
        return;
    }
    let Some(decay) = aura.effects.iter().find(|effect| effect.index == 1) else {
        return;
    };
    let decay = decay.amount;
    {
        let Some(mut auras) = world.get_mut::<super::ActiveAuras>(unit) else {
            return;
        };
        let Some(speed) = auras
            .get_mut(aura.instance_id)
            .and_then(|live| live.effects.iter_mut().find(|effect| effect.index == 0))
        else {
            return;
        };
        speed.amount -= decay;
    }
    super::auras_changed(world, unit);
    crate::spell_cast::mark_dirty(world, unit);
}

/// TC a352b1fa spell_druid.cpp:3072-3097, spell_dru_yseras_gift.
fn heal_yseras_gift(world: &mut World, unit: Entity, aura: &AuraInstance, effect_index: u8) {
    if effect_index != 0 {
        return;
    }
    let Some(percent) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return;
    };
    let health = world
        .get::<shared::components::Health>(unit)
        .expect("living aura unit health");
    let amount = (health.max * percent.amount / 100.0).floor();
    let heal = if health.current < health.max {
        YSERAS_GIFT_SELF
    } else {
        YSERAS_GIFT_GROUP
    };
    crate::spell_cast::cast_triggered_with_effect0_amount(world, unit, heal, Some(unit), 1, amount);
}

/// TC a352b1fa spell_monk.cpp:318-333, spell_monk_power_strike_proc.
fn energize_power_strike(
    world: &mut World,
    unit: Entity,
    _aura: &AuraInstance,
    _other: Option<Entity>,
) {
    crate::spell_cast::cast_triggered(world, unit, POWER_STRIKE_ENERGIZE, Some(unit), 1);
}

/// TC a352b1fa spell_monk_power_strike_periodic: only effect0 casts 129914.
fn cast_power_strike_proc(world: &mut World, unit: Entity, _aura: &AuraInstance, effect_index: u8) {
    if effect_index == 0 {
        crate::spell_cast::cast_triggered(world, unit, POWER_STRIKE_PROC, Some(unit), 1);
    }
}

/// TC a352b1fa spell_dh.cpp:632-655/1571-1593, spell_hunter.cpp:869-891:
/// effect1 recalculates effect0's crit-scaled proc or critical-damage bonus.
fn recalculate_crit_scaled_bonus(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    effect_index: u8,
) {
    if effect_index == 1 {
        update_crit_scaled_bonus(world, unit, aura);
    }
}

fn update_crit_scaled_bonus(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let Some(percent) = aura.effects.iter().find(|effect| effect.index == 1) else {
        return;
    };
    let crit_pct = world
        .get::<crate::stats::CharacterStats>(unit)
        .map_or(0.0, |stats| stats.crit_pct);
    let amount = crit_pct * percent.amount / 100.0;
    let Some(bonus) = world
        .get_mut::<super::ActiveAuras>(unit)
        .and_then(|auras| auras.into_inner().get_mut(aura.instance_id))
        .and_then(|live| live.effects.iter_mut().find(|effect| effect.index == 0))
    else {
        return;
    };
    bonus.amount = amount;
    super::auras_changed(world, unit);
}

/// TC a352b1fa spell_dh.cpp:1659-1689, spell_dh_monster_rising.
fn toggle_monster_rising(world: &mut World, unit: Entity, _aura: &AuraInstance, effect_index: u8) {
    if effect_index == 0 {
        toggle_outside_transforms(
            world,
            unit,
            MONSTER_RISING_AGILITY,
            &[METAMORPHOSIS_HAVOC, METAMORPHOSIS_VENGEANCE],
        );
    }
}

/// TC a352b1fa spell_dh.cpp:1086-1116, spell_dh_enduring_torment.
fn toggle_enduring_torment(
    world: &mut World,
    unit: Entity,
    _aura: &AuraInstance,
    effect_index: u8,
) {
    if effect_index == 0 {
        toggle_outside_transforms(
            world,
            unit,
            ENDURING_TORMENT_BUFF,
            &[METAMORPHOSIS_HAVOC, METAMORPHOSIS_DEVOURER],
        );
    }
}

/// Outside every `transforms` aura each tick ensures `buff`; inside one it removes it.
fn toggle_outside_transforms(world: &mut World, unit: Entity, buff: u32, transforms: &[u32]) {
    let transformed = transforms
        .iter()
        .any(|&transform| super::has_aura(world, unit, transform));
    let buffed = super::has_aura(world, unit, buff);
    if transformed && buffed {
        super::remove_auras(world, unit, super::RemoveReason::Cancelled, |aura| {
            aura.spell_id == buff
        });
    } else if !transformed && !buffed {
        crate::spell_cast::cast_triggered(world, unit, buff, Some(unit), 1);
    }
}

/// TC a352b1fa spell_warrior.cpp:282-316, spell_warr_bloodsurge: chance
/// sqrt(units carrying the warrior's Rend effect0) × effect0 amount per tick. TC
/// visits the grid cells covering 50 yd without a per-unit distance check; this
/// counts units within 50 yd on the warrior's map.
fn roll_bloodsurge(world: &mut World, unit: Entity, aura: &AuraInstance, effect_index: u8) {
    if effect_index != 0 {
        return;
    }
    let Some(amount) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return;
    };
    let rends = count_owned_rends(world, unit) as f32;
    let chance = rends.sqrt() * amount.amount;
    if rand::Rng::gen_range(&mut rand::thread_rng(), 0.0..100.0) < chance {
        crate::spell_cast::cast_triggered(world, unit, BLOODSURGE_ENERGIZE, Some(unit), 1);
    }
}

fn count_owned_rends(world: &mut World, warrior: Entity) -> usize {
    let Some(center) = world.get::<shared::components::Position>(warrior).copied() else {
        return 0;
    };
    let mut units = world.query::<(Entity, &shared::components::Position, &super::ActiveAuras)>();
    let candidates: Vec<Entity> = units
        .iter(world)
        .filter(|(_, at, _)| {
            let (dx, dy, dz) = (at.x - center.x, at.y - center.y, at.z - center.z);
            dx * dx + dy * dy + dz * dz <= BLOODSURGE_RADIUS * BLOODSURGE_RADIUS
        })
        .filter(|(_, _, auras)| {
            auras.iter().any(|aura| {
                aura.spell_id == REND_AURA
                    && aura.caster == warrior
                    && aura.effects.iter().any(|effect| effect.index == 0)
            })
        })
        .map(|(entity, ..)| entity)
        .collect();
    candidates
        .into_iter()
        .filter(|&target| crate::spell_cast::targets::same_map(world, warrior, target))
        .count()
}

/// TrinityCore a352b1fa spell_hun_aspect_of_the_turtle applies catalog 205769
/// while the parent is present, then removes it (spell_hunter.cpp:198-220).
fn apply_turtle_pacify(world: &mut World, unit: Entity, _aura: &AuraInstance) {
    super::apply_aura(
        world,
        &super::AuraApplication {
            caster: unit,
            target: unit,
            spell_id: TURTLE_PACIFY,
            effect_mask: u32::MAX,
            source: super::AuraSource::Cast,
        },
    );
}

/// TC a352b1fa spell_warlock.cpp:831-848: only death removal awards
/// 205292 (10 raw Soul Shards = one whole shard). Expiry/cancellation do not.
fn drain_soul_shard_on_death(
    world: &mut World,
    _unit: Entity,
    aura: &AuraInstance,
    reason: super::RemoveReason,
) {
    if reason == super::RemoveReason::Death {
        crate::spell_cast::cast_triggered(
            world,
            aura.caster,
            DRAIN_SOUL_ENERGIZE,
            Some(aura.caster),
            1,
        );
    }
}

/// TDB/TC spell_dru_survival_instincts:61336 effect0 owns50322's
/// base damage reduction for the parent's lifetime. Exclude unknown Dummy2.
fn apply_survival_instincts(world: &mut World, unit: Entity, _aura: &AuraInstance) {
    super::apply_aura(
        world,
        &super::AuraApplication {
            caster: unit,
            target: unit,
            spell_id: SURVIVAL_INSTINCTS_BUFF,
            effect_mask: 1,
            source: super::AuraSource::Cast,
        },
    );
}

fn remove_survival_instincts(
    world: &mut World,
    unit: Entity,
    _aura: &AuraInstance,
    reason: super::RemoveReason,
) {
    super::remove_auras(world, unit, reason, |aura| {
        aura.spell_id == SURVIVAL_INSTINCTS_BUFF && aura.caster == unit
    });
}

fn remove_turtle_pacify(
    world: &mut World,
    unit: Entity,
    _aura: &AuraInstance,
    _reason: super::RemoveReason,
) {
    super::remove_auras(world, unit, super::RemoveReason::Cancelled, |aura| {
        aura.spell_id == TURTLE_PACIFY && aura.caster == unit
    });
}
