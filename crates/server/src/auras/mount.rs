//! Mounts and the flight they grant (docs/wiki/systems/mounts.md), after TrinityCore
//! master (a352b1fa):
//! - MOUNTED 78 applied (`AuraEffect::HandleAuraMounted`, SpellAuraEffects.cpp:2752-2835):
//!   the rider shows the mount's display (`Unit::Mount`): a `MountXDisplay` of the spell's
//!   `Mount` row whose PlayerCondition the rider meets, at random; `DISPLAYID_HIDDEN_MOUNT`
//!   for a self mount; without a `Mount` row a model of the creature template in
//!   `EffectMiscValue_0` (`ObjectMgr::ChooseDisplayId`). The effect's amount is the
//!   `MountCapability` the rider qualifies for (`AuraEffect::CalculateAmount`, :813-822,
//!   `Unit::GetMountCapability`, Unit.cpp:8379-8474), whose `ModSpellAuraID` the rider
//!   casts on itself: the speed aura (Mount Speed Mod: Standard Flying Mount 86459 adds
//!   MOD_INCREASE_MOUNTED_SPEED 32 and MOD_INCREASE_MOUNTED_FLIGHT_SPEED 207).
//! - MOUNTED 78 removed (:2836-2858): `Unit::Dismount` (no display, Dismount aura
//!   interrupts), every other mount aura and the capability's speed aura go.
//! - FLY 201 / MOD_INCREASE_MOUNTED_FLIGHT_SPEED 207 (`HandleAuraAllowFlying`,
//!   `HandleAuraModIncreaseFlightSpeed` :3402-3422 → `Unit::SetCanFly`): while either is
//!   on, `PlayerMotion::CAN_FLY`; losing the last one drops `FLYING` too, so the player
//!   falls (`MoveFall`).
//! - ADV_FLYING 446 (`HandleModAdvFlying` :2986-2995 → `SetCanFly`, `SetCanAdvFly`): while
//!   on, `PlayerMotion::CAN_FLY | CAN_ADV_FLY`, the Skyriding capability's aura 406095;
//!   losing it drops `ADV_FLYING`.
//! - Every player carries a Flight Style aura: Steady 404468, which the steady flight
//!   capabilities require, or Skyriding 404464. Skyriding capability 494 needs Skyriding
//!   Basics 376777 and PlayerCondition 96927 (NOT Flight Style: Steady). TrinityCore always
//!   adds Steady on login (`Player::_LoadAuras`, Player.cpp:19126-19127, "TODO: finish
//!   dragonriding"); Retail keeps the style across logins, so the server saves it per
//!   character (redb `character_flight_style`) and enters the world on the saved one.
//!   Switch Flight Style (436854 toggles, 460002 to Skyriding, 460003 to Steady) has only a
//!   DUMMY effect, so a spell script switches (`spell_cast::scripts`); the next mount picks
//!   its capability from the new aura (both auras are DUMMY auras, which TrinityCore's
//!   `UpdateMountCapability` does not follow).
//!
//! Not modelled: `AreaTable.MountFlags` (the server has no area table, so ground and flying
//! capabilities are allowed everywhere), `ReqAreaID` capabilities (never selected), the
//! PlayerCondition fields other than its aura spells (ModifierTree, ...), and recomputing
//! the capability when the rider's water, area or riding skill changes
//! (`Unit::UpdateMountCapability`).

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use rusqlite::Connection;
use shared::components::{Mounted, Player, PlayerMotion};

use super::handlers::{ADV_FLYING, FLY, MOD_INCREASE_MOUNTED_FLIGHT_SPEED, MOUNTED};
use super::{ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason};

/// `SKILL_RIDING` (SharedDefines.h:5961).
pub const SKILL_RIDING: u32 = 762;
/// Flight Style: Steady.
pub const FLIGHT_STYLE_STEADY: u32 = 404468;
/// `DISPLAYID_HIDDEN_MOUNT` (Unit.h:40).
const DISPLAYID_HIDDEN_MOUNT: u32 = 73200;
/// `MountFlags::IsSelfMount` (DBCEnums.h:2127).
const MOUNT_FLAG_SELF_MOUNT: u32 = 0x2;
/// `MOUNT_CAPABILITY_FLAG_GROUND` / `_UNDERWATER` (DBCEnums.h:2117-2120).
const CAPABILITY_GROUND: u32 = 0x1;
const CAPABILITY_UNDERWATER: u32 = 0x8;
/// `SpellAuraInterruptFlags::Dismount` (SpellDefines.h:86).
const AURA_INTERRUPT_DISMOUNT: u32 = 0x40;

/// A `Mount` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountEntry {
    pub id: u32,
    pub mount_type: u32,
    pub flags: u32,
}

/// A `MountCapability` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountCapability {
    pub id: u32,
    pub flags: u32,
    pub req_riding_skill: u16,
    pub req_area: u32,
    pub req_spell_aura: u32,
    pub req_spell_known: u32,
    pub mod_spell_aura: u32,
    pub req_map: i32,
    pub player_condition: u32,
}

/// The aura spell requirement of a `PlayerCondition` (`AuraSpellLogic`, `AuraSpellID`,
/// `AuraStacks`), the only part evaluated here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuraCondition {
    pub logic: u32,
    pub spells: [u32; 4],
    pub stacks: [u32; 4],
}

impl AuraCondition {
    /// `IsPlayerMeetingCondition`'s aura spell check (ConditionMgr.cpp:3203-3217) with
    /// `PlayerConditionLogic` (:2821-2843); `stacks_of` counts a spell's aura stacks.
    fn met(&self, stacks_of: impl Fn(u32) -> u32) -> bool {
        if self.spells[0] == 0 {
            return true;
        }
        let results = self.spells.iter().zip(self.stacks).enumerate().fold(
            0u32,
            |mask, (index, (&spell, count))| {
                let held = stacks_of(spell);
                let met = spell == 0 || if count > 0 { held >= count } else { held > 0 };
                mask | (u32::from(met) << index)
            },
        );
        let results = results ^ (self.logic >> 16);
        let mut result = results & 1;
        for index in 1..4 {
            match (self.logic >> (2 * (index - 1))) & 3 {
                1 => result &= (results >> index) & 1,
                2 => result |= (results >> index) & 1,
                _ => {}
            }
        }
        result != 0
    }
}

/// `Mount`, `MountXDisplay`, `MountTypeXCapability`, `MountCapability` and the aura part of
/// the PlayerConditions they name.
#[derive(Resource, Debug, Default)]
pub struct MountStore {
    by_spell: HashMap<u32, MountEntry>,
    /// `(CreatureDisplayInfoID, PlayerConditionID)` by mount.
    displays: HashMap<u32, Vec<(u32, u32)>>,
    /// Capabilities of a mount type in `OrderIndex` order.
    capabilities: HashMap<u32, Vec<MountCapability>>,
    conditions: HashMap<u32, AuraCondition>,
}

/// What `Unit::GetMountCapability` reads of the rider.
pub struct Rider<'a> {
    pub riding_skill: u16,
    /// Swimming: `GetMountCapability`'s submerged and in water (a swimmer has
    /// `MOVEMENTFLAG_SWIMMING`); else out of water. The water surface case is not modelled.
    pub submerged: bool,
    pub map_id: u32,
    pub aura_stacks: &'a dyn Fn(u32) -> u32,
    pub knows: &'a dyn Fn(u32) -> bool,
}

impl MountStore {
    fn capability_by_id(&self, id: u32) -> Option<MountCapability> {
        self.capabilities
            .values()
            .flatten()
            .find(|capability| capability.id == id)
            .copied()
    }

    pub fn mount_for_spell(&self, spell_id: u32) -> Option<MountEntry> {
        self.by_spell.get(&spell_id).copied()
    }

    fn condition_met(&self, condition: u32, rider: &Rider) -> bool {
        condition == 0
            || self
                .conditions
                .get(&condition)
                .is_none_or(|condition| condition.met(rider.aura_stacks))
    }

    /// The displays of `mount` whose PlayerCondition the rider meets.
    pub fn usable_displays(&self, mount: u32, rider: &Rider) -> Vec<u32> {
        self.displays.get(&mount).map_or_else(Vec::new, |displays| {
            displays
                .iter()
                .filter(|&&(_, condition)| self.condition_met(condition, rider))
                .map(|&(display, _)| display)
                .collect()
        })
    }

    /// `Unit::GetMountCapability`: the first capability of `mount_type` the rider
    /// qualifies for.
    pub fn capability(&self, mount_type: u32, rider: &Rider) -> Option<MountCapability> {
        self.capabilities
            .get(&mount_type)?
            .iter()
            .find(|capability| self.qualifies(capability, rider))
            .copied()
    }

    fn qualifies(&self, capability: &MountCapability, rider: &Rider) -> bool {
        let terrain = if rider.submerged {
            CAPABILITY_UNDERWATER
        } else {
            CAPABILITY_GROUND
        };
        rider.riding_skill >= capability.req_riding_skill
            && capability.flags & terrain != 0
            && (capability.req_map == -1 || capability.req_map == rider.map_id as i32)
            && capability.req_area == 0
            && (capability.req_spell_aura == 0
                || (rider.aura_stacks)(capability.req_spell_aura) > 0)
            && (capability.req_spell_known == 0 || (rider.knows)(capability.req_spell_known))
            && self.condition_met(capability.player_condition, rider)
    }
}

fn uint(row: &rusqlite::Row, index: usize) -> rusqlite::Result<u32> {
    Ok(row.get::<_, i64>(index)? as u32)
}

pub fn load_mount_store(conn: &Connection) -> rusqlite::Result<MountStore> {
    let mut store = MountStore::default();
    let mut statement = conn.prepare("SELECT ID, MountTypeID, Flags, SourceSpellID FROM mount")?;
    for row in statement.query_map([], |row| {
        Ok((
            uint(row, 3)?,
            MountEntry {
                id: uint(row, 0)?,
                mount_type: uint(row, 1)?,
                flags: uint(row, 2)?,
            },
        ))
    })? {
        let (spell, mount) = row?;
        store.by_spell.insert(spell, mount);
    }
    let mut statement = conn.prepare(
        "SELECT MountID, CreatureDisplayInfoID, PlayerConditionID FROM mount_x_display ORDER BY ID",
    )?;
    for row in statement.query_map([], |row| Ok((uint(row, 0)?, uint(row, 1)?, uint(row, 2)?)))? {
        let (mount, display, condition) = row?;
        store
            .displays
            .entry(mount)
            .or_default()
            .push((display, condition));
    }
    let mut statement = conn.prepare(
        "SELECT x.MountTypeID, c.ID, c.Flags, c.ReqRidingSkill, c.ReqAreaID, c.ReqSpellAuraID,
                c.ReqSpellKnownID, c.ModSpellAuraID, c.ReqMapID, c.PlayerConditionID
         FROM mount_type_x_capability x JOIN mount_capability c ON c.ID = x.MountCapabilityID
         ORDER BY x.MountTypeID, x.OrderIndex",
    )?;
    for row in statement.query_map([], |row| {
        Ok((
            uint(row, 0)?,
            MountCapability {
                id: uint(row, 1)?,
                flags: uint(row, 2)?,
                req_riding_skill: row.get::<_, i64>(3)? as u16,
                req_area: uint(row, 4)?,
                req_spell_aura: uint(row, 5)?,
                req_spell_known: uint(row, 6)?,
                mod_spell_aura: uint(row, 7)?,
                req_map: row.get::<_, i64>(8)? as i32,
                player_condition: uint(row, 9)?,
            },
        ))
    })? {
        let (mount_type, capability) = row?;
        store
            .capabilities
            .entry(mount_type)
            .or_default()
            .push(capability);
    }
    let mut statement = conn.prepare(
        "SELECT ID, AuraSpellLogic, AuraSpellID_0, AuraSpellID_1, AuraSpellID_2, AuraSpellID_3,
                AuraStacks_0, AuraStacks_1, AuraStacks_2, AuraStacks_3
         FROM player_condition WHERE ID IN
           (SELECT PlayerConditionID FROM mount_capability
            UNION SELECT PlayerConditionID FROM mount_x_display)",
    )?;
    for row in statement.query_map([], |row| {
        Ok((
            uint(row, 0)?,
            AuraCondition {
                logic: uint(row, 1)?,
                spells: [uint(row, 2)?, uint(row, 3)?, uint(row, 4)?, uint(row, 5)?],
                stacks: [uint(row, 6)?, uint(row, 7)?, uint(row, 8)?, uint(row, 9)?],
            },
        ))
    })? {
        let (id, condition) = row?;
        store.conditions.insert(id, condition);
    }
    Ok(store)
}

pub(super) fn load_mount_store_startup(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    let store = if path.exists() {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("Failed to open world.db for mounts");
        load_mount_store(&conn).expect("Failed to load mount tables from world.db")
    } else {
        warn!("world.db not found, mounts empty");
        MountStore::default()
    };
    commands.insert_resource(store);
}

fn mounted_effect(aura: &AuraInstance) -> Option<&super::AuraEffectState> {
    aura.effects
        .iter()
        .find(|effect| effect.aura_type == MOUNTED)
}

fn mark_dirty(world: &World, unit: Entity) {
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(unit);
    }
}

/// The rider state `GetMountCapability` reads, handed to `read`.
pub(crate) fn with_rider<R>(world: &World, unit: Entity, read: impl FnOnce(&Rider) -> R) -> R {
    let swimming = world
        .get::<PlayerMotion>(unit)
        .is_some_and(|motion| motion.contains(PlayerMotion::SWIMMING));
    let auras = world.get::<ActiveAuras>(unit);
    let aura_stacks = |spell: u32| {
        auras.map_or(0, |auras| {
            auras
                .iter()
                .filter(|aura| aura.spell_id == spell)
                .map(|aura| u32::from(aura.stacks.max(1)))
                .sum()
        })
    };
    let knows = |spell: u32| {
        world
            .get::<crate::profession::PlayerProfessions>(unit)
            .is_some_and(|professions| professions.knows(spell))
            || world
                .get::<crate::class_progression::KnownSpells>(unit)
                .is_some_and(|known| known.contains(spell))
    };
    let rider = Rider {
        riding_skill: world
            .get::<crate::profession::PlayerProfessions>(unit)
            .map_or(0, |professions| professions.rank(SKILL_RIDING)),
        submerged: swimming,
        map_id: world
            .get::<crate::maps::InMap>(unit)
            .map_or(0, |map| map.map_id),
        aura_stacks: &aura_stacks,
        knows: &knows,
    };
    read(&rider)
}

/// The display a mount aura shows (`HandleAuraMounted`, SpellAuraEffects.cpp:2763-2822).
fn mount_display(world: &World, unit: Entity, aura: &AuraInstance, creature: i32) -> Option<u32> {
    let mut rng = rand::thread_rng();
    if let Some(mount) = world
        .get_resource::<MountStore>()
        .and_then(|store| store.mount_for_spell(aura.spell_id))
    {
        if mount.flags & MOUNT_FLAG_SELF_MOUNT != 0 {
            return Some(DISPLAYID_HIDDEN_MOUNT);
        }
        let displays = with_rider(world, unit, |rider| {
            world
                .resource::<MountStore>()
                .usable_displays(mount.id, rider)
        });
        if let Some(&display) = rand::seq::SliceRandom::choose(displays.as_slice(), &mut rng) {
            return Some(display);
        }
    }
    let template = world
        .get_resource::<crate::world_data::WorldData>()?
        .templates
        .get(&u32::try_from(creature).ok()?)?;
    Some(crate::zones::template_display_id(template, &mut rng))
}

/// A MOUNTED aura was applied: mount the unit and cast its capability's speed aura.
pub(super) fn on_applied(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let Some(effect) = mounted_effect(aura) else {
        return;
    };
    let creature = effect.misc_values[0];
    let mount_type = world
        .get_resource::<MountStore>()
        .and_then(|store| store.mount_for_spell(aura.spell_id))
        .map_or(effect.misc_values[1] as u32, |mount| mount.mount_type);
    match mount_display(world, unit, aura, creature) {
        Some(mount_display_id) => {
            let vehicle_id = crate::vehicle::mount_vehicle_id(world, creature as u32);
            let mount_id = world
                .get_resource::<MountStore>()
                .and_then(|store| store.mount_for_spell(aura.spell_id))
                .map_or(0, |mount| mount.id);
            world.entity_mut(unit).insert(Mounted {
                mount_id,
                mount_display_id,
                vehicle_id,
                ..Default::default()
            });
            mark_dirty(world, unit);
        }
        None => warn!(
            "Mount aura {}: no display for mount or creature {creature}",
            aura.spell_id
        ),
    }
    let capability = world
        .get_resource::<MountStore>()
        .and_then(|store| with_rider(world, unit, |rider| store.capability(mount_type, rider)));
    let Some(capability) = capability else {
        return;
    };
    // `GetAmount()` holds the capability, for its removal.
    if let Some(mut auras) = world.get_mut::<ActiveAuras>(unit)
        && let Some(effect) = auras
            .get_mut(aura.instance_id)
            .and_then(|aura| aura.effects.iter_mut().find(|e| e.aura_type == MOUNTED))
    {
        effect.amount = capability.id as f32;
    }
    cast_on_self(world, unit, capability.mod_spell_aura);
}

/// `CastSpell(target, spell, triggeredByAura)` of an aura-only spell: its aura effects on
/// the unit itself.
fn cast_on_self(world: &mut World, unit: Entity, spell_id: u32) {
    let Some(mask) = world
        .get_resource::<crate::spell_info::SpellStore>()
        .and_then(|store| store.get(spell_id))
        .map(super::aura_effect_mask)
    else {
        warn!("Self-cast aura spell {spell_id} not found");
        return;
    };
    super::apply_aura(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
}

/// A MOUNTED aura left the unit: dismount, and drop the other mount auras and the
/// capability's speed aura.
pub(super) fn on_removed(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let Some(effect) = mounted_effect(aura) else {
        return;
    };
    let capability = effect.amount as u32;
    if world.get::<Mounted>(unit).is_some() {
        world.entity_mut(unit).remove::<Mounted>();
        mark_dirty(world, unit);
        super::interrupt_auras(world, unit, [AURA_INTERRUPT_DISMOUNT, 0], None);
    }
    super::remove_auras(world, unit, RemoveReason::Cancelled, |other| {
        other.instance_id != aura.instance_id && mounted_effect(other).is_some()
    });
    let speed_aura = world
        .get_resource::<MountStore>()
        .and_then(|store| store.capability_by_id(capability))
        .map(|capability| capability.mod_spell_aura);
    if let Some(speed_aura) = speed_aura {
        super::remove_auras(world, unit, RemoveReason::Cancelled, |other| {
            other.spell_id == speed_aura && other.caster == unit
        });
    }
}

/// `Unit::CancelMountAura` (Unit.cpp:8367-8377, `CMSG_CANCEL_MOUNT_AURA`): remove the
/// unit's cancellable mount auras. Whether it had one.
pub fn cancel_mount_aura(world: &mut World, unit: Entity) -> bool {
    let uncancellable: Vec<u32> = world
        .get::<ActiveAuras>(unit)
        .map(|auras| {
            auras
                .iter()
                .filter(|aura| mounted_effect(aura).is_some())
                .map(|aura| aura.spell_id)
                .filter(|&spell| {
                    super::lookup_spell(world, spell).is_none_or(|spell| {
                        super::attributes(&spell, 0) & super::ATTR0_NO_AURA_CANCEL != 0
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    !super::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        mounted_effect(aura).is_some()
            && !aura.harmful
            && !aura.passive
            && !uncancellable.contains(&aura.spell_id)
    })
    .is_empty()
}

/// `HandleCancelMountAuraOpcode` (SpellHandler.cpp:327-334) for each `CancelMountAura`.
pub(super) fn receive_cancel_mount_aura(world: &mut World) {
    let mut receivers = world.query_filtered::<(
        &mut lightyear::prelude::MessageReceiver<shared::protocol::CancelMountAura>,
        &crate::networking::PlayerLink,
    ), (With<crate::auth::Authenticated>, With<crate::auth::InWorld>)>(
    );
    let players: Vec<Entity> = receivers
        .iter_mut(world)
        .filter_map(|(mut receiver, link)| (receiver.receive().count() > 0).then_some(link.0))
        .collect();
    for player in players {
        cancel_mount_aura(world, player);
    }
}

/// `Unit::SetCanFly` / `Unit::SetCanAdvFly` (Unit.cpp:14027-14035) from the unit's flight
/// auras; losing an ability drops its flight (`MOVEMENTFLAG_MASK_MOVING_FLY`, ADV_FLYING).
pub(super) fn reconcile_can_fly(world: &mut World, unit: Entity, flight: Flight) {
    let Some(mut motion) = world.get_mut::<PlayerMotion>(unit) else {
        return;
    };
    let mut flags = motion.0;
    for (able, ability, flight) in [
        (
            flight.can_fly,
            PlayerMotion::CAN_FLY,
            PlayerMotion::FLYING | PlayerMotion::ADV_FLYING,
        ),
        (
            flight.can_adv_fly,
            PlayerMotion::CAN_ADV_FLY,
            PlayerMotion::ADV_FLYING,
        ),
    ] {
        flags = if able {
            flags | ability
        } else {
            flags & !(ability | flight)
        };
    }
    if flags == motion.0 {
        return;
    }
    let gained_adv_fly = !motion.contains(PlayerMotion::CAN_ADV_FLY) && flight.can_adv_fly;
    motion.0 = flags;
    mark_dirty(world, unit);
    if gained_adv_fly {
        crate::skyriding::fill_skyriding_bar(world, unit);
    }
}

/// What a unit's auras let it do in the air.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Flight {
    pub(super) can_fly: bool,
    pub(super) can_adv_fly: bool,
}

/// `auras`' FLY, MOD_INCREASE_MOUNTED_FLIGHT_SPEED and ADV_FLYING abilities.
pub(super) fn flight(auras: &ActiveAuras) -> Flight {
    let can_adv_fly = auras.has_type(ADV_FLYING);
    Flight {
        can_fly: can_adv_fly
            || auras.has_type(FLY)
            || auras.has_type(MOD_INCREASE_MOUNTED_FLIGHT_SPEED),
        can_adv_fly,
    }
}

/// Players entering the world get their saved Flight Style, Steady before they first
/// switched (`Player::_LoadAuras`).
pub(super) fn apply_flight_style(
    world: &mut World,
    entered: &mut QueryState<
        (Entity, Option<&crate::persistence::ClientId>),
        (With<Player>, Added<ActiveAuras>),
    >,
) {
    let players: Vec<(Entity, Option<u64>)> = entered
        .iter(world)
        .map(|(player, character)| (player, character.map(|character| character.0)))
        .collect();
    for (player, character) in players {
        let saved = character.and_then(|character| {
            let db = &world.get_resource::<crate::persistence::GameDatabase>()?.db;
            crate::persistence_progression::load_flight_style(db, character)
        });
        cast_on_self(world, player, saved.unwrap_or(FLIGHT_STYLE_STEADY));
    }
}

/// Flight Style: Skyriding.
pub const FLIGHT_STYLE_SKYRIDING: u32 = 404464;
/// Skyriding Basics, MountCapability 494's `ReqSpellKnownID`.
pub const SKYRIDING_BASICS: u32 = 376777;

/// The flight style auras: Steady 404468 and Skyriding 404464.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightStyle {
    Steady,
    Skyriding,
}

/// Put the player on `style`: its Flight Style aura replaces the other one and is saved
/// for the character's next login. The next mount picks its capability.
pub fn set_flight_style(world: &mut World, unit: Entity, style: FlightStyle) {
    let (on, off) = match style {
        FlightStyle::Steady => (FLIGHT_STYLE_STEADY, FLIGHT_STYLE_SKYRIDING),
        FlightStyle::Skyriding => (FLIGHT_STYLE_SKYRIDING, FLIGHT_STYLE_STEADY),
    };
    super::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == off
    });
    if !super::has_aura(world, unit, on) {
        cast_on_self(world, unit, on);
    }
    if let Some(character) = world.get::<crate::persistence::ClientId>(unit)
        && let Some(db) = world.get_resource::<crate::persistence::GameDatabase>()
    {
        crate::persistence_progression::save_flight_style(&db.db, character.0, on);
    }
}
