//! Unit state an aura set implies, reconciled on every aura change:
//! - MOD_INVISIBILITY 18 (`HandleModInvisibility`): no player detects the invisible
//!   unit (there are no invisibility detection auras), so interest hides it from
//!   every client (`UpdateObjectVisibility`) and it is no valid target (`CanSeeOrDetect` in
//!   `IsValidAttackTarget`/`IsValidAssistTarget`, `spell_cast::targets`); the last
//!   aura's removal shows it again.
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
//!   (`ObjectMgr::ChooseDisplayId`); with none left the native display returns.

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{ModelDisplay, Position, UnitFlags};

use super::handlers::{MOD_CONFUSE, MOD_INVISIBILITY, MOD_UNATTACKABLE, TRANSFORM};
use super::{ActiveAuras, AuraInstance};

/// Under a MOD_INVISIBILITY aura (hidden in `interest`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Invisible;

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

/// Under a TRANSFORM aura: the aura instance whose display shows, and the display
/// to restore when no transform is left.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transformed {
    pub instance_id: u32,
    pub native_display_id: u32,
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
    let invisible = has_type(auras, MOD_INVISIBILITY);
    let unattackable = has_type(auras, MOD_UNATTACKABLE);
    let confused = has_type(auras, MOD_CONFUSE);
    if invisible != world.get::<Invisible>(unit).is_some() {
        if invisible {
            world.entity_mut(unit).insert(Invisible);
        } else {
            world.entity_mut(unit).remove::<Invisible>();
        }
        crate::interest::set_hidden(world, unit, invisible);
    }
    if unattackable != world.get::<Unattackable>(unit).is_some() {
        reconcile_unattackable(world, unit, unattackable);
    }
    if confused != world.get::<Confused>(unit).is_some() {
        reconcile_confused(world, unit, confused);
    }
    reconcile_transform(world, unit);
}

/// `Unit::SetControlled(UNIT_STATE_CONFUSED)`: applying stops the cast (`CastStop`) and
/// starts the confused wander where the unit stands.
fn reconcile_confused(world: &mut World, unit: Entity, confused: bool) {
    if !confused {
        world.entity_mut(unit).remove::<Confused>();
        return;
    }
    crate::spell_cast::cancel_cast(world, unit);
    let reference = world
        .get::<Position>(unit)
        .map_or((0.0, 0.0), |position| (position.x, position.z));
    world.entity_mut(unit).insert(Confused {
        reference,
        destination: None,
        wait_secs: 0.0,
    });
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

/// Show the display of the shown transform aura, or restore the native display.
fn reconcile_transform(world: &mut World, unit: Entity) {
    let shown = world
        .get::<ActiveAuras>(unit)
        .and_then(shown_transform)
        .map(|aura| (aura.instance_id, aura.spell_id, transform_entry(aura)));
    let current = world.get::<Transformed>(unit).copied();
    let display = match (shown, current) {
        (Some((instance_id, _, _)), Some(current)) if current.instance_id == instance_id => {
            return;
        }
        (Some((instance_id, spell_id, entry)), _) => {
            let Some(display_id) = transform_display(world, spell_id, entry) else {
                return;
            };
            let Some(native_display_id) =
                current
                    .map(|current| current.native_display_id)
                    .or_else(|| {
                        world
                            .get::<ModelDisplay>(unit)
                            .map(|model| model.display_id)
                    })
            else {
                debug!("Transform {spell_id} on {unit}: no ModelDisplay to replace");
                return;
            };
            world.entity_mut(unit).insert(Transformed {
                instance_id,
                native_display_id,
            });
            display_id
        }
        (None, Some(current)) => {
            world.entity_mut(unit).remove::<Transformed>();
            current.native_display_id
        }
        (None, None) => return,
    };
    world.entity_mut(unit).insert(ModelDisplay {
        display_id: display,
    });
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
    let Some(transformed) = world.get::<Transformed>(unit) else {
        return false;
    };
    let Some(aura) = world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| auras.get(transformed.instance_id))
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

/// A display of creature template `entry` (`ObjectMgr::ChooseDisplayId`). MiscValue 0
/// (TrinityCore's per-spell racial displays) and unknown templates are logged and
/// change nothing.
fn transform_display(world: &World, spell_id: u32, entry: i32) -> Option<u32> {
    let template = u32::try_from(entry)
        .ok()
        .filter(|&entry| entry != 0)
        .and_then(|entry| {
            world
                .get_resource::<crate::world_data::WorldData>()?
                .templates
                .get(&entry)
        });
    let Some(template) = template else {
        error!("Transform {spell_id}: no creature template {entry} for its display");
        return None;
    };
    Some(crate::zones::template_display_id(
        template,
        &mut rand::thread_rng(),
    ))
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
