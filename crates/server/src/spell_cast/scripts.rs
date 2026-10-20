//! Spell scripts (TrinityCore `SpellScript`) keyed by spell ID: behaviour that the
//! spell's DB2 effects do not carry.

mod batch10;
mod batch11;
mod batch12;
mod batch13;
mod batch14;
mod batch15;
mod batch16;
mod batch17;
mod batch19;
mod batch20;
mod batch21;
mod batch22;
mod batch23;
mod batch3;
mod batch4;
mod batch5;
mod batch6;
mod batch7;
mod batch8;
mod batch9;
mod chrono_flame;
mod clearcasting;
pub(crate) use clearcasting::{OMEN, check_omen_proc};
mod germination;
pub(super) use germination::redirect_rejuvenation;
pub(crate) mod prayer_of_mending;
mod scourge_strike;
mod soul_of_the_forest;
mod swiftmend;
pub(super) use soul_of_the_forest::direct_multiplier as soul_of_the_forest_direct_multiplier;
pub(crate) use soul_of_the_forest::snapshot_amount as snapshot_soul_of_the_forest;
mod touch_of_death;
pub(crate) use chrono_flame::record_health_change;

pub(super) use batch9::update_ground_pulses;
pub(crate) use batch9::{ground_pulse_radius, hostile_ground_fields, owned_ground_fields};
pub(crate) use batch19::{copy_havoc, dispersion_tick, on_dispelled, replace_havoc};
pub(crate) use batch20::proc_charges;
/// OnEffectHit runs once per effect without replacing generic or aura dispatch.
pub(super) fn on_cast_effect(spell: u32, index: u8) -> Option<super::effects::CastEffectHandler> {
    match (spell, index) {
        (crate::scripted_ai::azure_vault::ERUPTING_FISSURE, 0) => {
            Some(crate::scripted_ai::azure_vault::cast_fissure_selector)
        }
        (crate::scripted_ai::azure_vault::EXPLOSIVE_BRAND, 0) => {
            Some(crate::scripted_ai::azure_vault::cast_brand_child)
        }
        (crate::scripted_ai::azure_vault::LEY_LINE_SPROUTS, 0) => {
            Some(crate::scripted_ai::azure_vault::cast_sprout_missiles)
        }
        _ => None,
    }
}

pub(super) fn cast_effect(spell: u32, index: u8) -> Option<super::effects::CastEffectHandler> {
    if spell == super::mobile_banking::MOBILE_BANKING && index == 0 {
        return Some(super::mobile_banking::summon_chest);
    }
    if spell == 96538 && index == 0 {
        return Some(crate::pvp::arena_dalaran::cast_pipe_search);
    }
    if (spell, index) == (crate::scripted_ai::murder_row::SPELL_FEL_CRAZED, 1) {
        return Some(crate::scripted_ai::murder_row::cast_initial_felshield);
    }
    super::toy_scripts::cast_effect(spell, index)
        .or_else(|| batch14::cast_effect(spell, index))
        .or_else(|| batch9::cast_effect(spell, index))
}
/// Native per-unit scripts replace the default effect handler before dispatch.
pub(super) fn effect_handler(spell: u32, index: u8) -> Option<super::effects::EffectHandler> {
    prayer_of_mending::effect_handler(spell, index)
        .or_else(|| batch13::effect_handler(spell, index))
}

pub(super) use batch12::empower_gargoyles;
pub(super) use batch13::refund_execute;
pub(crate) use batch13::{HAUNT, RAPID_FIRE, cast_rapid_fire_tick, reset_haunt_on_death};
pub(crate) use batch14::{DEATHMARK, EYE_BEAM, cast_eye_beam_tick, deathmark_energy};
pub(super) use batch15::heal_beacon;
pub(crate) use batch15::{replace_beacon, snapshot_combustion};
pub(super) use batch16::{blast_cast_time, blast_multiplier, mana_cost};
pub(super) use batch17::bloom_lifebloom_before_refresh;
pub(crate) use batch17::{
    CENARION_WARD, LIFEBLOOM, bloom_lifebloom, heal_cenarion_ward, replace_lifebloom,
};
pub(super) use batch21::berserk_combo_bonus;
pub(super) use batch22::healing_multiplier;

pub(super) fn effect_enabled(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    batch19::effect_enabled(world, caster, spell, index)
        && batch11::effect_enabled(world, caster, spell, index)
        && batch13::effect_enabled(spell, index)
        && batch15::effect_enabled(world, caster, spell, index)
        && batch23::effect_enabled(world, caster, spell, index)
}

use bevy::prelude::*;

use super::targets::{self, Relation, relation_matches};
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
const ENRAGED_REGENERATION: u32 = 184_364;
const INVIGORATING_FURY: u32 = 383_468;
const INVIGORATING_FURY_HEAL: u32 = 385_174;
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
const DEMON_SPIKES: u32 = 203_720;
const DEMON_SPIKES_AURA: u32 = 203_819;
const DEFLECTING_SPIKES: u32 = 321_028;
const SKULL_BASH: u32 = 106_839;
const SKULL_BASH_CHARGE: u32 = 221_514;
const SKULL_BASH_INTERRUPT: u32 = 93_985;

/// `SpellEffectFn` on `OnEffectHitTarget`: runs per unit the effect selects,
/// after the effect's handler. A DUMMY effect with a hook selects targets.
pub type EffectHitHook = fn(&mut World, Entity, Entity, u8);

/// Source OnDestinationTargetSelect hooks, keyed by spell/effect/target selector.
pub(super) fn relocate_destination(
    spell_id: u32,
    effect: &crate::spell_info::SpellEffect,
    dest: &mut super::targets::Dest,
) {
    if spell_id == crate::scripted_ai::murder_row::SPELL_MIRROR_IMAGE
        && effect.index == 0
        && effect.implicit_targets.contains(&87)
    {
        crate::scripted_ai::murder_row::relocate_mirror_destination(dest);
    }
    if spell_id == crate::scripted_ai::ruby_life_pools::STORM_INFUSION
        && effect.index == 1
        && effect.implicit_targets.contains(&87)
    {
        crate::scripted_ai::ruby_life_pools::relocate_storm_destination(dest);
    }
}

pub fn on_effect_hit(spell_id: u32, effect_index: u8) -> Option<EffectHitHook> {
    match (spell_id, effect_index) {
        (20_538, 0) => Some(hate_to_zero),
        (30_693, 0) => Some(crate::scripted_ai::hellfire_ramparts::call_nazan),
        (30_691 | 32_491 | 33_793 | 33_794 | 34_653 | 36_920, 1) => {
            Some(crate::scripted_ai::hellfire_ramparts::liquid_fire::fireball_hit)
        }
        (30_926 | 36_921, 1) => Some(crate::scripted_ai::hellfire_ramparts::liquid_fire::cone_hit),
        (super::omor_orbital::ORBITAL_TARGET, 0) => Some(super::omor_orbital::cast_hit),
        (crate::scripted_ai::azure_vault::FROST_BOMB_CAST, 0) => {
            Some(crate::scripted_ai::azure_vault::apply_frost_bomb)
        }
        (crate::scripted_ai::azure_vault::ABSOLUTE_ZERO_CAST, 0) => {
            Some(crate::scripted_ai::azure_vault::cast_absolute_zero_outbound_jump)
        }
        (crate::scripted_ai::azure_vault::ACTIVATE_VAULT_RUNE, 0) => {
            Some(crate::scripted_ai::azure_vault::activate_vault_rune)
        }
        (crate::scripted_ai::azure_vault::ABSOLUTE_ZERO_DAMAGE, 1) => {
            Some(crate::scripted_ai::azure_vault::strip_vault_rune)
        }
        (crate::seething_parachute::PARACHUTE, 1) => {
            Some(crate::seething_parachute::reset_fall_information)
        }
        (crate::scripted_ai::murder_row::SPELL_FEL_NOVA_SELECTOR, 0) => {
            Some(crate::scripted_ai::murder_row::cast_blink_at_selected_target)
        }
        (crate::scripted_ai::ragefire_chasm::SHADOW_STORM_DUMMY, 0) => {
            Some(crate::scripted_ai::ragefire_chasm::cast_shadow_storm_effect)
        }
        (124491, 0) => Some(start_switch_capture),
        (crate::pvp::arena_cage::REACTION_LOW_HEALTH, 0) => {
            Some(crate::pvp::arena_cage::reaction_low_health)
        }
        (crate::pvp::arena_cage::WARNING_TELEPORT, 0) => {
            Some(crate::pvp::arena_cage::warning_teleport)
        }
        (97372, 0) => Some(crate::pvp::change_track),
        (batch17::CONFLAGRATE, 0) => Some(batch17::conflagrate),
        (batch17::SOUL_FIRE, 0) => Some(batch17::apply_soul_fire_dot),
        (batch17::BLOOD_BOIL, 0) => Some(batch17::apply_blood_plague),
        (batch17::MARROWREND, 1) => Some(batch17::grant_bone_shield),
        (batch16::RET_JUDGMENT | batch16::HOLY_JUDGMENT, 0) => Some(batch16::energize_judgment),
        (batch15::RISING_SUN_DAMAGE, 0) => Some(batch15::wound_rising_sun),
        (batch14::ORB_DAMAGE, 0) => Some(batch14::slow_orb),
        (batch14::BLIZZARD_DAMAGE, 0) => Some(batch14::slow_blizzard),
        (batch14::SURVIVAL_COMMAND, 0) => Some(batch14::command_pet),
        (KILL_COMMAND, 0) => Some(batch14::command_beast),
        (batch14::WILDFIRE_BOMB, 2) => Some(batch14::bomb_primary),
        (batch14::CONSUME, 0) => Some(batch14::consume_fury),
        (batch13::REND, 0) => Some(batch13::apply_rend_bleed),
        (batch13::ASPECT_WILD, 0) => Some(batch13::cast_wild_primary),
        (batch12::FERAL_SPIRIT, 0) => Some(batch12::summon_wolves),
        (batch12::BUTCHERY, 0) => Some(batch12::butchery_bleed),
        (batch11::VOID_VOLLEY, 0) => Some(batch11::volley),
        (batch11::VOIDFORM, 0) => Some(batch11::initial_volley),
        (batch11::HAMMER_RIGHTEOUS, 0) => Some(batch11::hammer_wave),
        (batch3::SHRED, 2) => Some(batch11::shred_points),
        (batch11::THRASH, 0) => Some(batch11::thrash_bleed),
        (batch10::FLURRY_DAMAGE, 0) => Some(batch10::energize_flurry),
        (batch10::FERAL_LUNGE, 0) => Some(batch10::bite_lunge),
        (batch10::PROTECTION_JUDGMENT, 0) => Some(batch10::energize_judgment),
        (batch10::HAND_OF_GULDAN, 0) => Some(batch10::launch_guldan_meteor),
        (batch10::LIGHTNING_BOLT, 0) => Some(batch10::energize_lightning_bolt),
        (batch9::WINTER_DAMAGE, 0) => Some(batch9::slow_winter_hit),
        (batch9::SCOURGE_STRIKE, 0) => Some(scourge_strike::erupt_and_apply_missing),
        (batch9::RADIANCE, 1) => Some(batch9::apply_radiance_atonement),
        (batch8::REAP, 0) => Some(batch8::reap),
        (batch8::PROVOKE, 0) => Some(batch8::provoke),
        (batch8::BREATH_OF_FIRE, 0) => Some(batch8::breath_of_fire),
        (batch8::FAN_OF_KNIVES, 0) => Some(batch8::fan_of_knives),
        (batch8::INCINERATE, 1) => Some(batch8::incinerate),
        (batch8::UNSTABLE_AFFLICTION, 0) => Some(batch8::unstable_affliction),
        (batch7::COLOSSUS_SMASH, 0) => Some(batch7::apply_colossus_wound),
        (batch7::HEART_STRIKE, 0) => Some(batch7::energize_heartbreaker),
        (batch7::RAGING_BLOW, 0) => Some(batch7::roll_raging_blow_reset),
        (batch7::WHIRLWIND, 1) => Some(batch7::energize_whirlwind),
        (batch7::RAKE, 0) => Some(batch7::stun_rake),
        (batch6::SUNFIRE, 0) => Some(batch6::sunfire),
        (batch6::TEMPLARS_VERDICT, 0) => Some(batch6::verdict),
        (batch6::ICE_LANCE, 0) => Some(batch6::ice_lance),
        (batch6::SHADOW_WORD_DEATH, 1) => Some(batch6::shadow_word_death),
        (batch5::DEMONS_BITE, 1) => Some(batch5::demons_bite),
        (batch5::CHAOS_STRIKE, 0) => Some(batch5::chaos_strike),
        (batch5::RAPTURE, 0) => Some(batch5::rapture),
        (batch5::SHURIKEN_STORM, 0) => Some(batch5::shuriken_storm),
        (batch5::DEATHS_CARESS, 0) => Some(batch5::deaths_caress),
        (batch5::EPIDEMIC, 0) => Some(batch5::epidemic),
        (batch4::RENEWING_MIST_CAST, 0) => Some(batch4::renewing_mist),
        (batch4::STEADY_SHOT, 0) => Some(batch4::steady_shot),
        (batch4::CHAIN_LIGHTNING, 0) => Some(batch4::chain_lightning),
        (batch4::HOWLING_BLAST, 0) => Some(batch4::howling_blast),
        (batch3::MOONFIRE, 0) => Some(batch3::moonfire),
        (batch3::THUNDER_CLAP, 0) => Some(batch3::thunder_clap),
        (batch3::DEVASTATE, 0) => Some(batch3::devastate),
        (SKULL_BASH, 0) => Some(skull_bash),
        (FORTIFYING_BREW, 0) => Some(fortifying_brew),
        (SHIELD_BLOCK, 0) => Some(shield_block),
        (BLOODTHIRST, 0) => Some(bloodthirst),
        (RAMPAGE, 0) => Some(rampage),
        (HOLY_SHOCK, 0) => Some(holy_shock),
        (PENANCE, 0) => Some(penance),
        (COBRA_SHOT, 1) => Some(cobra_shot),
        (BLACKOUT_KICK, 0) => Some(blackout_kick),
        (crate::auras::augmentation::PRESCIENCE, 1) => {
            Some(crate::auras::augmentation::grant_prescience)
        }
        (PYRE_IMPACT, 0) => Some(pyre),
        (DEATH_COIL, 0) => Some(death_coil),
        (DEMON_SPIKES, 0) => Some(demon_spikes),
        (CHAOS_STRIKE_INITIAL, 0) => Some(chaos_strike_initial),
        (LIVING_FLAME, 0) => Some(living_flame),
        (chrono_flame::CHRONO_FLAMES, 0) => Some(living_flame),
        (LIVING_FLAME_DAMAGE | LIVING_FLAME_HEAL, 0) => Some(chrono_flame::repeat_landed_flame),
        (SWITCH_FLIGHT_STYLE, 0) => Some(switch_flight_style),
        (SWITCH_TO_SKYRIDING, 0) => Some(switch_to_skyriding),
        (SWITCH_TO_STEADY, 0) => Some(switch_to_steady),
        _ => None,
    }
}

/// TC a352b1fa spell_mage.cpp:804-815 overrides Fire Blast's crit chance
/// to 100%; Retail108853 tooltip explicitly renders `${$s1*2}`.
pub fn crit_chance(world: &World, caster: Entity, target: Entity, spell_id: u32) -> Option<f32> {
    match spell_id {
        batch17::REGROWTH => batch17::regrowth_crit(world, caster, target),
        batch14::EYE_BEAM_DAMAGE => batch14::eye_beam_crit(world, caster),
        batch3::SHRED if batch11::shred_stealth_bonus(world, caster) => {
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(spell_id)
                .expect("Shred spell");
            let base = world
                .get::<crate::stats::CharacterStats>(caster)
                .map_or(0.0, |stats| stats.crit_pct);
            Some(
                2.0 * crate::auras::spell_mods::modify(
                    world,
                    caster,
                    spell,
                    crate::auras::SpellModOp::CritChance,
                    base,
                ),
            )
        }
        batch3::SCORCH => batch3::scorch_crit(world, caster, target),
        batch12::GARGOYLE_STRIKE => batch12::gargoyle_crit(world, caster),
        FIRE_BLAST | batch5::CHAOS_BOLT => Some(100.0),
        batch5::LAVA_BURST_DAMAGE => batch5::lava_burst_crit(world, target),
        _ => None,
    }
}

pub(crate) fn rake_bleed_snapshot(world: &World, caster: Entity, amount: f32) -> f32 {
    batch7::rake_initial_damage(world, caster, caster, amount)
}

pub type DamageAmountHook = fn(&World, Entity, Entity, f32) -> f32;

pub fn damage_amount(spell_id: u32) -> Option<DamageAmountHook> {
    if let Some(hook) = batch19::damage_amount(spell_id) {
        return Some(hook);
    }
    match spell_id {
        crate::auras::agony::AGONY => Some(crate::auras::agony::tick_damage),
        touch_of_death::TOUCH_OF_DEATH => Some(touch_of_death::damage),
        batch13::SHADOW_WORD_DEATH => Some(batch13::death_execute),
        batch11::SINISTER_STRIKE => Some(batch11::sinister_damage),
        batch3::SHRED => Some(batch11::shred_damage),
        batch7::RAKE => Some(batch7::rake_initial_damage),
        batch4::MANGLE => Some(batch4::mangle_damage),
        batch5::CHAOS_BOLT | batch5::LAVA_BURST_DAMAGE => Some(batch5::crit_scaled_damage),
        batch5::RAPTURE_DAMAGE => Some(batch5::rapture_damage),
        BACKSTAB => Some(backstab_damage),
        DRAIN_SOUL => Some(drain_soul_damage),
        COBRA_SHOT => Some(cobra_shot_damage),
        _ => None,
    }
}

/// Retail SpellXDescriptionVariables193455 ->331: unconditional $mult
/// = (min(player_level-1,19)*12+112)/340, unity from level20 onward.
fn cobra_shot_damage(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    let level = world
        .get::<shared::components::Level>(caster)
        .expect("class spell caster requires Level")
        .0;
    let multiplier = ((f32::from(level) - 1.0).min(19.0) * 12.0 + 112.0) / 340.0;
    amount * multiplier
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
    // SpellXDescriptionVariables53 ->339 supplies unconditional $mult.
    let level = world
        .get::<shared::components::Level>(caster)
        .expect("class spell caster requires Level")
        .0;
    let early_levels = (f32::from(level) - 10.0).clamp(0.0, 10.0);
    let amount = amount * (early_levels * 11.0 + 260.0) / 370.0;
    let from = super::targets::position(world, target).expect("Backstab target requires position");
    let to = super::targets::position(world, caster).expect("Backstab caster requires position");
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
    match spell_id {
        ENRAGED_REGENERATION => Some(cast_invigorating_fury_heal),
        crate::scripted_ai::azure_vault::ABSOLUTE_ZERO_DAMAGE => {
            Some(crate::scripted_ai::azure_vault::return_after_absolute_zero)
        }
        crate::auras::ebon_might::EBON => Some(crate::auras::ebon_might::cast_self_buff),
        crate::scripted_ai::murder_row::SPELL_BLINK => {
            Some(crate::scripted_ai::murder_row::cast_fel_nova_after_blink)
        }
        batch20::BARBED_SHOT => Some(batch20::grant_barbed_focus),
        batch17::SHADOW_BOLT => Some(batch17::energize_shadow_bolt),
        batch17::SOUL_FIRE => Some(batch17::energize_soul_fire),
        batch16::HOLY_LIGHT => Some(batch16::energize_holy_light),
        batch16::HOLY_SHOCK | batch16::CRUSADER_STRIKE => Some(batch16::reduce_holy_judgment),
        batch16::RIPTIDE => Some(batch16::grant_tidal_waves),
        batch13::BESTIAL_WRATH => Some(batch13::strike_with_owned_pet),
        batch12::SHIELD_RIGHTEOUS => Some(batch12::shield_armor),
        batch10::BLADE_FLURRY => Some(batch10::cast_flurry),
        batch10::VOIDFORM => Some(batch10::apply_voidform),
        batch10::ARCANE_SURGE => Some(batch10::apply_surge),
        batch9::CONSECRATION => Some(batch9::spawn_consecration),
        batch8::DEMONBOLT => Some(batch8::demonbolt),
        batch8::CRASH_LIGHTNING => Some(batch8::crash_lightning),
        batch8::ARCANE_BARRAGE => Some(batch8::arcane_barrage),
        batch3::FLASH_HEAL => Some(batch3::flash_heal),
        batch3::PRAYER_OF_HEALING => Some(batch4::prayer),
        batch4::VIVIFY => Some(batch4::vivify),
        batch4::ERUPTION => Some(batch4::eruption),
        batch6::STARFALL => Some(batch6::starfall),
        batch7::VERDANT_EMBRACE => Some(batch7::heal_panacea),
        _ => shared::skyriding::ability_impulse(spell_id)
            .map(|_| crate::skyriding::flap as AfterCastHook),
    }
}

/// TC a352b1fa spell_warrior.cpp1380–1405: AfterCast with IGNORE_CAST_IN_PROGRESS,
/// not FULL_MASK. Native385174 owns its self target and10% max-health payload.
fn cast_invigorating_fury_heal(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !crate::auras::has_aura(world, caster, INVIGORATING_FURY) {
        return;
    }
    if let Err(reason) =
        super::creature_cast_ignoring_in_progress(world, caster, INVIGORATING_FURY_HEAL, None)
    {
        warn!(?caster, ?reason, "Invigorating Fury385174 cast refused");
    }
}

/// `SpellObjectAreaTargetSelectFn`: narrows the units an effect selected.
/// Receives live target state, the spell and its explicit target.
pub type AreaTargetFilter = fn(&World, &SpellInfo, Option<Entity>, &mut Vec<Entity>);

pub fn area_target_filter(spell_id: u32, effect_index: u8) -> Option<AreaTargetFilter> {
    if let Some(filter) = prayer_of_mending::area_filter(spell_id, effect_index) {
        return Some(filter);
    }
    match (spell_id, effect_index) {
        (crate::auras::ebon_might::EBON, 0 | 1) => Some(crate::auras::ebon_might::filter_allies),
        (145_110, 0) => Some(pick_yseras_gift_group_target),
        (AZURE_STRIKE, 1) => Some(azure_strike_targets),
        (batch5::EPIDEMIC_DAMAGE, 1) => Some(batch5::epidemic_splash),
        _ => None,
    }
}

/// TC spell_generic.cpp2015–2034: each actual20538 hit loses100% caster threat.
fn hate_to_zero(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if let Some(mut threat) = world.get_mut::<shared::components::ThreatTable>(caster) {
        let amount = threat.threat_for(target.to_bits());
        threat.add_threat(target.to_bits(), -amount);
    }
}

/// TC a352b1fa spell_dh.cpp:1035-1055 bridges203720 to203819;
/// spell_dh_deflecting_spikes:891-910 excludes parry effect0 without321028.
fn demon_spikes(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    let mut effect_mask = u32::MAX;
    if !crate::auras::has_aura(world, caster, DEFLECTING_SPIKES) {
        effect_mask &= !1;
    }
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: DEMON_SPIKES_AURA,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

pub(super) type TargetPlanHook =
    fn(&mut World, Entity, &SpellInfo, Option<Entity>) -> super::TargetPlan;

/// Script-owned target plans still use normal validation, hit and effect paths.
pub(super) fn target_plan(spell_id: u32) -> Option<(Relation, TargetPlanHook)> {
    match spell_id {
        97372 => Some((Relation::Any, track_switch_plan)),
        batch16::HEALING_WAVE => Some((Relation::Ally, batch16::healing_wave_plan)),
        batch12::SHIELD_RIGHTEOUS => Some((Relation::Ally, batch12::shield_plan)),
        batch11::VOID_VOLLEY => Some((Relation::Enemy, batch11::volley_plan)),
        batch8::SANCTIFY => Some((Relation::Ally, batch8::sanctify_plan)),
        batch8::PROVOKE_SINGLE => Some((Relation::Enemy, batch8::provoke_plan)),
        batch8::PROVOKE_AREA => Some((Relation::Any, batch8::provoke_plan)),
        batch3::PRAYER_OF_HEALING => Some((Relation::Ally, batch3::prayer_plan)),
        batch4::LIGHT_OF_DAWN => Some((Relation::Ally, batch4::light_of_dawn_plan)),
        CHAIN_HEAL => Some((Relation::Ally, chain_heal_plan)),
        batch6::STARFIRE => Some((Relation::Enemy, batch6::starfire_plan)),
        batch6::KEG_SMASH => Some((Relation::Enemy, batch6::keg_smash_plan)),
        batch6::WILD_GROWTH => Some((Relation::Ally, batch6::wild_growth_plan)),
        batch7::RADIANCE => Some((Relation::Ally, batch7::radiance_plan)),
        _ => None,
    }
}

/// TC124491 OnEffectHitTarget casts124492 normally, retaining its authored3s cast.
fn start_switch_capture(world: &mut World, _caster: Entity, player: Entity, _depth: u8) {
    if world.get::<shared::components::Player>(player).is_none() {
        return;
    }
    let _ = super::cast_granted(world, player, 124492, Some(player));
}

/// DB2 target38 + TDB condition13/group1/97372/type31: nearest creature60283.
fn track_switch_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    _explicit: Option<Entity>,
) -> super::TargetPlan {
    let map = world.get::<crate::maps::InMap>(caster).copied();
    let origin = targets::position(world, caster).expect("validated caster position");
    let range = spell
        .misc
        .as_ref()
        .expect("source97372 misc")
        .range
        .expect("source97372 range6")
        .max_friendly;
    let mut query = world.query::<(
        Entity,
        &shared::components::Npc,
        &crate::maps::InMap,
        &shared::components::Position,
    )>();
    let target = query
        .iter(world)
        .filter(|(_, npc, at, _)| npc.template_id == 60283 && Some(**at) == map)
        .map(|(entity, _, _, at)| (entity, origin.distance(Vec3::new(at.x, at.y, at.z))))
        .filter(|(_, distance)| *distance <= range)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(entity, _)| entity);
    vec![super::PlannedEffect {
        index: 0,
        units: target.into_iter().collect(),
        dest: None,
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
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
    let amplitude = batch15::chain_falloff(
        world,
        caster,
        script_effect_value(world, caster, CHAIN_HEAL, 1),
    );
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
        blocked_units: Vec::new(),
        prevent_hit_script: false,
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
    if spell_id == touch_of_death::TOUCH_OF_DEATH
        && !touch_of_death::target_allowed(world, caster, target)
    {
        return Err(super::CastFailure::InvalidTarget);
    }
    if spell_id == chrono_flame::CHRONO_FLAMES {
        return chrono_flame::check_children(world);
    }
    if spell_id == batch8::PROVOKE {
        return batch8::check_provoke(world, caster, target);
    }
    if spell_id == batch4::KILL_SHOT && !batch4::kill_shot_allowed(world, caster, target) {
        return Err(super::CastFailure::InvalidTarget);
    }
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
    let mut plan = pyre_explosion_plan(world, caster, &child, center);
    super::hit::remove_defended_targets(world, caster, &child, &mut plan);
    super::missiles::launch(world, caster, &child, plan, depth + 1);
}

fn pyre_explosion_plan(
    world: &mut World,
    caster: Entity,
    child: &SpellInfo,
    center: Vec3,
) -> super::TargetPlan {
    let radius = super::targets::radius(&child.effects[0], 1);
    let units = super::targets::area_units(
        world,
        caster,
        center,
        radius,
        Relation::Enemy,
        super::max_targets(child),
    );
    vec![super::PlannedEffect {
        index: 0,
        units,
        dest: Some(super::targets::Dest {
            position: center,
            orientation: super::targets::orientation(world, caster),
        }),
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
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

/// SimC76f8118 monk4002–4070 uses result_total, including overheal.
/// Native115129 selects its own caster-centered20yd area; no explicit unit/range override.
pub(super) fn cast_expel_harm_damage(
    world: &mut World,
    ctx: &super::effects::EffectCtx,
    total: f32,
) {
    const EXPEL_HARM: u32 = 322101;
    const EXPEL_HARM_DAMAGE: u32 = 115129;
    if ctx.spell.id != EXPEL_HARM {
        return;
    }
    let percent = script_effect_value(world, ctx.caster, EXPEL_HARM, 1);
    super::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        EXPEL_HARM_DAMAGE,
        None,
        ctx.depth + 1,
        total * percent / 100.0,
    );
}

pub(crate) fn script_effect_value(world: &World, caster: Entity, spell_id: u32, index: u8) -> f32 {
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
    let now = super::history_now(world);
    let charge_category = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .and_then(|spell| spell.categories.as_ref())
        .map_or(0, |category| category.charge_category);
    let Some(mut history) = world.get_mut::<super::SpellHistory>(caster) else {
        return;
    };
    let (charges, remaining_ms) =
        reduce_running_timers(&mut history, charge_category, spell_id, now, seconds);
    if let Some(charges) = charges {
        super::send_to_owner(world, caster, charges);
    }
    if let Some(remaining_ms) = remaining_ms {
        send_reduced_cooldown(world, caster, spell_id, remaining_ms);
    }
}

fn reduce_running_timers(
    history: &mut super::SpellHistory,
    charge_category: u32,
    spell_id: u32,
    now: f64,
    seconds: f64,
) -> (Option<shared::protocol::SpellChargesUpdate>, Option<u32>) {
    let charges = history
        .charges
        .get_mut(&charge_category)
        .filter(|charges| charges.used() > 0)
        .map(|charges| {
            charges.reduce_recovery(now, seconds);
            charges.snapshot(now)
        });
    let remaining_ms = history
        .spell_ready_at
        .get_mut(&spell_id)
        .filter(|ready| **ready > now)
        .map(|ready| {
            *ready = (*ready - seconds).max(now);
            ((*ready - now) * 1000.0).round() as u32
        });
    (charges, remaining_ms)
}

/// Keep the full cooldown duration separate from remaining time: the client
/// computes its sweep from remaining/duration and must not restart it at100%.
fn send_reduced_cooldown(world: &mut World, caster: Entity, spell_id: u32, remaining_ms: u32) {
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .expect("running cooldown spell must be in the catalog");
    let base = spell
        .cooldowns
        .as_ref()
        .map_or(0, |cooldown| cooldown.recovery_ms);
    let duration_ms = super::modify(
        world,
        caster,
        spell,
        crate::auras::spell_mods::SpellModOp::Cooldown,
        base as f32,
    )
    .max(0.0)
    .round() as u32;
    super::send_to_owner(
        world,
        caster,
        shared::protocol::SpellCooldownUpdate {
            spell_id,
            category: 0,
            duration_ms,
            remaining_ms,
            is_gcd: false,
        },
    );
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

/// TC a352b1fa spell_druid.cpp:2037-2053: Skull Bash's DUMMY casts
/// its authored charge and interrupt children, in that order, triggered.
fn skull_bash(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    super::cast_triggered(world, caster, SKULL_BASH_CHARGE, Some(target), depth + 1);
    super::cast_triggered(world, caster, SKULL_BASH_INTERRUPT, Some(target), depth + 1);
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

/// TC spell_dru_yseras_gift_group_heal, Spell.cpp:9516-9580:
/// one random target in the best available injury/player priority tier.
fn pick_yseras_gift_group_target(
    world: &World,
    _spell: &SpellInfo,
    _explicit: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    use rand::seq::SliceRandom;
    units.shuffle(&mut rand::thread_rng());
    units.sort_by_key(|&unit| {
        let full = world
            .get::<shared::components::Health>(unit)
            .is_none_or(|health| health.current >= health.max);
        let non_player = world.get::<shared::components::Player>(unit).is_none();
        (full, non_player)
    });
    units.truncate(1);
}

/// `spell_evo_azure_strike::FilterTargets` (spell_evoker.cpp:94-107): the explicit
/// target plus effect 0's value minus one others picked at random
/// (`Trinity::Containers::RandomResize`).
fn azure_strike_targets(
    _world: &World,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    units: &mut Vec<Entity>,
) {
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
        swiftmend::SWIFTMEND => Some(swiftmend::hit),
        soul_of_the_forest::REGROWTH => Some(clearcasting::regrowth_hit),
        soul_of_the_forest::REJUVENATION => Some(soul_of_the_forest::consume_after_heal),
        batch13::MORTAL_STRIKE => Some(batch13::apply_mortal_wounds),
        batch3::GARROTE => Some(batch3::garrote),
        batch5::DISINTEGRATE => Some(batch5::disintegrate),
        batch6::BUTCHERY => Some(batch6::butchery),
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
