//! Controlled pets: hunter tame/call/revive, authored warlock demons and Mage elemental, persisted current pet,
//! shared spawn/follow/dismiss and demon death removal. TrinityCore a352b1fa:
//! `spell_hun_tame_beast` (spell_hunter.cpp:1255-1321), `Spell::EffectTameCreature`,
//! `EffectSummonPet`, `EffectDismissPet` (SpellEffects.cpp:2606-2728, 3586-3598), the
//! SUMMON_PET / DISMISS_PET checks of `Spell::CheckCast` (Spell.cpp:6454-6538),
//! `Unit::CreateTamedPetFrom` / `InitTamedPet` (Unit.cpp:11138-11215),
//! `Pet::CreateBaseAtCreature` (Pet.cpp:768-836) and `Player::RemovePet` (Player.cpp:22053).
//!
//! Tame Beast 1515 channels 6 s: APPLY_AURA DUMMY on the beast and a PERIODIC_TRIGGER_SPELL
//! (6 s) on the hunter that casts 13481, whose TAMECREATURE effect hits the channel target.
//! Pet bar/AI: pet_control.rs. Not modelled: full pet base stats, stable slots beyond
//! the five active ones, hunter pet death, and pets across map transfers.

use std::collections::HashMap;

use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::components::{
    CreatureClassification, CreatureMotion, Health, Level, ModelDisplay, MovementSpeed, Npc,
    Player, Position, PowerType, Rotation, UnitFactionTemplate, UnitFlags, UnitLevel, UnitPowers,
    UnitSummonedBy, Zone,
};
use shared::level_scaling::LevelScaling;
use shared::protocol::{NpcFlags, REACT_PASSIVE};
use shared::spell_data::CastFailReason;

use crate::auth::InWorld;
use crate::class_progression::KnownSpells;
use crate::creature_ai::FollowOwner;
use crate::interest::NetworkVisibility;
use crate::maps::InMap;
use crate::networking::{CombatTarget, PlayerLink};
use crate::persistence::GameDatabase;
use crate::persistence::pet_store::{StoredPet, StoredPetStable, load_pet_stable, save_pet_stable};
use crate::spell_cast::CastFailure;
use crate::spell_cast::effects::EffectCtx;
use crate::spell_cast::powers::{PetFocusRegen, PowerPools, PowerTypeTable};
use crate::spell_info::{SpellEffect, SpellInfo};
use crate::tame::{
    MAX_ACTIVE_PETS, PetStableSlots, TameFailReason, TameHunter, TameTarget, can_tame,
};
use crate::world_data::{CreatureTemplate, WorldData};

/// `CLASS_HUNTER`.
const CLASS_HUNTER: u8 = 3;
const CLASS_WARLOCK: u8 = 9;
const CLASS_MAGE: u8 = 8;
const SUMMON_WATER_ELEMENTAL: u32 = 31687;
const SUMMON_IMP: u32 = 688;
/// Local Retail SUMMON_PET spells; creature entries remain SpellEffect MiscValue data.
const DEMON_SUMMONS: [u32; 7] = [SUMMON_IMP, 691, 697, 712, 30146, 365349, 366222];
const IMP_FIREBOLT: u32 = 3110;
/// Tame Beast, whose `spell_hun_tame_beast::CheckCast` is `can_tame`.
const TAME_BEAST: u32 = 1515;
pub const EFFECT_TAME_CREATURE: u32 = 55;
pub const EFFECT_SUMMON_PET: u32 = 56;
pub const EFFECT_DISMISS_PET: u32 = 102;
/// Local Revive Pet982 effect109; TC a352b1fa EffectResurrectPet.
pub const EFFECT_RESURRECT_PET: u32 = 109;
/// `SPELL_AURA_ALLOW_TAME_PET_TYPE`: `Player::CanTameExoticPets`.
const AURA_ALLOW_TAME_PET_TYPE: u32 = 146;
/// `DEFAULT_PLAYER_COMBAT_REACH`: `GetClosePoint(..., owner->GetCombatReach())`.
const OWNER_COMBAT_REACH: f32 = 1.5;

pub struct HunterPetPlugin;

impl Plugin for HunterPetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PetCatalog>()
            .add_systems(Startup, load_pet_catalog)
            .add_systems(Update, load_pet_stables)
            .add_systems(
                Update,
                crate::pet_stable::process_pet_stable_requests
                    .after(crate::npc_interaction::NpcInteractionsProcessed)
                    .after(load_pet_stables),
            )
            .add_systems(
                FixedUpdate,
                persist_changed_pet_health.after(crate::combat::auto_attack_system),
            )
            .add_systems(FixedUpdate, remove_ownerless_pets)
            .add_systems(
                FixedUpdate,
                remove_dead_summoned_pets.after(crate::combat::auto_attack_system),
            );
    }
}

/// Imported Retail family defaults and normal (non-override) hunter pet specs.
#[derive(Resource, Default)]
pub(crate) struct PetCatalog {
    pub default_specializations: HashMap<u32, u32>,
    pub families: HashMap<u32, (String, u32)>,
    pub specializations: Vec<shared::protocol::PetSpecializationInfo>,
}

fn load_pet_catalog(mut commands: Commands) {
    #[cfg(test)]
    let path = crate::test_data::world_db_path();
    #[cfg(not(test))]
    let path = std::path::PathBuf::from(crate::spell_info::WORLD_DB_PATH);
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Open world pet catalog");
    commands.insert_resource(read_pet_catalog(&conn).expect("Load Retail pet family/spec catalog"));
}

pub(crate) fn read_pet_catalog(conn: &Connection) -> rusqlite::Result<PetCatalog> {
    // TC a352b1fa ChrSpecializationFlag::PetOverrideSpec=0x20: separate class index.
    let mut families = conn.prepare("SELECT f.ID, s.ID FROM creature_family f JOIN chr_specialization s ON s.ClassID=0 AND s.Flags & 32=0 AND s.PetTalentType=f.PetTalentType")?;
    let default_specializations = families
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    let mut family_names = conn.prepare("SELECT ID, Name_lang, IconFileID FROM creature_family")?;
    let families = family_names
        .query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    let mut specs = conn.prepare("SELECT ID, Name_lang FROM chr_specialization WHERE ClassID=0 AND Flags & 32=0 ORDER BY OrderIndex")?;
    let specializations = specs
        .query_map([], |row| {
            Ok(shared::protocol::PetSpecializationInfo {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(PetCatalog {
        default_specializations,
        families,
        specializations,
    })
}

fn default_pet_specialization(world: &World, family: u32) -> Option<u32> {
    world
        .resource::<PetCatalog>()
        .default_specializations
        .get(&family)
        .copied()
}

/// A hunter's `PetStable`, inserted with the first tamed pet or the stored stable at
/// login. `summoned` is the pet out in the world.
#[derive(Component, Debug, Clone)]
pub struct PetStable {
    /// None for runtime-only authenticated NPC companions; never a synthetic ID.
    pub character_id: Option<u64>,
    pub stored: StoredPetStable,
    pub summoned: Option<Entity>,
}

/// A controlled pet's owner and storage slot. Warlock demons reuse slot 0 internally;
/// it is not a hunter Call Pet slot exposed to the warlock.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HunterPet {
    pub owner: Entity,
    pub slot: u8,
}

/// Server-only SUMMON_PET kind: demon stat contribution; removed when dead.
#[derive(Component)]
pub(crate) struct SummonedDemon;

/// A Tame Beast, Call Pet or Dismiss Pet check failing (`crate::spell_cast::CastFailure::Pet`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetCastFailure {
    Tame(TameFailReason),
    /// `SPELL_FAILED_BAD_IMPLICIT_TARGETS`: Tame Beast at a unit that is no creature.
    NotACreature,
    /// `SPELL_FAILED_NO_PET`: Dismiss Pet without a pet out.
    NoPet,
    /// `PetTameResult::NoPetAvailable`: Call Pet of an empty slot.
    NoPetAvailable,
}

impl PetCastFailure {
    /// The contract reason and the Retail GlobalStrings text (no `CastFailReason` carries
    /// these yet, as with `SPELL_FAILED_CONFUSED`).
    pub(crate) fn to_contract(self) -> (CastFailReason, Option<String>) {
        use CastFailReason as R;
        let (reason, text) = match self {
            // SPELL_FAILED_BAD_TARGETS
            Self::Tame(TameFailReason::NotTameable) => return (R::InvalidTarget, None),
            Self::Tame(TameFailReason::TooHighLevel) => {
                (R::InvalidTarget, "Target is too high level")
            }
            Self::Tame(TameFailReason::AlreadyHaveSummon) => {
                (R::NotSupported, "You already control a summoned creature")
            }
            Self::Tame(TameFailReason::TooManyPets) => {
                (R::NotSupported, "You have too many pets already")
            }
            Self::Tame(TameFailReason::AlreadyHaveCharm) => {
                (R::NotSupported, "You already control a charmed creature")
            }
            Self::Tame(TameFailReason::CreatureAlreadyOwned) => {
                (R::InvalidTarget, "Creature is already controlled")
            }
            Self::NotACreature => (R::InvalidTarget, "No target"),
            Self::NoPet => (R::NotSupported, "You do not have a pet."),
            Self::NoPetAvailable => (R::NotSupported, "You do not have a pet to summon"),
        };
        (reason, Some(text.to_owned()))
    }
}

/// The pet `owner` has out (`Unit::GetPet`).
pub(crate) fn summoned_pet(world: &World, owner: Entity) -> Option<Entity> {
    world
        .get::<PetStable>(owner)
        .and_then(|stable| stable.summoned)
        .filter(|&pet| world.get_entity(pet).is_ok())
}

/// The pet checks of `Spell::CheckCast`: Tame Beast's script, then the SUMMON_PET and
/// DISMISS_PET effect checks.
pub(crate) fn check_cast(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> Result<(), CastFailure> {
    if spell.id == TAME_BEAST {
        check_tame(world, caster, explicit).map_err(CastFailure::Pet)?;
    }
    for effect in &spell.effects {
        match effect.effect {
            EFFECT_SUMMON_PET => check_summon_pet(world, caster, spell, effect)?,
            EFFECT_RESURRECT_PET => check_revive_pet(world, caster)?,
            EFFECT_DISMISS_PET if summoned_pet(world, caster).is_none() => {
                return Err(CastFailure::Pet(PetCastFailure::NoPet));
            }
            _ => {}
        }
    }
    Ok(())
}

/// `spell_hun_tame_beast::CheckCast` for a player caster and its explicit target.
fn check_tame(
    world: &World,
    caster: Entity,
    explicit: Option<Entity>,
) -> Result<(), PetCastFailure> {
    let target = explicit
        .filter(|&target| world.get::<Npc>(target).is_some())
        .ok_or(PetCastFailure::NotACreature)?;
    let npc = world.get::<Npc>(target).expect("filtered on Npc");
    let template = world
        .get_resource::<WorldData>()
        .and_then(|data| data.template_for(npc.template_id, 0))
        .ok_or(PetCastFailure::NotACreature)?;
    let level = world.get::<Level>(caster).map_or(1, |level| level.0);
    let target_level = shared::level_scaling::level_for_viewer(
        world
            .get::<UnitLevel>(target)
            .copied()
            .unwrap_or(UnitLevel(1)),
        world.get::<LevelScaling>(target),
        level,
    );
    let stable = world.get::<PetStable>(caster).map(|stable| PetStableSlots {
        pet_out: stable.summoned.is_some(),
        active: std::array::from_fn(|slot| stable.stored.active[slot].is_some()),
    });
    let known: Vec<u32> = world
        .get::<KnownSpells>(caster)
        .map(|known| known.iter().collect())
        .unwrap_or_default();
    can_tame(
        &TameHunter {
            level,
            can_tame_exotic: has_aura_type(world, caster, AURA_ALLOW_TAME_PET_TYPE),
            pet_stable: stable.as_ref(),
            known_spells: &known,
            // No charm is modelled.
            has_charm: false,
        },
        &TameTarget {
            template,
            level: target_level,
            owned: world.get::<HunterPet>(target).is_some(),
        },
    )
    .map_err(PetCastFailure::Tame)
}

fn has_aura_type(world: &World, unit: Entity, aura_type: u32) -> bool {
    world
        .get::<crate::auras::ActiveAuras>(unit)
        .is_some_and(|auras| {
            auras
                .iter()
                .flat_map(|aura| &aura.effects)
                .any(|effect| effect.aura_type == aura_type)
        })
}

/// SUMMON_PET of a player with a stable (MiscValue 0, the slot in its value): an empty
/// slot has no pet available.
fn check_summon_pet(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> Result<(), CastFailure> {
    if effect.misc_values[0] != 0 {
        let class = owner_class(world, caster);
        let demon = DEMON_SUMMONS.contains(&spell.id) && class == Some(CLASS_WARLOCK);
        let elemental = spell.id == SUMMON_WATER_ELEMENTAL && class == Some(CLASS_MAGE);
        if !(demon || elemental) {
            return Err(CastFailure::Unsupported(
                "Unsupported controlled pet summon".into(),
            ));
        }
        let entry = effect.misc_values[0] as u32;
        let template = world
            .get_resource::<WorldData>()
            .and_then(|data| data.template_for(entry, 0));
        if template.is_none_or(|template| template.models.is_empty()) {
            return Err(CastFailure::Unsupported(format!(
                "Missing summoned pet template/model {entry}"
            )));
        }
        return Ok(());
    }
    let Some(stable) = world.get::<PetStable>(caster) else {
        return Err(CastFailure::Pet(PetCastFailure::NoPetAvailable));
    };
    let stored = stable
        .stored
        .active
        .get(effect_slot(effect))
        .and_then(Option::as_ref);
    if stored.is_none() {
        return Err(CastFailure::Pet(PetCastFailure::NoPetAvailable));
    }
    Ok(())
}

/// Call Pet N's slot: its effect value (`GetEffectValueAsInt`), 0 for Call Pet 1.
fn effect_slot(effect: &SpellEffect) -> usize {
    effect.base_points.round().max(0.0) as usize
}

/// `Spell::EffectTameCreature` on the channel target: the hunter's channel finishes, the
/// creature becomes a pet at the hunter's level in the first free active slot, out at
/// once, and the creature itself despawns.
pub(crate) fn effect_tame_creature(
    world: &mut World,
    ctx: &EffectCtx,
    _effect: &SpellEffect,
    target: Entity,
) {
    let hunter = ctx.caster;
    if summoned_pet(world, hunter).is_some()
        || world.get::<HunterPet>(target).is_some()
        || world.get::<Player>(hunter).map(|player| player.class) != Some(CLASS_HUNTER)
    {
        return;
    }
    let Some(npc) = world.get::<Npc>(target).cloned() else {
        return;
    };
    crate::spell_cast::end_cast(world, hunter);
    let Some(pet) = tamed_pet(world, hunter, target, &npc) else {
        return;
    };
    let Some(slot) = first_free_slot(world, hunter) else {
        return;
    };
    let Some(at) = world.get::<Position>(target).copied() else {
        return;
    };
    let facing = world
        .get::<Rotation>(target)
        .map_or(0.0, |rotation| rotation.y);
    crate::combat::despawn_creature(world, target);
    if let Some(mut stable) = world.get_mut::<PetStable>(hunter) {
        stable.stored.active[usize::from(slot)] = Some(pet);
    }
    put_pet_out(world, hunter, slot, at, facing);
    info!("{hunter} tamed {} ({target}) into slot {slot}", npc.name);
}

/// `Pet::CreateBaseAtCreature` + `InitStatsForLevel`: the creature's template, model and
/// name at the hunter's level. TrinityCore names it after its `CreatureFamily`, which is
/// not imported; the creature's name is TrinityCore's name without a family.
fn tamed_pet(world: &World, hunter: Entity, creature: Entity, npc: &Npc) -> Option<StoredPet> {
    let template = world
        .get_resource::<WorldData>()?
        .template_for(npc.template_id, 0)?;
    Some(StoredPet {
        creature_entry: npc.template_id,
        display_id: world.get::<ModelDisplay>(creature)?.display_id,
        name: npc.name.clone(),
        level: world.get::<Level>(hunter).map_or(1, |level| level.0),
        family: template.family,
        // The new pet's `CharmInfo` sets it Passive (`CharmInfo::CharmInfo`).
        react_state: REACT_PASSIVE,
        health: None,
        specialization: default_pet_specialization(world, template.family),
    })
}

/// `Player::GetOrInitPetStable`: attach this character's pet storage once.
pub(crate) fn init_pet_stable(world: &mut World, owner: Entity) -> Option<()> {
    if world.get::<PetStable>(owner).is_none() {
        let character_id = if world.get::<crate::followers::Follower>(owner).is_some() {
            None
        } else {
            Some(character_id(world, owner)?)
        };
        world.entity_mut(owner).insert(PetStable {
            character_id,
            stored: StoredPetStable::default(),
            summoned: None,
        });
    }
    Some(())
}

fn first_free_slot(world: &mut World, hunter: Entity) -> Option<u8> {
    init_pet_stable(world, hunter)?;
    let stable = world.get::<PetStable>(hunter)?;
    let slot = stable.stored.active.iter().position(Option::is_none)?;
    u8::try_from(slot).ok()
}

/// The character the player entity plays (its connection's `InWorld`).
fn character_id(world: &mut World, player: Entity) -> Option<u64> {
    world
        .query::<(&PlayerLink, &InWorld)>()
        .iter(world)
        .find_map(|(link, in_world)| (link.0 == player).then_some(in_world.character_id))
}

/// `Spell::EffectSummonPet` (TrinityCore a352b1fa, SpellEffects.cpp): MiscValue is
/// the creature entry, or 0 for a hunter's slot. The same living pet is recalled;
/// Authored demon summons create/load through the existing controlled-pet path.
pub(crate) fn effect_summon_pet(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    let owner = ctx.caster;
    if owner_class(world, owner).is_none() {
        return;
    }
    let Some((at, facing)) = close_point(world, owner) else {
        return;
    };
    let entry = effect.misc_values[0] as u32;
    if let Some(pet) = summoned_pet(world, owner)
        && (entry == 0
            || world
                .get::<Npc>(pet)
                .is_some_and(|npc| npc.template_id == entry))
    {
        if world.get::<Health>(pet).expect("pet health").current <= 0.0 {
            return;
        }
        if let Some(mut position) = world.get_mut::<Position>(pet) {
            *position = at;
        }
        crate::interest::mark_interest_position_changed(
            world
                .get_resource::<crate::interest::InterestDirtyQueue>()
                .map(|queue| &*queue),
            pet,
        );
        crate::pet_control::send_pet_spells(world, owner, pet);
        return;
    }
    let pet = if entry != 0 {
        summon_authored_pet(world, owner, entry, at, facing)
    } else {
        let slot = effect_slot(effect);
        let stored = world
            .get::<PetStable>(owner)
            .and_then(|stable| stable.stored.active.get(slot).cloned().flatten());
        if stored.is_none() || slot >= MAX_ACTIVE_PETS {
            return;
        }
        put_pet_out(world, owner, slot as u8, at, facing);
        summoned_pet(world, owner)
    };
    if let Some(pet) = pet {
        crate::spell_cast::log_summon(world, owner, pet, ctx.spell);
    }
}

/// `Player::SummonPet` loads an existing record or creates a SUMMON_PET from its
/// creature template at the owner's level, then saves it as current. CheckCast admits
/// sourced demon spells and Mage31687; timed guardians use their own path.
fn summon_authored_pet(
    world: &mut World,
    owner: Entity,
    entry: u32,
    at: Position,
    facing: f32,
) -> Option<Entity> {
    let stored = create_or_reuse_summon_record(world, owner, entry);
    if init_pet_stable(world, owner).is_none() {
        error!("Summoned pet owner {owner} has no in-world character");
        return None;
    }
    if let Some(pet) = summoned_pet(world, owner) {
        remove_pet(world, pet);
    }
    world
        .get_mut::<PetStable>(owner)
        .expect("stable initialized")
        .stored
        .active[0] = Some(stored);
    put_pet_out(world, owner, 0, at, facing);
    summoned_pet(world, owner)
}

fn create_or_reuse_summon_record(world: &World, owner: Entity, entry: u32) -> StoredPet {
    let level = world.get::<Level>(owner).expect("summoning player level").0;
    let existing = world
        .get::<PetStable>(owner)
        .and_then(|stable| stable.stored.active[0].clone())
        .filter(|stored| stored.creature_entry == entry);
    match existing {
        Some(mut stored) => {
            stored.level = level;
            // Explicit summoning creates a living demon, unlike current-pet relog.
            // Same living demon recall returns before this record path.
            stored.health = None;
            stored
        }
        None => {
            let data = world.resource::<WorldData>();
            let template = data
                .template_for(entry, 0)
                .expect("summon template validated");
            StoredPet {
                creature_entry: entry,
                display_id: crate::zones::template_display_id(template, &mut rand::thread_rng()),
                name: template.name.clone(),
                level,
                family: template.family,
                react_state: REACT_PASSIVE,
                health: None,
                specialization: None,
            }
        }
    }
}

fn first_dead_active_slot(world: &World, owner: Entity) -> Option<u8> {
    world
        .get::<PetStable>(owner)?
        .stored
        .active
        .iter()
        .position(|pet| pet.as_ref().is_some_and(|pet| pet.health == Some(0.0)))
        .map(|slot| slot as u8)
}

/// TC Spell.cpp RESURRECT_PET checks: reject a living summon, otherwise require
/// a dead current pet or the first dead active record (not a stable-storage pet).
fn check_revive_pet(world: &World, owner: Entity) -> Result<(), CastFailure> {
    if world.get::<Player>(owner).map(|player| player.class) != Some(CLASS_HUNTER) {
        return Err(CastFailure::InvalidTarget);
    }
    if let Some(pet) = summoned_pet(world, owner) {
        if world
            .get::<Health>(pet)
            .expect("controlled pet health")
            .current
            > 0.0
        {
            return Err(CastFailure::Pet(PetCastFailure::Tame(
                TameFailReason::AlreadyHaveSummon,
            )));
        }
        return Ok(());
    }
    first_dead_active_slot(world, owner)
        .map(|_| ())
        .ok_or(CastFailure::InvalidTarget)
}

pub(crate) fn effect_revive_pet(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    let owner = ctx.caster;
    if check_revive_pet(world, owner).is_err() {
        return;
    }
    let Some((at, facing)) = close_point(world, owner) else {
        return;
    };
    if summoned_pet(world, owner).is_none() {
        let slot = first_dead_active_slot(world, owner).expect("dead pet validated");
        put_pet_out(world, owner, slot, at, facing);
    }
    let Some(pet) = summoned_pet(world, owner) else {
        return;
    };
    let percentage =
        crate::spell_cast::effects::modded_base_points(world, owner, ctx.spell, effect);
    if percentage < 0.0 {
        return;
    }
    restore_pet_life(world, pet, at, percentage);
    persist_current_pet_health(world, owner);
    crate::pet_control::send_pet_spells(world, owner, pet);
}

fn restore_pet_life(world: &mut World, pet: Entity, at: Position, percentage: f32) {
    crate::combat::combat_stop(world, pet);
    world
        .entity_mut(pet)
        .remove::<(crate::combat::DeathState, crate::combat::Corpse)>();
    let mut health = world.get_mut::<Health>(pet).expect("controlled pet health");
    health.current = (health.max * percentage / 100.0).min(health.max);
    *world
        .get_mut::<Position>(pet)
        .expect("controlled pet position") = at;
    if let Some(mut charm) = world.get_mut::<crate::pet_control::PetCharm>(pet) {
        charm.command_attack = false;
        charm.follow_ordered = false;
        charm.stay_at = (charm.command == shared::protocol::COMMAND_STAY).then_some(at);
    }
    crate::interest::set_hidden(world, pet, false);
    crate::interest::mark_interest_position_changed(
        world
            .get_resource::<crate::interest::InterestDirtyQueue>()
            .map(|queue| &*queue),
        pet,
    );
    if let Some(queue) = world.get_resource::<ReplicationDirtyQueue>() {
        queue.mark(pet);
    }
}

/// `WorldObject::GetClosePoint(owner->GetCombatReach())`: the owner's combat reach ahead
/// of it, facing its way.
fn close_point(world: &World, owner: Entity) -> Option<(Position, f32)> {
    let position = *world.get::<Position>(owner)?;
    let rotation_y = world
        .get::<Rotation>(owner)
        .map_or(0.0, |rotation| rotation.y);
    let orientation = rotation_y - std::f32::consts::FRAC_PI_2;
    // WoW (x, y) offsets are Bevy (x, -z).
    let at = Position {
        x: position.x + OWNER_COMBAT_REACH * orientation.cos(),
        y: position.y,
        z: position.z - OWNER_COMBAT_REACH * orientation.sin(),
    };
    Some((at, rotation_y))
}

/// `Spell::EffectDismissPet` on the pet: `Pet::Remove(PET_SAVE_NOT_IN_SLOT)` keeps it in its
/// slot, no longer out.
pub(crate) fn effect_dismiss_pet(
    world: &mut World,
    _ctx: &EffectCtx,
    _effect: &SpellEffect,
    pet: Entity,
) {
    remove_pet(world, pet);
}

/// TC a352b1fa PetHandler.cpp:217-238: command 3 deletes hunter pets, but
/// keeps a SUMMON_PET record for its next explicit summon.
pub(crate) fn abandon_or_dismiss_pet(world: &mut World, pet: Entity) {
    let summoned = world
        .get::<HunterPet>(pet)
        .is_some_and(|pet| is_summoned_pet_owner(world, pet.owner));
    if world.get::<SummonedDemon>(pet).is_none() && !summoned {
        let Some((owner, slot)) = world
            .get::<HunterPet>(pet)
            .map(|pet| (pet.owner, usize::from(pet.slot)))
        else {
            return;
        };
        if let Some(mut stable) = world.get_mut::<PetStable>(owner) {
            stable.stored.active[slot] = None;
        }
    }
    remove_pet(world, pet);
}

pub(crate) fn remove_pet(world: &mut World, pet: Entity) {
    let Some(owner) = world.get::<HunterPet>(pet).map(|pet| pet.owner) else {
        return;
    };
    persist_current_pet_health(world, owner);
    crate::combat::combat_stop(world, pet);
    world.entity_mut(pet).despawn();
    if let Some(mut stable) = world.get_mut::<PetStable>(owner) {
        stable.stored.current = None;
        stable.summoned = None;
    }
    save_stable(world, owner);
    crate::pet_control::clear_pet_spells(world, owner);
}

/// Summon the pet of `slot` at `at` as the owner's current pet and save the stable.
fn put_pet_out(world: &mut World, owner: Entity, slot: u8, at: Position, facing: f32) {
    let Some((pet, stored)) = spawn_pet(world, owner, slot, at, facing) else {
        return;
    };
    if let Some(mut stable) = world.get_mut::<PetStable>(owner) {
        stable.stored.active[usize::from(slot)] = Some(stored.clone());
        stable.stored.current = Some(slot);
        stable.summoned = Some(pet);
    }
    save_stable(world, owner);
    crate::pet_control::init_pet_control(
        world,
        owner,
        pet,
        stored.family,
        stored.level,
        stored.react_state,
    );
}

/// The pet's new stance kept in its stable slot and saved.
pub(crate) fn save_pet_react_state(world: &mut World, owner: Entity, pet: Entity, react: u32) {
    let Some(slot) = world.get::<HunterPet>(pet).map(|pet| usize::from(pet.slot)) else {
        return;
    };
    let Some(mut stable) = world.get_mut::<PetStable>(owner) else {
        return;
    };
    let Some(stored) = stable.stored.active.get_mut(slot).and_then(Option::as_mut) else {
        return;
    };
    if stored.react_state == react {
        return;
    }
    stored.react_state = react;
    save_stable(world, owner);
}

/// The pet of the owner's `slot` as a creature: its template's entry and classification,
/// the stored model/name, owner's level for demons, full health and Focus for a hunter
/// pet (`Pet::CreateBaseAtTamed`), or Firebolt's DB2 power pool for an imp, the owner's
/// faction and map, player controlled, summoned by its owner (`SetOwnerGUID` →
/// `SummonedBy`), following it (`PET_FOLLOW_DIST`, `PET_FOLLOW_ANGLE`). Returns it with
/// its stored record.
fn spawn_pet(
    world: &mut World,
    owner: Entity,
    slot: u8,
    at: Position,
    facing: f32,
) -> Option<(Entity, StoredPet)> {
    let mut stored = world
        .get::<PetStable>(owner)?
        .stored
        .active
        .get(usize::from(slot))?
        .clone()?;
    let demon = owner_class(world, owner) == Some(CLASS_WARLOCK);
    let summoned = is_summoned_pet_owner(world, owner);
    if summoned {
        // Pet::LoadPetFromDB: SUMMON_PET always takes the owner's level.
        stored.level = world.get::<Level>(owner).expect("summoning player level").0;
    }
    let (health, classification, mana, flags_extra) = {
        let data = world.get_resource::<WorldData>()?;
        let Some(template) = data.template_for(stored.creature_entry, 0) else {
            error!(
                "Pet creature template {} does not exist",
                stored.creature_entry
            );
            return None;
        };
        (
            pet_max_health(data, template, stored.level),
            CreatureClassification::from_rank(template.rank)
                .expect("creature_template.rank is validated when loaded"),
            (owner_class(world, owner) == Some(CLASS_MAGE))
                .then(|| authored_pet_mana(data, template, stored.level)),
            template.flags_extra,
        )
    };
    let map = *world.get::<InMap>(owner)?;
    let zone = world.get::<Zone>(owner).copied().unwrap_or(Zone { id: 0 });
    let faction = *world.get::<UnitFactionTemplate>(owner)?;
    let replicate = world.get::<Replicate>(owner).is_some();
    let companion = world.get::<crate::followers::Follower>(owner).is_some();
    let power = match owner_class(world, owner)? {
        CLASS_HUNTER => PowerType::Focus,
        CLASS_WARLOCK => demon_spell_power(world),
        CLASS_MAGE => PowerType::Mana,
        class => {
            error!("Unsupported controlled pet owner class {class}");
            return None;
        }
    };
    let pools = PowerPools::from_table(world.resource::<PowerTypeTable>(), vec![power]);
    let current_health = stored.health.unwrap_or(health).clamp(0.0, health);
    stored.health = Some(current_health);
    if !summoned && stored.specialization.is_none() {
        stored.specialization = default_pet_specialization(world, stored.family);
    }
    let mut pet = world.spawn((
        (
            at,
            Rotation {
                x: 0.0,
                y: facing,
                z: 0.0,
            },
            Npc {
                template_id: stored.creature_entry,
                name: stored.name.clone(),
            },
            crate::zones::CreatureFlagsExtra(flags_extra),
            Health {
                current: current_health,
                max: health,
            },
            ModelDisplay {
                display_id: stored.display_id,
            },
            zone,
            map,
        ),
        (
            UnitLevel(stored.level),
            classification,
            UnitPowers::default(),
            faction,
            UnitFlags(crate::faction::UNIT_FLAG_PLAYER_CONTROLLED),
            NpcFlags(0),
            CreatureMotion::Still,
            MovementSpeed(0.0),
            HunterPet { owner, slot },
            FollowOwner::new(owner),
            UnitSummonedBy(owner.to_bits()),
            shared::components::UnitIsPet(true),
            pools,
            CombatTarget::default(),
        ),
    ));
    if power == PowerType::Focus {
        pet.insert(PetFocusRegen::default());
    }
    if companion {
        pet.insert(crate::followers::FollowerReach(
            crate::creature_ai::CHASE_REACH,
        ));
    }
    if demon {
        pet.insert(SummonedDemon);
    }
    if let Some(max) = mana {
        pet.insert(shared::components::Mana { current: max, max });
    }
    if replicate {
        pet.insert((
            Replicate::to_clients(NetworkTarget::All),
            NetworkVisibility::default(),
        ));
    }
    Some((pet.id(), stored))
}

fn owner_class(world: &World, owner: Entity) -> Option<u8> {
    if let Some(player) = world.get::<Player>(owner) {
        return Some(player.class);
    }
    world
        .get::<crate::followers::Follower>(owner)
        .map(|follower| follower.profile.class_id)
}

pub(crate) fn is_summoned_pet_owner(world: &World, owner: Entity) -> bool {
    matches!(owner_class(world, owner), Some(CLASS_WARLOCK | CLASS_MAGE))
}

fn authored_pet_mana(data: &WorldData, template: &CreatureTemplate, level: u8) -> f32 {
    let base = data
        .class_level_base_mana
        .get(&(level, template.unit_class))
        .expect("controlled mana pet class/level base mana imported");
    (*base as f32 * template.mana_modifier).round()
}

/// The admitted local demon families15/16/17/23/29 all use SpellPower.PowerType3.
/// Firebolt3110 supplies that shared Energy type; PowerTypeTable supplies max/regen.
fn demon_spell_power(world: &World) -> PowerType {
    let firebolt = world
        .resource::<crate::spell_info::SpellStore>()
        .get(IMP_FIREBOLT)
        .expect("Imp Firebolt imported");
    let cost = firebolt
        .power_costs
        .first()
        .expect("Imp Firebolt power cost imported");
    PowerType::from_db(cost.power_type).expect("Imp Firebolt supported power pool")
}

/// `Guardian::InitStatsForLevel` without `pet_levelstats` (not imported): ExpectedStat
/// CreatureHealth at the pet's level × the template's HealthModifier; an untuned
/// AzerothCore template takes `creature_classlevelstats` (`CreatureBaseStats::GenerateHealth`).
pub(crate) fn pet_max_health(data: &WorldData, template: &CreatureTemplate, level: u8) -> f32 {
    if let Some(tuning) = &template.tuning {
        return tuning.stats.max_health(level).max(1.0);
    }
    let base = data
        .class_level_base_hp
        .get(&(level, template.unit_class))
        .map_or(1, |hp| hp[usize::from(template.expansion)]);
    (base as f32 * template.health_modifier).ceil().max(1.0)
}

pub(crate) fn persist_current_pet_health(world: &mut World, owner: Entity) {
    let Some(pet) = summoned_pet(world, owner) else {
        return;
    };
    let Some(health) = world.get::<Health>(pet).map(|health| health.current) else {
        return;
    };
    let slot = world
        .get::<HunterPet>(pet)
        .expect("owned controlled pet")
        .slot;
    let Some(mut stable) = world.get_mut::<PetStable>(owner) else {
        return;
    };
    let Some(record) = stable.stored.active[usize::from(slot)].as_mut() else {
        return;
    };
    if record.health == Some(health) {
        return;
    }
    record.health = Some(health);
    save_stable(world, owner);
}

fn persist_changed_pet_health(world: &mut World) {
    let owners: Vec<_> = world
        .query_filtered::<&HunterPet, Changed<Health>>()
        .iter(world)
        .map(|pet| pet.owner)
        .collect();
    for owner in owners {
        persist_current_pet_health(world, owner);
    }
}

pub(crate) fn save_stable(world: &World, owner: Entity) {
    let (Some(db), Some(stable)) = (
        world.get_resource::<GameDatabase>(),
        world.get::<PetStable>(owner),
    ) else {
        return;
    };
    if let Some(character_id) = stable.character_id {
        save_pet_stable(&db.db, character_id, &stable.stored);
    }
}

/// At world entry: the stored stable, and `Player::LoadPet` re-summons the pet that was
/// out at logout (`PET_SAVE_AS_CURRENT`).
fn load_pet_stables(
    entered: Query<(&PlayerLink, &InWorld), Added<InWorld>>,
    db: Option<Res<GameDatabase>>,
    mut commands: Commands,
) {
    let Some(db) = db else {
        return;
    };
    for (link, in_world) in &entered {
        let Some(stored) = load_pet_stable(&db.db, in_world.character_id) else {
            continue;
        };
        let (player, character_id) = (link.0, in_world.character_id);
        commands.queue(move |world: &mut World| {
            restore_pet_stable(world, player, character_id, stored);
        });
    }
}

fn restore_pet_stable(
    world: &mut World,
    player: Entity,
    character_id: u64,
    stored: StoredPetStable,
) {
    let Ok(mut entity) = world.get_entity_mut(player) else {
        return;
    };
    let current = stored.current;
    entity.insert(PetStable {
        character_id: Some(character_id),
        stored,
        summoned: None,
    });
    if let Some(slot) = current
        && let Some((at, facing)) = close_point(world, player)
    {
        put_pet_out(world, player, slot, at, facing);
    }
}

/// A pet whose owner left the world (logout: `Player::RemovePet(PET_SAVE_AS_CURRENT)`)
/// or no longer has it out leaves the world; its slot stays as saved.
fn remove_ownerless_pets(
    pets: Query<(Entity, &HunterPet)>,
    owners: Query<&PetStable, Without<crate::networking::PendingDespawn>>,
    mut commands: Commands,
) {
    for (pet, hunter_pet) in &pets {
        let still_out = owners
            .get(hunter_pet.owner)
            .is_ok_and(|stable| stable.summoned == Some(pet));
        if !still_out {
            commands.entity(pet).despawn();
        }
    }
}

/// Pet::Update: a non-hunter corpse is removed with PET_SAVE_NOT_IN_SLOT.
/// Keep the record for another explicit summon, but never restore it at login.
fn remove_dead_summoned_pets(world: &mut World) {
    let dead: Vec<Entity> = world
        .query_filtered::<(Entity, &Health, Option<&HunterPet>, Option<&SummonedDemon>), Or<(With<HunterPet>, With<SummonedDemon>)>>()
        .iter(world)
        .filter(|(_, health, pet, demon)| {
            let summoned = demon.is_some() || pet.is_some_and(|pet| is_summoned_pet_owner(world, pet.owner));
            health.current <= 0.0 && summoned
        })
        .map(|(pet, ..)| pet)
        .collect();
    for pet in dead {
        remove_pet(world, pet);
    }
}

#[cfg(test)]
#[path = "hunter_pet_tests.rs"]
mod tests;
