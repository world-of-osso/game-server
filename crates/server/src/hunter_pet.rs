//! Hunter pets: taming with Tame Beast, the persisted pet stable, Call Pet 1-5 and Dismiss
//! Pet, and the summoned pet following its owner. TrinityCore a352b1fa:
//! `spell_hun_tame_beast` (spell_hunter.cpp:1255-1321), `Spell::EffectTameCreature`,
//! `EffectSummonPet`, `EffectDismissPet` (SpellEffects.cpp:2606-2728, 3586-3598), the
//! SUMMON_PET / DISMISS_PET checks of `Spell::CheckCast` (Spell.cpp:6454-6538),
//! `Unit::CreateTamedPetFrom` / `InitTamedPet` (Unit.cpp:11138-11215),
//! `Pet::CreateBaseAtCreature` (Pet.cpp:768-836) and `Player::RemovePet` (Player.cpp:22053).
//!
//! Tame Beast 1515 channels 6 s: APPLY_AURA DUMMY on the beast and a PERIODIC_TRIGGER_SPELL
//! (6 s) on the hunter that casts 13481, whose TAMECREATURE effect hits the channel target.
//! Not modelled: pet stats beyond health, pet AI and commands, stable slots beyond the five
//! active ones, pet death, and taking the pet along through a map transfer.

use bevy::prelude::*;
use lightyear::prelude::*;
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
use crate::spell_cast::powers::{PowerPools, PowerTypeTable};
use crate::spell_info::{SpellEffect, SpellInfo};
use crate::tame::{
    MAX_ACTIVE_PETS, PetStableSlots, TameFailReason, TameHunter, TameTarget, can_tame,
};
use crate::world_data::{CreatureTemplate, WorldData};

/// `CLASS_HUNTER`.
const CLASS_HUNTER: u8 = 3;
/// Tame Beast, whose `spell_hun_tame_beast::CheckCast` is `can_tame`.
const TAME_BEAST: u32 = 1515;
pub const EFFECT_TAME_CREATURE: u32 = 55;
pub const EFFECT_SUMMON_PET: u32 = 56;
pub const EFFECT_DISMISS_PET: u32 = 102;
/// `SPELL_AURA_ALLOW_TAME_PET_TYPE`: `Player::CanTameExoticPets`.
const AURA_ALLOW_TAME_PET_TYPE: u32 = 146;
/// `DEFAULT_PLAYER_COMBAT_REACH`: `GetClosePoint(..., owner->GetCombatReach())`.
const OWNER_COMBAT_REACH: f32 = 1.5;

pub struct HunterPetPlugin;

impl Plugin for HunterPetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, load_pet_stables)
            .add_systems(FixedUpdate, remove_ownerless_pets);
    }
}

/// A hunter's `PetStable`, inserted with the first tamed pet or the stored stable at
/// login. `summoned` is the pet out in the world.
#[derive(Component, Debug, Clone)]
pub struct PetStable {
    pub character_id: u64,
    pub stored: StoredPetStable,
    pub summoned: Option<Entity>,
}

/// A summoned hunter pet: its owner and the active slot it is stored in.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HunterPet {
    pub owner: Entity,
    pub slot: u8,
}

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
            EFFECT_SUMMON_PET => check_summon_pet(world, caster, effect)?,
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
    effect: &SpellEffect,
) -> Result<(), CastFailure> {
    let Some(stable) = world.get::<PetStable>(caster) else {
        return Ok(());
    };
    if effect.misc_values[0] != 0 {
        return Ok(());
    }
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
    })
}

/// `Player::GetOrInitPetStable`, then its first free active slot.
fn first_free_slot(world: &mut World, hunter: Entity) -> Option<u8> {
    if world.get::<PetStable>(hunter).is_none() {
        let character_id = character_id(world, hunter)?;
        world.entity_mut(hunter).insert(PetStable {
            character_id,
            stored: StoredPetStable::default(),
            summoned: None,
        });
    }
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

/// `Spell::EffectSummonPet` for a player (MiscValue 0): a pet already out comes to the
/// owner (`NearTeleportTo` its close point); otherwise the pet in the effect's slot is
/// summoned there.
pub(crate) fn effect_summon_pet(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    let owner = ctx.caster;
    if world.get::<Player>(owner).is_none() || effect.misc_values[0] != 0 {
        return;
    }
    let Some((at, facing)) = close_point(world, owner) else {
        return;
    };
    if let Some(pet) = summoned_pet(world, owner) {
        if let Some(mut position) = world.get_mut::<Position>(pet) {
            *position = at;
        }
        crate::interest::mark_interest_position_changed(
            world
                .get_resource::<crate::interest::InterestDirtyQueue>()
                .map(|queue| &*queue),
            pet,
        );
        return;
    }
    let slot = effect_slot(effect);
    let stored = world
        .get::<PetStable>(owner)
        .and_then(|stable| stable.stored.active.get(slot).cloned().flatten());
    if stored.is_some() && slot < MAX_ACTIVE_PETS {
        put_pet_out(world, owner, slot as u8, at, facing);
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
    let Some(owner) = world.get::<HunterPet>(pet).map(|pet| pet.owner) else {
        return;
    };
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
/// the stored model, name and level, full health, full Focus (`Pet::CreateBaseAtTamed`
/// gives a Beast `POWER_FOCUS`; `power_type` MaxBasePower and DefaultPower), the owner's
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
    let stored = world
        .get::<PetStable>(owner)?
        .stored
        .active
        .get(usize::from(slot))?
        .clone()?;
    let (health, classification) = {
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
        )
    };
    let map = *world.get::<InMap>(owner)?;
    let zone = world.get::<Zone>(owner).copied().unwrap_or(Zone { id: 0 });
    let faction = *world.get::<UnitFactionTemplate>(owner)?;
    let replicate = world.get::<Replicate>(owner).is_some();
    let focus = world
        .get_resource::<PowerTypeTable>()
        .map_or_else(PowerPools::default, |table| {
            PowerPools::from_table(table, vec![PowerType::Focus])
        });
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
            Health {
                current: health,
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
            focus,
            CombatTarget::default(),
        ),
    ));
    if replicate {
        pet.insert((
            Replicate::to_clients(NetworkTarget::All),
            NetworkVisibility::default(),
        ));
    }
    Some((pet.id(), stored))
}

/// `Guardian::InitStatsForLevel` without `pet_levelstats` (not imported): ExpectedStat
/// CreatureHealth at the pet's level × the template's HealthModifier; an untuned
/// AzerothCore template takes `creature_classlevelstats` (`CreatureBaseStats::GenerateHealth`).
fn pet_max_health(data: &WorldData, template: &CreatureTemplate, level: u8) -> f32 {
    if let Some(tuning) = &template.tuning {
        return tuning.stats.max_health(level).max(1.0);
    }
    let base = data
        .class_level_base_hp
        .get(&(level, template.unit_class))
        .map_or(1, |hp| hp[usize::from(template.expansion)]);
    (base as f32 * template.health_modifier).ceil().max(1.0)
}

fn save_stable(world: &World, owner: Entity) {
    let (Some(db), Some(stable)) = (
        world.get_resource::<GameDatabase>(),
        world.get::<PetStable>(owner),
    ) else {
        return;
    };
    save_pet_stable(&db.db, stable.character_id, &stable.stored);
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
        character_id,
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

#[cfg(test)]
#[path = "hunter_pet_tests.rs"]
mod tests;
