//! Hunter pet control: the pet action bar sent to its owner, the orders its buttons give
//! (Attack, Follow, Stay, Move To) and its stances (Passive, Defensive, Assist), and the
//! pet AI that acts on them. TrinityCore a352b1fa: `WorldSession::HandlePetAction` /
//! `HandlePetActionHelper` (PetHandler.cpp:64-286), `CharmInfo::InitPetActionBar` /
//! `AddSpellToActionBar` (CharmInfo.cpp:49-62, 166-215), `Player::PetSpellInitialize`
//! (Player.cpp:22440-22477), `PetAI::UpdateAI`, `SelectNextTarget`, `CanAttack`, `DoAttack`,
//! `HandleReturnMovement`, `NeedToStop` (PetAI.cpp), `SpellMgr::LoadPetLevelupSpellMap`
//! (SpellMgr.cpp:2152-2199), `Pet::InitLevelupSpellsForLevel` (Pet.cpp:1494-1529), the
//! spell buttons of `HandlePetActionHelper` (PetHandler.cpp:287-391),
//! `HandlePetSpellAutocastOpcode` (PetHandler.cpp:628-676) and `PetAI::UpdateAI`'s autocast
//! (PetAI.cpp:105-212).
//!
//! Where this departs from TrinityCore:
//! - The bar is Retail's: Attack, Follow, Move To, four pet spells, Assist, Defensive,
//!   Passive. TrinityCore's `InitPetActionBar` puts Stay in the third slot and Aggressive
//!   in the eighth; Stay is still obeyed when ordered.
//! - Assist (`REACT_ASSIST`, which TrinityCore's handler ignores) attacks the owner's
//!   victim and what attacks the owner or the pet; Defensive only what attacks them (the
//!   Retail stance texts). TrinityCore's Defensive also takes the owner's victim, through
//!   `getAttackerForHelper`.
//! - Attackers are polled each tick (units auto-attacking the owner or the pet) instead
//!   of `OwnerAttackedBy` / `AttackedBy` on damage.
//! - A pet sent somewhere with Move To stays there, as after Stay: TrinityCore's
//!   `HandleReturnMovement` only holds a `COMMAND_STAY` pet and makes it follow again.
//! - The stance is saved to the stable when it changes, not at `SavePetToDB` time.
//! - After an autocast toggle the owner gets the whole bar again (`PetSpells`).
//! - Autocast casts the first ready autocast spell in bar order, not a random one, and
//!   never positive spells (Dash).
//! - Not modelled: `SMSG_PET_CAST_FAILED` for a failed ordered cast, autocast
//!   states kept in the stable (`character_pet.abilities`), spell ranks on the bar
//!   (`SupercedesSpell`).

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, ReplicationDirtyQueue};
use rusqlite::{Connection, OpenFlags};
use shared::components::{Health, Mounted, Player, Position, UnitFactionTemplate, UnitFlags};
use shared::protocol::{
    ACT_COMMAND, ACT_DISABLED, ACT_ENABLED, ACT_PASSIVE, ACT_REACTION, COMMAND_ABANDON,
    COMMAND_ATTACK, COMMAND_FOLLOW, COMMAND_MOVE_TO, COMMAND_STAY, PET_ACTION_BAR_SLOTS, PetAction,
    PetClearSpells, PetSpellAutocast, PetSpells, REACT_AGGRESSIVE, REACT_ASSIST, REACT_DEFENSIVE,
    REACT_PASSIVE, pet_action_button, pet_action_button_action, pet_action_button_type,
};

use crate::combat::AutoAttack;
use crate::creature_ai::{FollowOwner, PetMotion};
use crate::faction::{FactionTemplates, UnitFaction};
use crate::hunter_pet::{HunterPet, summoned_pet};
use crate::networking::{CombatTarget, PlayerLink};

/// `ACTION_BAR_INDEX_PET_SPELL_START` .. `_END` (CharmInfo.h:80-81).
const PET_SPELL_SLOTS: std::ops::Range<usize> = 3..7;
/// `SkillLineAbilityAcquireMethod::AutomaticCharLevel` (DBCEnums.h:2380).
const ACQUIRE_AUTOMATIC_CHAR_LEVEL: u32 = 2;
/// `SPELL_ATTR0_PASSIVE`.
const SPELL_ATTR0_PASSIVE: u32 = 0x0000_0040;
/// `SPELL_ATTR1_NO_AUTOCAST_AI`: `SpellInfo::IsAutocastable` is false.
const SPELL_ATTR1_NO_AUTOCAST_AI: u32 = 0x0002_0000;
/// `SPELL_ATTR9_AUTOCAST_OFF_BY_DEFAULT`.
const SPELL_ATTR9_AUTOCAST_OFF_BY_DEFAULT: u32 = 0x0002_0000;
/// `NeedToStop`: a pet stops chasing once its owner is this far away, the continent
/// visibility range (100, worldserver.conf.dist) less 10.
const MAX_CHASE_FROM_OWNER: f32 = 90.0;
/// `IsWithinMeleeRange` for a staying pet: `MELEE_RANGE` of the auto-attack.
const MELEE_RANGE: f32 = 5.0;
/// A pet ordered to follow is back (`MovementInform` FOLLOW) within its follow reach,
/// `2 × DEFAULT_PLAYER_COMBAT_REACH + PET_FOLLOW_DIST`, and `FOLLOW_RANGE_TOLERANCE`.
const FOLLOW_ARRIVED: f32 = 2.0 * 1.5 + 1.0 + 1.0;

pub struct PetControlPlugin;

impl Plugin for PetControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_pet_levelup_spells_startup)
            .add_systems(
                FixedUpdate,
                (
                    process_pet_actions,
                    process_pet_autocast_toggles,
                    pet_ai_system,
                )
                    .chain()
                    .after(crate::combat::auto_attack_system),
            );
    }
}

/// A pet's `CharmInfo` and react state.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct PetCharm {
    /// `CommandState`: `COMMAND_FOLLOW`, `COMMAND_STAY` or `COMMAND_MOVE_TO`.
    pub command: u32,
    /// `ReactState`.
    pub react: u32,
    /// `IsCommandAttack`: the owner ordered its current attack.
    pub command_attack: bool,
    /// `IsCommandFollow` while `IsReturning`: ordered to follow and not back at the
    /// owner yet, it attacks nothing (`PetAI::CanAttack`).
    pub follow_ordered: bool,
    /// `SaveStayPosition`: where Stay held it or Move To sent it.
    pub stay_at: Option<Position>,
    /// `PetActionBar`: packed action buttons.
    pub action_bar: [u32; PET_ACTION_BAR_SLOTS],
}

impl PetCharm {
    /// A pet out: following (`CharmInfo::CharmInfo`) in its stored stance
    /// (`Pet::LoadPetFromDB` → `SetReactState(petInfo->ReactState)`).
    pub fn new(action_bar: [u32; PET_ACTION_BAR_SLOTS], react: u32) -> Self {
        Self {
            command: COMMAND_FOLLOW,
            react,
            command_attack: false,
            follow_ordered: false,
            stay_at: None,
            action_bar,
        }
    }

    fn holds_position(&self) -> bool {
        self.command == COMMAND_STAY || self.command == COMMAND_MOVE_TO
    }

    /// The bar slot holding `spell` (`HasSpell`: the bar holds the pet's learned spells).
    pub(crate) fn spell_slot(&self, spell: u32) -> Option<usize> {
        PET_SPELL_SLOTS.clone().find(|&slot| {
            let button = self.action_bar[slot];
            button != 0
                && is_spell_button(pet_action_button_type(button))
                && pet_action_button_action(button) == spell
        })
    }

    /// `GetPetAutoSpellOnPos`: the spells on the bar with autocast on, in bar order.
    fn autocast_spells(&self) -> Vec<u32> {
        PET_SPELL_SLOTS
            .clone()
            .map(|slot| self.action_bar[slot])
            .filter(|&button| pet_action_button_type(button) == ACT_ENABLED)
            .map(pet_action_button_action)
            .collect()
    }
}

fn is_spell_button(kind: u8) -> bool {
    matches!(kind, ACT_ENABLED | ACT_DISABLED | ACT_PASSIVE)
}

/// One of a family's level-up spells: learned at `level`, on the bar as `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetLevelupSpell {
    pub level: u8,
    pub spell: u32,
    pub kind: u8,
}

/// `mPetLevelupSpellMap`: per `CreatureFamily`, its two skill lines' `AutomaticCharLevel`
/// abilities with a spell level, by level and then skill line and ability order (the
/// multimap's insertion order). Passive spells are left out: they never reach the bar.
#[derive(Resource, Debug, Default, Clone)]
pub struct PetLevelupSpells(pub HashMap<u32, Vec<PetLevelupSpell>>);

pub fn load_pet_levelup_spells(conn: &Connection) -> rusqlite::Result<PetLevelupSpells> {
    let mut stmt = conn.prepare(
        "SELECT f.ID, a.Spell, l.SpellLevel, COALESCE(m.Attributes_0, 0),
                COALESCE(m.Attributes_1, 0), COALESCE(m.Attributes_9, 0)
         FROM creature_family f
         JOIN skill_line_ability a ON a.SkillLine IN (f.SkillLine_0, f.SkillLine_1)
         JOIN spell_levels l ON l.SpellID = a.Spell AND l.DifficultyID = 0
         LEFT JOIN spell_misc m ON m.SpellID = a.Spell AND m.DifficultyID = 0
         WHERE a.SkillLine != 0 AND a.AcquireMethod = ?1 AND l.SpellLevel > 0
         ORDER BY f.ID, l.SpellLevel, a.SkillLine != f.SkillLine_0, a.ID",
    )?;
    let rows = stmt.query_map([ACQUIRE_AUTOMATIC_CHAR_LEVEL], |r| {
        let attrs = |i| r.get::<_, i64>(i).map(|value| value as u32);
        Ok((
            r.get::<_, i64>(0)? as u32,
            r.get::<_, i64>(1)? as u32,
            r.get::<_, i64>(2)?.clamp(0, i64::from(u8::MAX)) as u8,
            [attrs(3)?, attrs(4)?, attrs(5)?],
        ))
    })?;
    let mut families: HashMap<u32, Vec<PetLevelupSpell>> = HashMap::new();
    for row in rows {
        let (family, spell, level, [attr0, attr1, attr9]) = row?;
        if attr0 & SPELL_ATTR0_PASSIVE != 0 {
            continue;
        }
        families.entry(family).or_default().push(PetLevelupSpell {
            level,
            spell,
            kind: bar_state(attr1, attr9),
        });
    }
    Ok(PetLevelupSpells(families))
}

/// `AddSpellToActionBar`'s `ACT_DECIDE` for an active spell: not autocastable →
/// `ACT_PASSIVE`; autocast on by default → `ACT_ENABLED`; else `ACT_DISABLED`.
fn bar_state(attr1: u32, attr9: u32) -> u8 {
    if attr1 & SPELL_ATTR1_NO_AUTOCAST_AI != 0 {
        ACT_PASSIVE
    } else if attr9 & SPELL_ATTR9_AUTOCAST_OFF_BY_DEFAULT != 0 {
        ACT_DISABLED
    } else {
        ACT_ENABLED
    }
}

fn load_pet_levelup_spells_startup(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    let spells = if path.exists() {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("Failed to open world.db for pet spells");
        load_pet_levelup_spells(&conn).expect(
            "Failed to load pet level-up spells (creature_family: run scripts/db2.py import)",
        )
    } else {
        warn!("world.db not found, pets learn no spells");
        PetLevelupSpells::default()
    };
    commands.insert_resource(spells);
}

/// The bar of a new pet of `family` at `level`: `InitPetActionBar` with Retail's slots,
/// then `InitLevelupSpellsForLevel` learning the level-up spells from the highest level
/// down, each into the first free spell slot (`AddSpellToActionBar`).
pub fn pet_action_bar(spells: &[PetLevelupSpell], level: u8) -> [u32; PET_ACTION_BAR_SLOTS] {
    let mut bar = [0; PET_ACTION_BAR_SLOTS];
    bar[0] = pet_action_button(COMMAND_ATTACK, ACT_COMMAND);
    bar[1] = pet_action_button(COMMAND_FOLLOW, ACT_COMMAND);
    bar[2] = pet_action_button(COMMAND_MOVE_TO, ACT_COMMAND);
    bar[7] = pet_action_button(REACT_ASSIST, ACT_REACTION);
    bar[8] = pet_action_button(REACT_DEFENSIVE, ACT_REACTION);
    bar[9] = pet_action_button(REACT_PASSIVE, ACT_REACTION);
    let learned = spells.iter().rev().filter(|spell| spell.level <= level);
    for spell in learned {
        let Some(slot) = PET_SPELL_SLOTS.clone().find(|&slot| bar[slot] == 0) else {
            break;
        };
        bar[slot] = pet_action_button(spell.spell, spell.kind);
    }
    bar
}

/// A pet put out: its `CharmInfo` with the bar of its family at its level, and
/// `SMSG_PET_SPELLS_MESSAGE` to its owner (`PetSpellInitialize`).
pub(crate) fn init_pet_control(
    world: &mut World,
    owner: Entity,
    pet: Entity,
    family: u32,
    level: u8,
    react: u32,
) {
    let bar = world
        .get_resource::<PetLevelupSpells>()
        .and_then(|spells| spells.0.get(&family))
        .map_or_else(
            || pet_action_bar(&[], level),
            |spells| pet_action_bar(spells, level),
        );
    world.entity_mut(pet).insert(PetCharm::new(bar, react));
    send_pet_spells(world, owner, pet);
}

pub(crate) fn send_pet_spells(world: &mut World, owner: Entity, pet: Entity) {
    let Some(charm) = world.get::<PetCharm>(pet) else {
        return;
    };
    let message = PetSpells {
        pet: pet.to_bits(),
        command_state: charm.command,
        react_state: charm.react,
        action_buttons: charm.action_bar,
    };
    crate::spell_cast::send_to_owner(world, owner, message);
}

/// `SMSG_PET_CLEAR_SPELLS` (`Player::RemovePet`): the owner's pet left.
pub(crate) fn clear_pet_spells(world: &mut World, owner: Entity) {
    crate::spell_cast::send_to_owner(world, owner, PetClearSpells);
}

/// `CMSG_PET_ACTION` from each player.
fn process_pet_actions(world: &mut World) {
    let mut actions = Vec::new();
    let mut receivers = world.query::<(&mut MessageReceiver<PetAction>, &PlayerLink)>();
    for (mut receiver, link) in receivers.iter_mut(world) {
        actions.extend(receiver.receive().map(|action| (link.0, action)));
    }
    for (player, action) in actions {
        handle_pet_action(world, player, action);
    }
}

/// `HandlePetAction`: a mounted owner's orders and orders to anything but its living pet
/// are dropped; then `HandlePetActionHelper`, and the owner learns the pet's new command
/// and react state.
fn handle_pet_action(world: &mut World, owner: Entity, action: PetAction) {
    if world.get::<Mounted>(owner).is_some() {
        return;
    }
    let pet =
        crate::auras::charm::controlled_unit(world, owner).or_else(|| summoned_pet(world, owner));
    let Some(pet) = pet.filter(|pet| pet.to_bits() == action.pet) else {
        return;
    };
    if is_dead(world, pet) {
        return;
    }
    let order = pet_action_button_action(action.action);
    match pet_action_button_type(action.action) {
        ACT_COMMAND => command(world, owner, pet, order, &action),
        ACT_REACTION => react(world, owner, pet, order),
        kind if is_spell_button(kind) => {
            cast_ordered_spell(world, owner, pet, order, &action);
            return;
        }
        _ => return,
    }
    send_pet_spells(world, owner, pet);
}

/// `TARGET_UNIT_SRC_AREA_ENEMY`, `TARGET_UNIT_DEST_AREA_ENEMY`, `TARGET_DEST_DYNOBJ_ENEMY`:
/// spells the bar does not cast (`HandlePetActionHelper`).
const AREA_ENEMY_TARGETS: [u32; 3] = [15, 16, 28];

/// A spell button (`ACT_ENABLED`/`ACT_DISABLED`/`ACT_PASSIVE`, `HandlePetActionHelper`):
/// a learned, non-passive spell without an enemy area target is cast by the pet through
/// the spell pipeline at the given unit; as with an Attack order the pet goes after a unit
/// its owner is not friendly to (`_AttackStart`). A failed cast clears the attack order.
fn cast_ordered_spell(
    world: &mut World,
    owner: Entity,
    pet: Entity,
    spell_id: u32,
    action: &PetAction,
) {
    let learned = world
        .get::<PetCharm>(pet)
        .is_some_and(|charm| charm.spell_slot(spell_id).is_some());
    let Some(spell) = world
        .get_resource::<crate::spell_info::SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned()
    else {
        return;
    };
    let area_enemy = spell
        .effects
        .iter()
        .any(|effect| AREA_ENEMY_TARGETS.contains(&effect.implicit_targets[0]));
    if !learned || area_enemy || crate::auras::is_passive(&spell) {
        return;
    }
    update_charm(world, pet, |charm| {
        charm.command_attack = true;
        charm.follow_ordered = false;
    });
    let target = action.target.and_then(Entity::try_from_bits);
    match crate::spell_cast::creature_cast(world, pet, spell_id, target, false) {
        Ok(()) => {
            if let Some(target) = target
                && valid_attack_target(world, owner, target)
                && victim(world, pet) != Some(target)
            {
                attack_start(world, pet, target);
            }
        }
        Err(failure) => {
            debug!("Pet {pet} could not cast {spell_id}: {failure:?}");
            update_charm(world, pet, |charm| charm.command_attack = false);
        }
    }
}

/// `CMSG_PET_SPELL_AUTOCAST` from each player.
fn process_pet_autocast_toggles(world: &mut World) {
    let mut toggles = Vec::new();
    let mut receivers = world.query::<(&mut MessageReceiver<PetSpellAutocast>, &PlayerLink)>();
    for (mut receiver, link) in receivers.iter_mut(world) {
        toggles.extend(receiver.receive().map(|toggle| (link.0, toggle)));
    }
    for (player, toggle) in toggles {
        toggle_autocast(world, player, toggle);
    }
}

/// `HandlePetSpellAutocastOpcode` → `CharmInfo::SetSpellAutocast`: the owner's living pet
/// sets a learned autocastable spell's button `ACT_ENABLED` or `ACT_DISABLED`. The owner
/// then gets the bar again (TrinityCore leaves the client to update its own bar).
fn toggle_autocast(world: &mut World, owner: Entity, toggle: PetSpellAutocast) {
    let pet =
        crate::auras::charm::controlled_unit(world, owner).or_else(|| summoned_pet(world, owner));
    let Some(pet) = pet.filter(|pet| pet.to_bits() == toggle.pet) else {
        return;
    };
    if is_dead(world, pet) {
        return;
    }
    let Some(mut charm) = world.get_mut::<PetCharm>(pet) else {
        return;
    };
    let Some(slot) = charm.spell_slot(toggle.spell) else {
        return;
    };
    // `SpellInfo::IsAutocastable`: the bar holds a spell that is not as `ACT_PASSIVE`.
    if pet_action_button_type(charm.action_bar[slot]) == ACT_PASSIVE {
        return;
    }
    let kind = if toggle.enabled {
        ACT_ENABLED
    } else {
        ACT_DISABLED
    };
    charm.action_bar[slot] = pet_action_button(toggle.spell, kind);
    send_pet_spells(world, owner, pet);
}

/// `ACT_COMMAND`.
fn command(world: &mut World, owner: Entity, pet: Entity, order: u32, action: &PetAction) {
    match order {
        COMMAND_ABANDON => {
            if world.get::<crate::auras::charm::CharmState>(pet).is_some() {
                crate::auras::charm::release(world, pet);
            } else {
                crate::hunter_pet::abandon_or_dismiss_pet(world, pet);
            }
        }
        COMMAND_STAY => {
            set_motion(world, pet, PetMotion::Idle);
            let here = world.get::<Position>(pet).copied();
            update_charm(world, pet, |charm| {
                charm.command = COMMAND_STAY;
                charm.command_attack = false;
                charm.follow_ordered = false;
                charm.stay_at = here;
            });
        }
        COMMAND_FOLLOW => {
            stop_attacking(world, pet);
            crate::spell_cast::cancel_cast(world, pet);
            follow(world, pet, owner);
            update_charm(world, pet, |charm| {
                charm.command = COMMAND_FOLLOW;
                charm.command_attack = false;
                charm.follow_ordered = true;
                charm.stay_at = None;
            });
        }
        COMMAND_ATTACK => {
            let Some(target) = action
                .target
                .and_then(Entity::try_from_bits)
                .filter(|&target| world.get::<Health>(target).is_some())
            else {
                return;
            };
            if !valid_attack_target(world, owner, target) {
                return;
            }
            let attacking = victim(world, pet) == Some(target);
            let ordered = world
                .get::<PetCharm>(pet)
                .is_some_and(|charm| charm.command_attack);
            if attacking && ordered {
                return;
            }
            update_charm(world, pet, |charm| {
                charm.command_attack = true;
                charm.follow_ordered = false;
            });
            attack_start(world, pet, target);
        }
        COMMAND_MOVE_TO => {
            let Some([x, y, z]) = action.position else {
                return;
            };
            let point = Position { x, y, z };
            set_motion(world, pet, PetMotion::Point(point));
            update_charm(world, pet, |charm| {
                charm.command = COMMAND_MOVE_TO;
                charm.command_attack = false;
                charm.follow_ordered = false;
                charm.stay_at = Some(point);
            });
        }
        _ => {}
    }
}

/// `ACT_REACTION`: Passive stops the attack (`AttackStop`); every stance is set and kept
/// with the pet in its stable slot (`Pet::SavePetToDB` saves `ReactState`).
fn react(world: &mut World, owner: Entity, pet: Entity, react: u32) {
    if !matches!(
        react,
        REACT_PASSIVE | REACT_DEFENSIVE | REACT_AGGRESSIVE | REACT_ASSIST
    ) {
        return;
    }
    if react == REACT_PASSIVE {
        stop_attacking(world, pet);
    }
    update_charm(world, pet, |charm| charm.react = react);
    crate::hunter_pet::save_pet_react_state(world, owner, pet, react);
}

fn update_charm(world: &mut World, pet: Entity, change: impl FnOnce(&mut PetCharm)) {
    if let Some(mut charm) = world.get_mut::<PetCharm>(pet) {
        change(&mut charm);
    }
}

/// `PetAI::UpdateAI` for every pet.
pub(crate) fn pet_ai_system(world: &mut World) {
    let pets: Vec<_> = world
        .query_filtered::<(
            Entity,
            Option<&HunterPet>,
            Option<&crate::auras::charm::CharmState>,
        ), With<PetCharm>>()
        .iter(world)
        .filter_map(|(pet, hunter_pet, charm)| {
            if let Some(charm) = charm {
                Some((pet, charm.owner, charm.possessed))
            } else {
                hunter_pet.map(|hunter| (pet, hunter.owner, false))
            }
        })
        .collect();
    for (pet, owner, possessed) in pets {
        if possessed {
            if victim(world, pet).is_some_and(|target| !valid_attack_target(world, pet, target)) {
                stop_attacking(world, pet);
            }
        } else {
            update_pet(world, owner, pet);
        }
    }
}

/// With a living victim, stop when `NeedToStop`. Without one (none, or it died:
/// `KilledUnit`), attack the next target if any, else go back to the owner or the held
/// spot (`HandleReturnMovement`).
fn update_pet(world: &mut World, owner: Entity, pet: Entity) {
    if is_dead(world, pet) {
        return;
    }
    if world
        .get::<PetCharm>(pet)
        .is_some_and(|charm| charm.follow_ordered)
        && distance(world, pet, owner).is_some_and(|gap| gap <= FOLLOW_ARRIVED)
    {
        update_charm(world, pet, |charm| charm.follow_ordered = false);
    }
    if let Some(target) = victim(world, pet) {
        if !is_dead(world, target) {
            if need_to_stop(world, owner, pet, target) {
                stop_attacking(world, pet);
                update_charm(world, pet, |charm| charm.command_attack = false);
                return_movement(world, owner, pet);
                return;
            }
            autocast(world, pet, target);
            return;
        }
        stop_attacking(world, pet);
        update_charm(world, pet, |charm| charm.command_attack = false);
    }
    match select_next_target(world, owner, pet) {
        Some(next) => attack_start(world, pet, next),
        None => return_movement(world, owner, pet),
    }
}

/// `PetAI::UpdateAI`'s autocast for a pet not casting with a victim it may attack: the
/// first autocast spell on the bar that its victim takes (`CanAutoCast` → `CheckPetCast`:
/// cooldown, Focus, range, sight) is cast at it. TrinityCore picks at random among every
/// spell that could be cast; positive spells (cast at attackers or allies) are not
/// autocast here.
fn autocast(world: &mut World, pet: Entity, victim: Entity) {
    if world.get::<shared::casting::CastState>(pet).is_some() || !can_attack(world, pet, victim) {
        return;
    }
    let Some(spells) = world.get::<PetCharm>(pet).map(PetCharm::autocast_spells) else {
        return;
    };
    for spell_id in spells {
        let negative = world
            .get_resource::<crate::spell_info::SpellStore>()
            .and_then(|store| {
                let spell = store.get(spell_id)?;
                Some(crate::spell_info::positivity::negative_effects(store, spell) != 0)
            });
        if negative != Some(true) {
            continue;
        }
        let cast = crate::spell_cast::creature_cast(world, pet, spell_id, Some(victim), false);
        if cast.is_ok() {
            return;
        }
    }
}

/// `NeedToStop`: the owner is too far away, or the victim is no valid attack target.
fn need_to_stop(world: &World, owner: Entity, pet: Entity, target: Entity) -> bool {
    distance(world, pet, owner).is_none_or(|gap| gap >= MAX_CHASE_FROM_OWNER)
        || !valid_attack_target(world, pet, target)
}

/// `SelectNextTarget` for a pet that is not passive: what attacks the pet, then what
/// attacks the owner, then (Assist) the owner's victim, the first `CanAttack` allows.
fn select_next_target(world: &mut World, owner: Entity, pet: Entity) -> Option<Entity> {
    let react = world.get::<PetCharm>(pet)?.react;
    if react == REACT_PASSIVE {
        return None;
    }
    let assists_owner =
        react == REACT_ASSIST || world.get::<crate::auras::charm::CharmState>(pet).is_some();
    let owner_victim = assists_owner
        .then(|| {
            world
                .get::<AutoAttack>(owner)
                .map(|attack| attack.victim)
                .or_else(|| victim(world, owner))
        })
        .flatten();
    let candidates = [
        nearest_attacker(world, pet, pet),
        nearest_attacker(world, owner, pet),
    ];
    candidates
        .into_iter()
        .flatten()
        .chain(owner_victim)
        .find(|&target| can_attack(world, pet, target))
}

/// The unit auto-attacking `unit` nearest to `pet` (`getAttackerForHelper`'s
/// `m_attackers`).
fn nearest_attacker(world: &mut World, unit: Entity, pet: Entity) -> Option<Entity> {
    let at = *world.get::<Position>(pet)?;
    world
        .query::<(Entity, &AutoAttack, &Position)>()
        .iter(world)
        .filter(|(attacker, attack, _)| attack.victim == unit && *attacker != pet)
        .map(|(attacker, _, position)| (attacker, planar_distance(&at, position)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(attacker, _)| attacker)
}

/// `PetAI::CanAttack` without crowd control: alive; a passive pet only what it was
/// ordered to attack; nothing while ordered back to its owner; a staying pet what is in
/// reach unless ordered.
fn can_attack(world: &World, pet: Entity, target: Entity) -> bool {
    let Some(charm) = world.get::<PetCharm>(pet) else {
        return false;
    };
    if is_dead(world, target) || !valid_attack_target(world, pet, target) {
        return false;
    }
    if charm.react == REACT_PASSIVE {
        return charm.command_attack;
    }
    if charm.follow_ordered {
        return false;
    }
    if charm.command == COMMAND_STAY {
        return charm.command_attack
            || distance(world, pet, target).is_some_and(|gap| gap <= MELEE_RANGE);
    }
    true
}

/// `PetAI::_AttackStart` → `DoAttack`: attack `target` (`Unit::Attack` through the pet's
/// AI target) going after it (`UNIT_FLAG_PET_IN_COMBAT`), chasing it unless held by Stay
/// without an order.
/// Server-side orders from an authenticated NPC companion, using the same pet AI.
pub(crate) fn order_companion_pet(world: &mut World, owner: Entity, target: Option<Entity>) {
    let Some(pet) = summoned_pet(world, owner) else {
        return;
    };
    let at = world.get::<Position>(pet).copied();
    update_charm(world, pet, |charm| {
        charm.command = if target.is_some() {
            COMMAND_FOLLOW
        } else {
            COMMAND_STAY
        };
        charm.command_attack = target.is_some();
        charm.follow_ordered = false;
        if target.is_none() {
            charm.stay_at = at;
        }
    });
    if let Some(target) = target {
        attack_start(world, pet, target);
    } else {
        stop_attacking(world, pet);
        set_motion(world, pet, PetMotion::Idle);
    }
}

fn attack_start(world: &mut World, pet: Entity, target: Entity) {
    if !can_attack(world, pet, target) {
        return;
    }
    let possessed = world
        .get::<crate::auras::charm::CharmState>(pet)
        .is_some_and(|state| state.possessed);
    let chase = !possessed
        && world
            .get::<PetCharm>(pet)
            .is_some_and(|charm| charm.command != COMMAND_STAY || charm.command_attack);
    match world.get_mut::<CombatTarget>(pet) {
        Some(mut combat_target) => combat_target.0 = Some(target),
        None => {
            world.entity_mut(pet).insert(CombatTarget(Some(target)));
        }
    }
    set_pet_in_combat(world, pet, true);
    set_motion(
        world,
        pet,
        if chase {
            PetMotion::Chase(target)
        } else {
            PetMotion::Idle
        },
    );
}

/// `Unit::AttackStop` through the pet's AI target (`combat::creature_attack_system`
/// stops the swing).
fn stop_attacking(world: &mut World, pet: Entity) {
    if let Some(mut combat_target) = world.get_mut::<CombatTarget>(pet)
        && combat_target.0.is_some()
    {
        combat_target.0 = None;
    }
}

/// `HandleReturnMovement`: no longer going after a target; a pet holding a spot (Stay,
/// Move To) runs back to it, any other follows its owner.
fn return_movement(world: &mut World, owner: Entity, pet: Entity) {
    set_pet_in_combat(world, pet, false);
    let Some(charm) = world.get::<PetCharm>(pet) else {
        return;
    };
    if !charm.holds_position() {
        if world.get::<FollowOwner>(pet).is_none() {
            follow(world, pet, owner);
        }
        return;
    }
    let stay_at = charm.stay_at;
    if let (Some(PetMotion::Chase(_)), Some(spot)) = (world.get::<PetMotion>(pet), stay_at) {
        set_motion(world, pet, PetMotion::Point(spot));
    }
}

/// `MoveFollow(owner)`: `FollowOwner` replaces any other pet motion.
fn follow(world: &mut World, pet: Entity, owner: Entity) {
    world
        .entity_mut(pet)
        .remove::<PetMotion>()
        .insert(FollowOwner::new(owner));
}

/// Any other motion replaces following.
fn set_motion(world: &mut World, pet: Entity, motion: PetMotion) {
    world.entity_mut(pet).remove::<FollowOwner>().insert(motion);
}

/// `UNIT_FLAG_PET_IN_COMBAT`, replicated.
fn set_pet_in_combat(world: &mut World, pet: Entity, on: bool) {
    let Some(mut flags) = world.get_mut::<UnitFlags>(pet) else {
        return;
    };
    let value = if on {
        flags.0 | UnitFlags::PET_IN_COMBAT
    } else {
        flags.0 & !UnitFlags::PET_IN_COMBAT
    };
    if flags.0 == value {
        return;
    }
    flags.0 = value;
    if let Some(queue) = world.get_resource::<ReplicationDirtyQueue>() {
        queue.mark(pet);
    }
}

/// The unit the pet's AI attacks.
fn victim(world: &World, pet: Entity) -> Option<Entity> {
    world.get::<CombatTarget>(pet).and_then(|target| target.0)
}

fn is_dead(world: &World, unit: Entity) -> bool {
    world
        .get::<Health>(unit)
        .is_none_or(|health| health.current <= 0.0)
}

/// `WorldObject::IsValidAttackTarget` as far as modelled: alive, attackable, a hostile
/// faction.
fn valid_attack_target(world: &World, attacker: Entity, target: Entity) -> bool {
    if attacker == target || is_dead(world, target) {
        return false;
    }
    if world
        .get::<UnitFlags>(target)
        .is_some_and(|flags| !flags.is_selectable() || !flags.is_attackable())
    {
        return false;
    }
    let Some(templates) = world.get_resource::<FactionTemplates>() else {
        return false;
    };
    let faction = |unit: Entity| UnitFaction {
        template: world
            .get::<UnitFactionTemplate>(unit)
            .map_or(0, |template| template.0),
        is_player: crate::faction::player_affected(
            world.get::<Player>(unit).is_some(),
            world.get::<UnitFlags>(unit),
        ),
    };
    templates.can_attack(faction(attacker), faction(target))
}

fn distance(world: &World, a: Entity, b: Entity) -> Option<f32> {
    Some(planar_distance(
        world.get::<Position>(a)?,
        world.get::<Position>(b)?,
    ))
}

fn planar_distance(a: &Position, b: &Position) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}

#[cfg(test)]
#[path = "pet_control_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "warlock_pet_tests.rs"]
mod warlock_tests;
