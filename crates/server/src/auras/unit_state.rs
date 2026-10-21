//! Unit state an aura set implies, reconciled on every aura change:
//! - MOD_INVISIBILITY 18 / MOD_INVISIBILITY_DETECT 19: typed values reconciled
//!   for per-observer interest and target detection (invisibility.rs).
//! - MOD_UNATTACKABLE 93 (`HandleModUnattackable`): `UNIT_FLAG_NON_ATTACKABLE_2`
//!   while any such aura is on; applying it stops the unit's attack in a dungeon
//!   (`AttackStop`) and its whole combat elsewhere (`CombatStop`).
//! - MOD_CONFUSE 5 (`HandleModConfuse` → `SetControlled(UNIT_STATE_CONFUSED)`): the first
//!   aura stops the unit's cast (`CastStop`) and gives it `Confused`, anchored where it
//!   stands; a creature then wanders around that point (`creature_ai`, `MoveConfused`)
//!   instead of chasing, does not swing (`UNIT_STATE_CANNOT_AUTOATTACK`) and shows no
//!   target (`SetTarget(Empty)`). The last aura's removal ends it.
//! - TRANSFORM 56 (`HandleAuraTransform`, `RestoreDisplayId`): the newest transform aura,
//!   a harmful one first, shows a model of the creature template in its MiscValue
//!   (`ObjectMgr::ChooseDisplayId`); else a MOD_SHAPESHIFT 36 form shows its model
//!   (`shapeshift`); with neither left the native display returns.

use crate::random as rand;
use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{ModelDisplay, Position, UnitFlags, UnitFlags2};

use super::handlers::{MOD_CONFUSE, MOD_FEAR, MOD_UNATTACKABLE, TRANSFORM};
use super::{ActiveAuras, AuraInstance};

/// Under a MOD_UNATTACKABLE aura: the aura set `NON_ATTACKABLE_2`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct Unattackable;

/// Under a MOD_CONFUSE aura (`UNIT_STATE_CONFUSED`), with its `ConfusedMovementGenerator`
/// state: the Bevy (x, z) it wanders around, the point it walks to, and the pause
/// before it picks the next one.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Confused {
    pub reference: (f32, f32),
    pub destination: Option<(f32, f32)>,
    pub wait_secs: f32,
}

/// Under a MOD_FEAR aura (`UNIT_STATE_FLEEING`): the first fear aura's caster, which
/// `Unit::SetFeared` passes to `MoveFleeing`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Feared {
    pub flee_from: Entity,
}

/// The aura whose display shows instead of the native one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplaySource {
    /// A TRANSFORM aura instance.
    Transform(u32),
    /// A MOD_SHAPESHIFT aura instance.
    Form(u32),
}

/// Under a TRANSFORM or MOD_SHAPESHIFT aura: the aura whose display shows, and the
/// display to restore when neither is left. `None` for a unit that had no
/// `ModelDisplay`: a player, whose native display is its race and sex model (which
/// clients resolve themselves), gets it back by losing `ModelDisplay`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transformed {
    pub source: DisplaySource,
    pub native_display_id: Option<u32>,
}

fn has_type(auras: &ActiveAuras, aura_type: u32) -> bool {
    auras
        .iter()
        .flat_map(|aura| &aura.effects)
        .any(|effect| effect.aura_type == aura_type)
}

pub(super) fn reconcile(world: &mut World, unit: Entity) {
    let Some(auras) = world.get::<ActiveAuras>(unit) else {
        return;
    };
    let visibility = super::Invisibility::from_auras(auras);
    let stealth = super::stealth::Stealth::from_auras(auras);
    let preparing = has_type(auras, super::handlers::ARENA_PREPARATION);
    let forced_forward = has_type(auras, super::handlers::FORCE_MOVE_FORWARD);
    let unattackable = has_type(auras, MOD_UNATTACKABLE);
    let confused = has_type(auras, MOD_CONFUSE);
    let fear_caster = auras
        .iter()
        .find(|aura| {
            aura.effects
                .iter()
                .any(|effect| effect.aura_type == MOD_FEAR)
        })
        .map(|aura| aura.caster);
    let flight = super::mount::flight(auras);
    let feather_fall =
        shared::components::FeatherFall(has_type(auras, super::handlers::FEATHER_FALL));
    reconcile_preparation(world, unit, preparing);
    reconcile_forced_forward(world, unit, forced_forward);
    reconcile_scale(world, unit);
    reconcile_feather_fall(world, unit, feather_fall);
    if world.get::<super::Invisibility>(unit) != Some(&visibility) {
        world.entity_mut(unit).insert(visibility.clone());
        crate::interest::set_invisibility(world, unit, visibility);
    }
    if world.get::<super::stealth::Stealth>(unit) != Some(&stealth) {
        world.entity_mut(unit).insert(stealth.clone());
        crate::interest::set_stealth(world, unit, stealth);
    }
    if unattackable != world.get::<Unattackable>(unit).is_some() {
        reconcile_unattackable(world, unit, unattackable);
    }
    if confused != world.get::<Confused>(unit).is_some() {
        reconcile_confused(world, unit, confused);
    }
    if fear_caster.is_some() != world.get::<Feared>(unit).is_some() {
        reconcile_feared(world, unit, fear_caster);
    }
    super::mount::reconcile_can_fly(world, unit, flight);
    reconcile_display(world, unit);
    super::shapeshift::reconcile_display_power(world, unit);
}

/// TC HandleForceMoveForward: removing one aura retains the flag if another remains.
fn reconcile_forced_forward(world: &mut World, unit: Entity, forced: bool) {
    let previous = world.get::<UnitFlags2>(unit).copied().unwrap_or_default();
    let next = if forced {
        UnitFlags2(previous.0 | UnitFlags2::FORCE_MOVEMENT)
    } else {
        UnitFlags2(previous.0 & !UnitFlags2::FORCE_MOVEMENT)
    };
    if next == previous {
        return;
    }
    world.entity_mut(unit).insert(next);
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(unit);
    }
}

fn reconcile_preparation(world: &mut World, unit: Entity, preparing: bool) {
    let previous = world
        .get::<UnitFlags>(unit)
        .copied()
        .unwrap_or(UnitFlags(0));
    let mut flags = previous;
    if preparing {
        flags.0 |= UnitFlags::ARENA_PREPARATION;
    } else {
        flags.0 &= !UnitFlags::ARENA_PREPARATION;
    }
    if flags != previous {
        world.entity_mut(unit).insert(flags);
        if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
            dirty.mark(unit);
        }
    }
}

fn reconcile_scale(world: &mut World, unit: Entity) {
    let points = world
        .get::<ActiveAuras>(unit)
        .expect("aura unit")
        .modifier(super::handlers::MOD_SCALE);
    let minimum = if world.get::<shared::components::Player>(unit).is_some() {
        0.1
    } else {
        0.01
    };
    let native = world
        .get::<crate::zones::NativeObjectScale>(unit)
        .map_or(1.0, |scale| scale.0);
    let object_scale = (native * (1.0 + points / 100.0)).max(minimum);
    let display_scale = world
        .get::<crate::spell_cast::summon::SummonedCreature>(unit)
        .map_or(1.0, |summon| summon.display_scale);
    let scale = shared::components::UnitScale(object_scale * display_scale);
    if world.get::<shared::components::UnitScale>(unit).copied() == Some(scale) {
        return;
    }
    world.entity_mut(unit).insert(scale);
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(unit);
    }
}

fn reconcile_feather_fall(world: &mut World, unit: Entity, next: shared::components::FeatherFall) {
    let previous = world
        .get::<shared::components::FeatherFall>(unit)
        .copied()
        .unwrap_or_default();
    if previous == next {
        return;
    }
    world.entity_mut(unit).insert(next);
    // On removal Retail starts the damaging fall at the current height. Reset the
    // envelope on application too, so activating mid-fall starts the capped arc here.
    if let Some(position) = world.get::<Position>(unit).copied()
        && let Some(mut tracker) = world.get_mut::<crate::networking::FallTracker>(unit)
    {
        tracker.restart_at(position.y);
    }
    if next.0
        && let Some(mut velocity) = world.get_mut::<shared::components::VerticalVelocity>(unit)
    {
        velocity.0 = velocity.0.max(-shared::movement::FEATHER_FALL_SPEED);
    }
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(unit);
    }
}

/// `Unit::SetControlled(UNIT_STATE_CONFUSED)`: applying stops the cast (`CastStop`) and
/// starts the confused wander where the unit stands.
fn reconcile_confused(world: &mut World, unit: Entity, confused: bool) {
    if !confused {
        world.entity_mut(unit).remove::<Confused>();
        return;
    }
    crate::spell_cast::stop_all_casts(world, unit);
    let reference = world
        .get::<Position>(unit)
        .map_or((0.0, 0.0), |position| (position.x, position.z));
    world.entity_mut(unit).insert(Confused {
        reference,
        destination: None,
        wait_secs: 0.0,
    });
}

/// `Unit::SetControlled(UNIT_STATE_FLEEING)`: applying stops the cast (`CastStop`) and
/// clears the unit's target (`SetFeared`).
fn reconcile_feared(world: &mut World, unit: Entity, fear_caster: Option<Entity>) {
    let Some(flee_from) = fear_caster else {
        world.entity_mut(unit).remove::<Feared>();
        super::fleeing::stop(world, unit);
        return;
    };
    crate::spell_cast::stop_all_casts(world, unit);
    world.entity_mut(unit).insert(Feared { flee_from });
    super::fleeing::start(world, unit);
}

/// The transform aura whose display shows (`Unit::RestoreDisplayId`): the newest,
/// unless an older one is harmful.
fn shown_transform(auras: &ActiveAuras) -> Option<&AuraInstance> {
    let mut transforms = auras.auras.iter().rev().filter(|aura| {
        aura.effects
            .iter()
            .any(|effect| effect.aura_type == TRANSFORM)
    });
    let newest = transforms.next()?;
    if newest.harmful {
        return Some(newest);
    }
    transforms.find(|aura| aura.harmful).or(Some(newest))
}

/// `Unit::RestoreDisplayId`: the shown transform aura, else the shapeshift form's model,
/// else the native display.
fn shown_display(world: &World, unit: Entity) -> Option<(DisplaySource, u32)> {
    let auras = world.get::<ActiveAuras>(unit)?;
    if let Some(aura) = shown_transform(auras) {
        return Some((DisplaySource::Transform(aura.instance_id), aura.spell_id));
    }
    let (aura, _) = super::shapeshift::current_form(auras)?;
    Some((DisplaySource::Form(aura.instance_id), aura.spell_id))
}

/// The display `source` shows; `None` when its model cannot be resolved.
fn source_display(
    world: &World,
    unit: Entity,
    source: DisplaySource,
    spell_id: u32,
) -> Option<u32> {
    let auras = world.get::<ActiveAuras>(unit)?;
    match source {
        DisplaySource::Transform(instance_id) => {
            let entry = transform_entry(auras.get(instance_id)?);
            transform_display(world, unit, spell_id, entry)
        }
        DisplaySource::Form(instance_id) => {
            let form = super::shapeshift::aura_form(auras.get(instance_id)?)?;
            let display = world
                .get_resource::<super::shapeshift::ShapeshiftForms>()
                .and_then(|forms| forms.display_id(form));
            if display.is_none() {
                debug!("Shapeshift {spell_id}: form {form} has no display");
            }
            display
        }
    }
}

/// Show the display of the shown transform or form aura, or restore the native display.
fn reconcile_display(world: &mut World, unit: Entity) {
    let shown = shown_display(world, unit);
    let current = world.get::<Transformed>(unit).copied();
    let display = match (shown, current) {
        (Some((source, _)), Some(current)) if current.source == source => {
            return;
        }
        (Some((source, spell_id)), _) => {
            let Some(display_id) = source_display(world, unit, source, spell_id) else {
                return;
            };
            let native_display_id = match current {
                Some(current) => current.native_display_id,
                None => world
                    .get::<ModelDisplay>(unit)
                    .map(|model| model.display_id),
            };
            world.entity_mut(unit).insert(Transformed {
                source,
                native_display_id,
            });
            Some(display_id)
        }
        (None, Some(current)) => {
            world.entity_mut(unit).remove::<Transformed>();
            current.native_display_id
        }
        (None, None) => return,
    };
    // Unit::SetDisplayId / RestoreDisplayId.
    match display {
        Some(display_id) => {
            world.entity_mut(unit).insert(ModelDisplay { display_id });
        }
        None => {
            world.entity_mut(unit).remove::<ModelDisplay>();
        }
    }
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(unit);
    }
}

/// `SPELLFAMILY_MAGE` and the Polymorph family flag (`SpellInfo::GetSpellSpecific`).
const SPELLFAMILY_MAGE: u32 = 3;
const MAGE_POLYMORPH_FLAG: u32 = 0x0100_0000;

/// `Unit::IsPolymorphed`: the shown transform is `SPELL_SPECIFIC_MAGE_POLYMORPH` (a mage
/// spell with family flag 0x1000000 whose effect 0 is MOD_CONFUSE).
pub fn is_polymorphed(world: &World, unit: Entity) -> bool {
    let Some(Transformed {
        source: DisplaySource::Transform(instance_id),
        ..
    }) = world.get::<Transformed>(unit).copied()
    else {
        return false;
    };
    let Some(aura) = world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| auras.get(instance_id))
    else {
        return false;
    };
    world
        .get_resource::<crate::spell_info::SpellStore>()
        .and_then(|store| store.get(aura.spell_id))
        .is_some_and(|spell| {
            spell.class_options.as_ref().is_some_and(|options| {
                options.spell_class_set == SPELLFAMILY_MAGE
                    && options.spell_class_mask[0] & MAGE_POLYMORPH_FLAG != 0
            }) && spell
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .is_some_and(|effect| super::is_aura_effect(effect) && effect.aura == MOD_CONFUSE)
        })
}

/// The creature template entry (`EffectMiscValue_0`) of the aura's transform effect.
fn transform_entry(aura: &AuraInstance) -> i32 {
    aura.effects
        .iter()
        .find(|effect| effect.aura_type == TRANSFORM)
        .map_or(0, |effect| effect.misc_values[0])
}

/// The pink pig TrinityCore shows for a transform into an unknown creature template.
const UNKNOWN_TEMPLATE_DISPLAY: u32 = 16358;

/// The display a TRANSFORM aura shows (AuraEffect::HandleAuraTransform,
/// SpellAuraEffects.cpp:2090-2242): a model of creature template `entry`
/// (`ObjectMgr::ChooseDisplayId`); the pink pig 16358, with an error, for an entry with no
/// template. Source-defined zero-entry costumes use the exact authored special
/// cases below; unimplemented zero-entry spells keep their current display.
fn transform_display(world: &World, unit: Entity, spell_id: u32, entry: i32) -> Option<u32> {
    if entry == 0 {
        if spell_id == 42365 {
            return Some(21723);
        }
        let player = world.get::<shared::components::Player>(unit)?;
        return zero_entry_costume_display(spell_id, player.race, player.appearance.sex);
    }
    let entry = u32::try_from(entry).ok()?;
    let template = world
        .get_resource::<crate::world_data::WorldData>()?
        .templates
        .get(&entry);
    let Some(template) = template else {
        error!(
            "Auras: unknown creature id = {entry} (only need its modelid) From Spell Aura \
             Transform in Spell ID = {spell_id}"
        );
        return Some(UNKNOWN_TEMPLATE_DISPLAY);
    };
    Some(crate::zones::template_display_id(
        template,
        &mut rand::thread_rng(),
    ))
}

/// TC a352b1fa HandleAuraTransform: native sex, not the currently transformed model.
fn zero_entry_costume_display(spell_id: u32, race: u8, sex: u8) -> Option<u32> {
    match (spell_id, sex) {
        (16739, _) => orb_deception_display(race, sex),
        (42365, _) => Some(21723),
        (75531, 0) => Some(31654),
        (75531, 1) => Some(31655),
        (75532, 0) => Some(31737),
        (75532, 1) => Some(31738),
        _ => None,
    }
}

/// TC a352b1fa HandleAuraTransform2110-2161: exact race/native-gender identities.
/// Unlisted races have no source mapping; do not invent an opposing-faction model.
pub(crate) fn orb_deception_display(race: u8, sex: u8) -> Option<u32> {
    let (male, female) = match race {
        1 => (10137, 10138),
        2 => (10139, 10140),
        3 => (10141, 10142),
        4 => (10143, 10144),
        5 => (10146, 10145),
        6 => (10136, 10147),
        7 => (10148, 10149),
        8 => (10135, 10134),
        10 => (17829, 17830),
        11 => (17827, 17828),
        _ => return None,
    };
    Some(if sex == 1 { female } else { male })
}

pub(crate) fn supports_zero_entry_costume(spell_id: u32) -> bool {
    zero_entry_costume_display(spell_id, 1, 0).is_some()
}

/// The first MOD_UNATTACKABLE aura sets the flag, the last one's removal clears it.
fn reconcile_unattackable(world: &mut World, unit: Entity, unattackable: bool) {
    let flags = world
        .get::<UnitFlags>(unit)
        .copied()
        .unwrap_or(UnitFlags(0));
    let next = if unattackable {
        world.entity_mut(unit).insert(Unattackable);
        flags.0 | UnitFlags::NON_ATTACKABLE_2
    } else {
        world.entity_mut(unit).remove::<Unattackable>();
        flags.0 & !UnitFlags::NON_ATTACKABLE_2
    };
    world.entity_mut(unit).insert(UnitFlags(next));
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(unit);
    }
    if !unattackable {
        return;
    }
    let in_dungeon = world
        .get::<crate::maps::InMap>(unit)
        .is_some_and(|map| map.instance_id != 0);
    if in_dungeon {
        crate::scripted_ai::attack_stop(world, unit);
    } else {
        crate::combat::combat_stop(world, unit);
    }
}
