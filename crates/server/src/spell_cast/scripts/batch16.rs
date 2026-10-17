//! Named rank consumers from local Mainline12.1.0.69933 rows/descriptions.
use super::script_effect_value;
use crate::spell_cast::{self, powers};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;
use shared::components::PowerType;

pub(super) const HOLY_LIGHT: u32 = 82326;
pub(super) const HOLY_SHOCK: u32 = 20473;
pub(super) const CRUSADER_STRIKE: u32 = 35395;
pub(super) const RET_JUDGMENT: u32 = 20271;
pub(super) const HOLY_JUDGMENT: u32 = 275773;
pub(super) const RIPTIDE: u32 = 61295;
pub(super) const HEALING_WAVE: u32 = 77472;
const HOLY_LIGHT_RANK: u32 = 231642;
const HOLY_LIGHT_POWER: u32 = 88852;
const JUDGMENT_RANK: u32 = 315867;
const JUDGMENT_POWER: u32 = 220637;
const CRUSADERS_MIGHT: u32 = 196926;
const TIDAL_WAVES: u32 = 51564;
const TIDAL_BUFF: u32 = 53390;
const ARCANE_BLAST: u32 = 30451;
const ARCANE_CHARGE: u32 = 36032;

///82326 explicitly gates the named88852s1 gain on231642, once per cast.
pub(super) fn energize_holy_light(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if crate::auras::has_aura(world, caster, HOLY_LIGHT_RANK) {
        spell_cast::cast_triggered(world, caster, HOLY_LIGHT_POWER, Some(caster), 1);
    }
}

///20271/275773 name220637s1 under315867. Only actual primary damage hits
///generate power; latent splash effects do not multiply the root's gain.
pub(super) fn energize_judgment(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if crate::auras::has_aura(world, caster, JUDGMENT_RANK)
        && targets_primary_judgment(world, caster, target)
    {
        spell_cast::cast_triggered(world, caster, JUDGMENT_POWER, Some(caster), depth + 1);
    }
}
fn targets_primary_judgment(world: &World, caster: Entity, target: Entity) -> bool {
    [RET_JUDGMENT, HOLY_JUDGMENT]
        .into_iter()
        .any(|root| super::batch8::first_damage_target(world, caster, root, target))
}

///196926 names separate millisecond fields for Holy Shock/Crusader Strike.
///This bounded Holy path reduces275773; offspec Judgment variants stay open.
pub(super) fn reduce_holy_judgment(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if crate::auras::has_aura(world, caster, CRUSADERS_MIGHT) {
        let index = u8::from(spell.id == CRUSADER_STRIKE);
        let milliseconds = script_effect_value(world, caster, CRUSADERS_MIGHT, index);
        super::reduce_cooldown(
            world,
            caster,
            HOLY_JUDGMENT,
            f64::from(-milliseconds) / 1000.0,
        );
    }
}

///77472's native45 selector has zero chain targets. Resolve only its validated
///primary ally through the normal heal handler, not a generic chain selector.
pub(super) fn healing_wave_plan(
    _world: &mut World,
    _caster: Entity,
    _spell: &SpellInfo,
    explicit: Option<Entity>,
) -> spell_cast::TargetPlan {
    vec![spell_cast::PlannedEffect {
        index: 0,
        units: vec![explicit.expect("Healing Wave requires a validated ally")],
        dest: None,
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

///51564s1 grants53390; native CumulativeAura supplies the cap, not a guessed
///two-stack reset. Each cast adds its authored stack count to existing uses.
pub(super) fn grant_tidal_waves(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if crate::auras::has_aura(world, caster, TIDAL_WAVES) {
        let stacks = script_effect_value(world, caster, TIDAL_WAVES, 0) as u32;
        for _ in 0..stacks {
            spell_cast::cast_triggered(world, caster, TIDAL_BUFF, Some(caster), 1);
        }
    }
}

///30451's description binds live charge count to36032s1 damage, s4 cast
///time and s5 mana cost. Damage callers snapshot before30451s2 gains a charge.
pub(in crate::spell_cast) fn blast_multiplier(world: &World, caster: Entity, index: u8) -> f32 {
    let (charges, _) = powers::power_value(world, caster, PowerType::ArcaneCharges)
        .expect("Arcane Blast requires ArcaneCharges");
    1.0 + charges * script_effect_value(world, caster, ARCANE_CHARGE, index) / 100.0
}
pub(in crate::spell_cast) fn blast_cast_time(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    base: f32,
) -> f32 {
    if spell.id == ARCANE_BLAST {
        base * blast_multiplier(world, caster, 3)
    } else {
        base
    }
}

/// Arcane Blast's charge cost follows the generic school-cost aura fold.
/// Arcane Power's native aura72 is already applied there, without a rank gate.
pub(in crate::spell_cast) fn mana_cost(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    base: f32,
) -> f32 {
    if spell.id == ARCANE_BLAST {
        base * blast_multiplier(world, caster, 4)
    } else {
        base
    }
}
