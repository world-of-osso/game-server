//! Creature aggro (docs/specs/creature-aggro.md): hostile creatures attack
//! players that come within their aggro radius, any creature with a threat
//! list fights back when damaged, the top-threat unit is chased and
//! auto-attacked, and a creature with no valid target left evades home.
//!
//! AzerothCore references: `CreatureAI::MoveInLineOfSight` (CreatureAI.cpp:166),
//! `Creature::CanStartAttack` (Creature.cpp:1869), `Creature::GetAggroRange`
//! (Creature.cpp:3224), `Unit::DealDamage` threat (Unit.cpp:1197-1203),
//! `CreatureAI::UpdateVictim` / `Creature::SelectVictim` (CreatureAI.cpp:277,
//! Unit.cpp:11613), `CreatureAI::EnterEvadeMode` (CreatureAI.cpp:205).

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{
    Health, Player, Position, ThreatTable, UnitFactionTemplate, UnitFlags, UnitLevel,
    UnitThreatList,
};
use shared::level_scaling::{LevelScaling, level_for_viewer};

use crate::combat::DeathState;
use crate::creature_ai::{CreatureState, SpawnPoint, chase_system};
use crate::death::PlayerDeathState;
use crate::faction::{FactionTemplates, Reaction, UnitFaction};
use crate::grid_activity::GridDormant;
use crate::maps::InMap;
use crate::networking::CombatTarget;
use crate::taxi::TaxiFlight;

/// `MAX_AGGRO_RADIUS` (Unit.h:45).
pub(crate) const MAX_AGGRO_RADIUS: f32 = 45.0;
#[path = "creature_aggro_distance.rs"]
mod attack_distance;
pub(crate) use attack_distance::AggroDistance;
/// `ATTACK_DISTANCE` (ObjectDefines.h:25): melee range for the threat
/// swap threshold.
const ATTACK_DISTANCE: f32 = 5.0;
/// `Creature::CanCreatureAttack` (Creature.cpp:2741-2755): a victim farther than this
/// from the creature's home is out of reach: the continent visibility range
/// (`Visibility.Distance.Continents` = 100, worldserver.conf.dist:2365; under the
/// 2 × `SIZE_OF_GRID_CELL` = 133 yd cap) plus both default combat reaches (1.5).
const HOME_REACH: f32 = 100.0 + 2.0 * 1.5;
/// `MAX_AGGRO_RESET_TIME` (Unit.h:44): after direct damage, a creature ignores
/// the home distance of its victims this long.
const MAX_AGGRO_RESET_SECS: f64 = 10.0;

const CREATURE_TYPE_CRITTER: u32 = 8;
const CREATURE_TYPE_NON_COMBAT_PET: u32 = 12;
const CREATURE_FLAG_EXTRA_CIVILIAN: u32 = 0x2;
const CREATURE_FLAG_EXTRA_TRIGGER: u32 = 0x80;
const UNIT_FLAG_IMMUNE_TO_PC: u32 = 0x100;

pub struct CreatureAggroPlugin;

impl Plugin for CreatureAggroPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::creature_sight::CreatureSightPlugin);
        app.add_observer(replicate_woken_threat_list);
        app.add_systems(
            FixedUpdate,
            (
                aggro_on_sight_system,
                update_victim_system,
                replicate_threat_list_system,
            )
                .chain()
                .before(chase_system),
        );
    }
}

/// How a creature template takes part in combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggroMode {
    /// No threat list: never enters combat or fights back. Triggers
    /// (`Unit::CombatStart` skips them), critters and non-combat pets
    /// (CritterAI flees, `CanStartAttack` rejects them), `IMMUNE_TO_PC`
    /// units and `npc_training_dummy` (stunned, never attacks).
    Passive,
    /// Fights back when attacked, never aggroes on sight: civilians
    /// (`CanStartAttack` rejects `IsCivilian`).
    Reactive,
    /// Also attacks hostile players inside its aggro radius.
    Aggressive,
}

impl AggroMode {
    pub fn for_template(
        creature_type: u32,
        flags_extra: u32,
        unit_flags: u32,
        script_name: &str,
    ) -> Self {
        let passive = flags_extra & CREATURE_FLAG_EXTRA_TRIGGER != 0
            || creature_type == CREATURE_TYPE_CRITTER
            || creature_type == CREATURE_TYPE_NON_COMBAT_PET
            || unit_flags & UNIT_FLAG_IMMUNE_TO_PC != 0
            || script_name == crate::creature_scripts::TRAINING_DUMMY_SCRIPT;
        if passive {
            Self::Passive
        } else if flags_extra & CREATURE_FLAG_EXTRA_CIVILIAN != 0 {
            Self::Reactive
        } else {
            Self::Aggressive
        }
    }
}

/// An aggressive creature participates in proximity acquisition. Distance comes
/// from TC GetAttackDistance, not the legacy template detection_range column.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct AggroRange;

/// Spawn-time combat components: a threat list for every non-passive
/// creature, an aggro radius for aggressive ones.
pub(crate) fn insert_aggro_components(entity: &mut EntityCommands, mode: AggroMode) {
    match mode {
        AggroMode::Passive => {}
        AggroMode::Reactive => {
            entity.insert((ThreatTable::default(), CreatureState::Idle));
        }
        AggroMode::Aggressive => {
            entity.insert((ThreatTable::default(), CreatureState::Idle, AggroRange));
        }
    }
}

type Victims<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static Health,
        Option<&'static UnitFactionTemplate>,
        Option<&'static UnitLevel>,
        Option<&'static PlayerDeathState>,
        Has<Player>,
        Has<TaxiFlight>,
        Has<DeathState>,
        Option<&'static InMap>,
        Option<&'static UnitFlags>,
    ),
>;

/// A unit a creature may keep attacking: in the creature's map copy (`IsInMap`),
/// alive (not a corpse or ghost), not on a flight path, and attackable by the
/// creature's faction (`Creature::_IsTargetAcceptable` / `CanCreatureAttack`).
fn valid_victim(
    victims: &Victims,
    templates: &FactionTemplates,
    creature_faction: u32,
    creature_map: &InMap,
    victim: Entity,
) -> bool {
    let Ok((_, health, faction, _, death, is_player, flying, npc_dead, map, flags)) =
        victims.get(victim)
    else {
        return false;
    };
    let alive = health.current > 0.0
        && !npc_dead
        && death.is_none_or(|death| death.state == shared::death::DeathState::Alive);
    map == Some(creature_map)
        && alive
        && !flying
        && templates.can_attack(
            UnitFaction {
                template: creature_faction,
                is_player: false,
            },
            UnitFaction {
                template: faction.map_or(0, |faction| faction.0),
                is_player: crate::faction::player_affected(is_player, flags),
            },
        )
}

type Watchers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Position,
        &'static UnitLevel,
        Option<&'static LevelScaling>,
        &'static UnitFactionTemplate,
        &'static CreatureState,
        &'static InMap,
        Option<&'static crate::scripted_ai::ReactState>,
    ),
    (With<AggroRange>, Without<DeathState>, Without<GridDormant>),
>;

/// `CreatureAI::MoveInLineOfSight` → `Creature::CanStartAttack`: an idle
/// aggressive creature (`REACT_AGGRESSIVE`) attacks the nearest valid player it is hostile to
/// inside its level-scaled aggro radius and in its line of sight
/// (`Creature::CanStartAttack` → `IsWithinLOSInMap`; `crate::line_of_sight`). There is
/// no Z-difference test.
fn aggro_on_sight_system(
    watchers: Watchers,
    players: Query<(Entity, &Position, &InMap), With<Player>>,
    victims: Victims,
    templates: Res<FactionTemplates>,
    attack_distance: AggroDistance,
    bodies: Query<crate::line_of_sight::BodyParts>,
    los: Option<Res<crate::line_of_sight::LineOfSight>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let players: Vec<(Entity, Position, InMap)> = players
        .iter()
        .map(|(player, position, map)| (player, *position, *map))
        .collect();
    if players.is_empty() {
        return;
    }
    for (creature, position, level, scaling, faction, state, map, react) in &watchers {
        let aggressive =
            react.is_none_or(|react| *react == crate::scripted_ai::ReactState::Aggressive);
        if !state.is_idle() || !aggressive {
            continue;
        }
        // Model reaches are added after TC's45yd clamp; do not truncate the
        // center-distance candidates at the unclothed attack-distance limit.
        let in_reach = players
            .iter()
            .filter_map(|(player, player_pos, player_map)| {
                let distance = distance(position, player_pos);
                (player_map == map).then_some((*player, distance))
            });
        let mut candidates: Vec<(Entity, f32)> = in_reach
            .filter(|&(player, distance)| {
                let Ok((_, _, player_faction, player_level, ..)) = victims.get(player) else {
                    return false;
                };
                let player_level = player_level.map_or(1, |l| l.0);
                // Creature::GetAttackDistance: both levels against each other.
                let creature_level = level_for_viewer(*level, scaling, player_level);
                let Some(radius) =
                    attack_distance.center_radius(creature, player, creature_level, player_level)
                else {
                    return false;
                };
                let hostile = templates.reaction(faction.0, player_faction.map_or(0, |f| f.0))
                    == Reaction::Hostile;
                distance < radius
                    && hostile
                    && valid_victim(&victims, &templates, faction.0, map, player)
            })
            .collect();
        // The nearest one it sees: rays only until one is seen.
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
        let seen = candidates.into_iter().find(|&(player, _)| {
            match (bodies.get(creature), bodies.get(player)) {
                (Ok(me), Ok(them)) => {
                    crate::line_of_sight::unit_sees(los.as_deref(), *map, me, them)
                }
                _ => false,
            }
        });
        if let Some((player, center_distance)) = seen {
            debug!(
                creature = creature.to_bits(),
                target = player.to_bits(),
                map = ?map,
                creature_position = ?position,
                target_position = ?players.iter().find(|(unit, ..)| *unit == player).map(|(_, position, _)| position),
                center_distance,
                simulation_secs = time.elapsed_secs_f64(),
                "player proximity acquisition"
            );
            commands.queue(move |world: &mut World| attack_start(world, creature, player));
        }
    }
}

type Engaged<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Position,
        &'static UnitFactionTemplate,
        &'static mut ThreatTable,
        &'static mut CreatureState,
        Option<&'static mut CombatTarget>,
        &'static InMap,
        Option<&'static crate::scripted_ai::ReactState>,
        Option<&'static crate::auras::ActiveAuras>,
        (
            Option<&'static SpawnPoint>,
            Option<&'static LastDamagedTime>,
        ),
        Has<crate::scripted_ai::ScriptedVictimUpdatePaused>,
        Option<&'static Fixate>,
    ),
    (
        Without<DeathState>,
        Without<GridDormant>,
        Without<crate::followers::Follower>,
    ),
>;

/// `Creature::SetLastDamagedTime`: until when direct damage exempts the creature's
/// victims from the home distance check.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub(crate) struct LastDamagedTime(f64);

/// `Unit::DealDamage` (Unit.cpp:1085-1089): direct damage taken by a creature, not
/// periodic damage ("DoT's and Thorns / Retribution Aura do not contribute to this").
pub(crate) fn note_damage_taken(world: &mut World, victim: Entity) {
    if world.get::<Player>(victim).is_some() {
        return;
    }
    let until = world.resource::<Time>().elapsed_secs_f64() + MAX_AGGRO_RESET_SECS;
    if let Ok(mut entity) = world.get_entity_mut(victim) {
        entity.insert(LastDamagedTime(until));
    }
}

/// `Creature::CanCreatureAttack` (Creature.cpp:2731-2755): outside a dungeon, and
/// unless it took direct damage in the last `MAX_AGGRO_RESET_SECS`, a creature only
/// keeps attacking a victim within `HOME_REACH` of its home. A rooted or stationary
/// creature thus drops a player who walks away and, with no victim left, evades.
/// Not modelled: world bosses (always checked), `CanNotReachTarget`
/// (no path data).
pub(crate) fn within_reach_of_home(
    home: Option<&SpawnPoint>,
    last_damaged: Option<&LastDamagedTime>,
    now: f64,
    dungeon: bool,
    victim: &Position,
) -> bool {
    let Some(home) = home else {
        return true;
    };
    if dungeon || last_damaged.is_some_and(|damaged| damaged.0 > now) {
        return true;
    }
    let (dx, dy, dz) = (victim.x - home.x, victim.y - home.y, victim.z - home.z);
    dx * dx + dy * dy + dz * dz <= HOME_REACH * HOME_REACH
}

/// `ThreatManager::_fixateRef`: the unit `ReselectVictim` always prefers while it is
/// on the threat list (`HandleAuraModFixate`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fixate(pub Entity);

/// `CreatureAI::UpdateVictim` / `Creature::SelectVictim`: drop threat
/// entries that are no longer valid targets, attack the top-threat unit
/// (a challenger needs 110% of the current target's threat in melee range,
/// 130% beyond it: `ThreatMgr::getHostileTarget`), and evade when no valid
/// target is left. A passive creature keeps its victim without attacking it
/// (`CreatureAI::UpdateVictim`, `scripted_ai::Ctx::update_victim`).
pub(crate) fn update_victim_system(
    time: Res<Time>,
    mut engaged: Engaged,
    victims: Victims,
    templates: Res<FactionTemplates>,
    maps: Option<Res<crate::maps::MapStore>>,
    mut commands: Commands,
) {
    let now = time.elapsed_secs_f64();
    for (
        creature,
        position,
        faction,
        mut threat,
        mut state,
        combat_target,
        map,
        react,
        auras,
        (home, last_damaged),
        scripted_update_paused,
        fixate,
    ) in &mut engaged
    {
        let dungeon = maps
            .as_deref()
            .and_then(|maps| maps.map(map.map_id))
            .is_some_and(|entry| entry.is_dungeon());
        let CreatureState::Pursuing { target } = *state else {
            continue;
        };
        if scripted_update_paused || react == Some(&crate::scripted_ai::ReactState::Passive) {
            continue;
        }
        let valid = |unit: u64| {
            Entity::try_from_bits(unit).is_some_and(|unit| {
                valid_victim(&victims, &templates, faction.0, map, unit)
                    && victims.get(unit).is_ok_and(|(victim, ..)| {
                        within_reach_of_home(home, last_damaged, now, dungeon, victim)
                    })
            })
        };
        let current = valid(target).then_some(target);
        if current.is_none() {
            threat.remove(target);
        }
        while let Some(top) = threat.top_threat().map(|entry| entry.entity) {
            if valid(top) {
                break;
            }
            threat.remove(top);
        }
        let Some(top) = threat.top_threat().map(|entry| entry.entity) else {
            commands.queue(move |world: &mut World| enter_evade_mode(world, creature));
            continue;
        };
        let top_in_melee = Entity::try_from_bits(top)
            .and_then(|top| victims.get(top).ok())
            .is_some_and(|(top_pos, ..)| distance(position, top_pos) <= ATTACK_DISTANCE);
        let taunted = auras.and_then(|auras| {
            auras
                .iter()
                .filter(|aura| {
                    aura.effects
                        .iter()
                        .any(|effect| effect.aura_type == crate::auras::handlers::MOD_TAUNT)
                        && valid(aura.caster.to_bits())
                })
                .last()
                .map(|aura| aura.caster.to_bits())
        });
        // ReselectVictim: a listed fixate target is always preferred; removing its
        // threat reference clears the fixate.
        let fixated = fixate
            .map(|fixate| fixate.0.to_bits())
            .filter(|&fixated| threat.entries().iter().any(|entry| entry.entity == fixated));
        if fixate.is_some() && fixated.is_none() {
            commands.entity(creature).remove::<Fixate>();
        }
        let Some(victim) = fixated
            .or(taunted)
            .or_else(|| threat.aggro_target(current, top_in_melee))
        else {
            continue;
        };
        let attacking = combat_target
            .as_ref()
            .is_some_and(|combat_target| combat_target.0.map(Entity::to_bits) == Some(victim));
        if victim == target && attacking {
            continue;
        }
        if victim != target {
            *state = CreatureState::Pursuing { target: victim };
        }
        let victim = Entity::from_bits(victim);
        match combat_target {
            Some(mut combat_target) => combat_target.0 = Some(victim),
            None => {
                commands.entity(creature).insert(CombatTarget(Some(victim)));
            }
        }
    }
}

/// `ThreatManager::SendThreatListToClients` membership: a creature's replicated
/// `UnitThreatList` holds the units on its `ThreatTable`, re-sent only when they change
/// (threat amounts are not sent). Clients show a creature with them on it as fighting
/// them (nameplate visibility, hostile colour).
fn replicate_threat_list_system(
    creatures: Query<
        (Entity, &ThreatTable, Option<&UnitThreatList>),
        (Changed<ThreatTable>, Without<GridDormant>),
    >,
    dirty: Option<Res<ReplicationDirtyQueue>>,
    mut commands: Commands,
) {
    for (creature, threat, replicated) in &creatures {
        publish_threat_membership(
            creature,
            threat,
            replicated,
            dirty.as_deref(),
            &mut commands,
        );
    }
}

fn replicate_woken_threat_list(
    event: On<Remove, GridDormant>,
    creatures: Query<(&ThreatTable, Option<&UnitThreatList>)>,
    dirty: Option<Res<ReplicationDirtyQueue>>,
    mut commands: Commands,
) {
    if let Ok((threat, replicated)) = creatures.get(event.entity) {
        publish_threat_membership(
            event.entity,
            threat,
            replicated,
            dirty.as_deref(),
            &mut commands,
        );
    }
}

fn publish_threat_membership(
    creature: Entity,
    threat: &ThreatTable,
    replicated: Option<&UnitThreatList>,
    dirty: Option<&ReplicationDirtyQueue>,
    commands: &mut Commands,
) {
    let units: Vec<u64> = threat.entries().iter().map(|entry| entry.entity).collect();
    if replicated.is_some_and(|replicated| replicated.0 == units)
        || (replicated.is_none() && units.is_empty())
    {
        return;
    }
    commands.entity(creature).insert(UnitThreatList(units));
    if let Some(dirty) = dirty {
        dirty.mark(creature);
    }
}

fn distance(a: &Position, b: &Position) -> f32 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z).length()
}

/// TC a352b1fa Spell.cpp:2893 and ThreatManager.cpp:733-760:
/// effective healing generates half threat, split across the healed unit's enemies.
/// No threat from overhealing or healing a unit that has no engaged creature.
/// Controlled creatures retain membership but gain no assist threat.
pub(crate) fn add_healing_threat(
    world: &mut World,
    healer: Entity,
    healed: Entity,
    effective: f32,
    school: u32,
) {
    if effective <= 0.0 {
        return;
    }
    let enemies: Vec<Entity> = world
        .query::<(Entity, &ThreatTable)>()
        .iter(world)
        .filter(|(_, table)| {
            table
                .entries()
                .iter()
                .any(|entry| entry.entity == healed.to_bits())
        })
        .map(|(creature, _)| creature)
        .collect();
    let (controlled, available): (Vec<_>, Vec<_>) = enemies
        .into_iter()
        .filter(|creature| can_engage(world, *creature))
        .partition(|creature| {
            world
                .get::<crate::auras::ActiveAuras>(*creature)
                .is_some_and(|auras| {
                    auras
                        .iter()
                        .flat_map(|aura| &aura.effects)
                        .any(|effect| matches!(effect.aura_type, 5 | 7 | 12))
                })
        });
    let amount = effective * 0.5 / available.len().max(1) as f32;
    for creature in available {
        add_school_threat(world, creature, healer, amount, school);
    }
    for creature in controlled {
        add_school_threat(world, creature, healer, 0.0, school);
    }
}

fn can_engage(world: &World, creature: Entity) -> bool {
    world.get::<ThreatTable>(creature).is_some()
        && world.get::<DeathState>(creature).is_none()
        && world
            .get::<Health>(creature)
            .is_some_and(|health| health.current > 0.0)
        && world.get::<CreatureState>(creature) != Some(&CreatureState::Returning)
}

/// `CreatureAI::AttackStart` → `Unit::Attack`: an idle creature puts
/// `target` on its threat list with no threat, starts chasing and
/// auto-attacking it, and both enter combat. Evading and dead creatures,
/// and creatures without a threat list, ignore it.
pub(crate) fn attack_start(world: &mut World, creature: Entity, target: Entity) {
    add_threat(world, creature, target, 0.0);
}

/// `Unit::AddThreat` from damage dealt by `attacker` (`Unit::DealDamage`,
/// 1 threat per point) or a harmful spell hit (0 threat, `Spell::DoSpellHitOnUnit`
/// → `CombatStart`). An idle creature starts attacking `attacker`
/// (`Unit::CombatStart` → `AttackStart`); an engaged one picks its target
/// from threat next tick.
pub(crate) fn add_threat(world: &mut World, creature: Entity, attacker: Entity, amount: f32) {
    add_school_threat(world, creature, attacker, amount, 1);
}

/// TC a352b1fa ThreatManager.cpp:677-730, SPELL_AURA_MOD_THREAT (10),
/// including school-specific Righteous Fury when its aura is active.
fn threat_multiplier(world: &World, source: Entity, school: u32) -> f32 {
    world
        .get::<crate::auras::ActiveAuras>(source)
        .map_or(1.0, |auras| {
            auras
                .iter()
                .flat_map(|aura| aura.effects.iter().map(move |effect| (aura.stacks, effect)))
                .filter(|(_, effect)| {
                    effect.aura_type == crate::auras::handlers::MOD_THREAT
                        && effect.misc_values[0] as u32 & school != 0
                })
                .fold(1.0, |mult, (stacks, effect)| {
                    mult * (1.0 + effect.amount * f32::from(stacks.max(1)) / 100.0)
                })
        })
}

pub(crate) fn add_school_threat(
    world: &mut World,
    creature: Entity,
    attacker: Entity,
    amount: f32,
    school: u32,
) {
    let taunt_multiplier = world
        .get::<crate::auras::ActiveAuras>(creature)
        .map_or(1.0, |auras| {
            auras
                .iter()
                .filter(|aura| aura.caster == attacker)
                .flat_map(|aura| &aura.effects)
                .filter(|effect| effect.aura_type == crate::auras::handlers::MOD_TAUNT)
                .fold(1.0, |multiplier, effect| {
                    multiplier * (1.0 + effect.amount / 100.0)
                })
        });
    let amount = amount * threat_multiplier(world, attacker, school) * taunt_multiplier;
    if creature == attacker || !can_engage(world, creature) {
        return;
    }
    if let Some(mut threat) = world.get_mut::<ThreatTable>(creature) {
        threat.add_threat(attacker.to_bits(), amount);
    }
    let engages = world.get::<CreatureState>(creature) == Some(&CreatureState::Idle);
    if engages {
        let mut entity = world.entity_mut(creature);
        entity.insert((
            CreatureState::Pursuing {
                target: attacker.to_bits(),
            },
            CombatTarget(Some(attacker)),
        ));
        stand_up_for_combat(&mut entity);
    }
    crate::combat_refs::set_in_combat_with(world, creature, attacker);
    if engages {
        // Creature::AtEngage → CreatureAI::JustEngagedWith.
        crate::scripted_ai::dispatch(
            world,
            creature,
            crate::scripted_ai::Hook::JustEngagedWith(attacker),
        );
    }
}

///114 ATTACK_ME: match the highest threat without adding arbitrary threat,
///then attack the caster. Aura11 holds selection independently until removal.
///Units without a creature threat list (including players) cannot be forced.
pub(crate) fn taunt(world: &mut World, creature: Entity, caster: Entity) {
    if !can_engage(world, creature) {
        return;
    }
    let table = world
        .get::<ThreatTable>(creature)
        .expect("engageable threat list");
    let top = table.top_threat().map_or(0.0, |entry| entry.threat);
    let delta = (top - table.threat_for(caster.to_bits())).max(0.0);
    world
        .get_mut::<ThreatTable>(creature)
        .unwrap()
        .add_threat(caster.to_bits(), delta);
    attack_start(world, creature, caster);
    world.entity_mut(creature).insert((
        CreatureState::Pursuing {
            target: caster.to_bits(),
        },
        CombatTarget(Some(caster)),
    ));
}

/// `Unit::Attack`: "Remove emote and stand state - will be restored on creature reset"
/// (`SetEmoteState(EMOTE_ONESHOT_NONE)`, `SetStandState(UNIT_STAND_STATE_STAND)`).
fn stand_up_for_combat(entity: &mut EntityWorldMut) {
    if let Some(mut pose) = entity.get_mut::<shared::components::UnitPose>() {
        let standing = shared::components::UnitPose {
            stand_state: shared::components::StandState::Stand,
            emote_state: 0,
            ..*pose
        };
        if pose.set_if_neq(standing) {
            let creature = entity.id();
            if let Some(queue) = entity.world().get_resource::<ReplicationDirtyQueue>() {
                queue.mark(creature);
            }
        }
    }
}

/// `CreatureAI::EnterEvadeMode`: `_EnterEvadeMode` (auras removed —
/// `RemoveAurasOnEvade` —, cast interrupted, threat cleared, combat stopped: the
/// creature and every unit auto-attacking it stop swinging, spell cooldowns
/// reset), run home (`MoveTargetedHome`) with its health untouched, then the script's
/// `EnterEvadeMode` / `Reset`. An evading creature does not evade again
/// (`IsInEvadeMode`).
pub(crate) fn enter_evade_mode(world: &mut World, creature: Entity) {
    if world.get::<DeathState>(creature).is_some()
        || world.get::<CreatureState>(creature) == Some(&CreatureState::Returning)
    {
        return;
    }
    world
        .entity_mut(creature)
        .remove::<crate::creature_ai::CannotReachTarget>();
    remove_auras_on_evade(world, creature);
    crate::spell_cast::cancel_cast(world, creature);
    // `CreatureAI::_EnterEvadeMode`: `SetTappedBy(nullptr)`, `ResetPlayerDamageReq`,
    // `SetLastDamagedTime(0)` (CreatureAI.cpp:313-317).
    crate::creature_tap::clear(world, creature);
    world.entity_mut(creature).remove::<LastDamagedTime>();
    world
        .entity_mut(creature)
        .remove::<crate::spell_cast::SpellHistory>();
    let mut entity = world.entity_mut(creature);
    entity.insert(CreatureState::Returning);
    if let Some(mut threat) = entity.get_mut::<ThreatTable>() {
        threat.reset();
    }
    if let Some(mut target) = entity.get_mut::<CombatTarget>() {
        target.0 = None;
    }
    crate::combat::combat_stop(world, creature);
    crate::scripted_ai::dispatch(world, creature, crate::scripted_ai::Hook::EnterEvadeMode);
}

/// `SPELL_ATTR1_AURA_STAYS_AFTER_COMBAT`.
const SPELL_ATTR1_AURA_STAYS_AFTER_COMBAT: u32 = 0x0200_0000;

/// `Unit::RemoveAurasOnEvade` (TC a352b1fa Unit.cpp:4459-4487): player-owned
/// units keep their auras; ordinary creatures retain only stays-after-combat
/// auras (vehicle/clone auras unmodelled) and restore addon auras at home.
fn remove_auras_on_evade(world: &mut World, creature: Entity) {
    if crate::faction::player_affected(
        world.get::<Player>(creature).is_some(),
        world.get::<UnitFlags>(creature),
    ) {
        return;
    }
    let store = world.get_resource::<crate::spell_info::SpellStore>();
    let stays: Vec<u32> = world
        .get::<crate::auras::ActiveAuras>(creature)
        .map(|auras| {
            auras
                .iter()
                .filter(|aura| {
                    store
                        .and_then(|store| store.get(aura.spell_id))
                        .and_then(|spell| spell.misc.as_ref())
                        .is_some_and(|misc| {
                            misc.attributes[1] & SPELL_ATTR1_AURA_STAYS_AFTER_COMBAT != 0
                        })
                })
                .map(|aura| aura.instance_id)
                .collect()
        })
        .unwrap_or_default();
    crate::auras::remove_auras(world, creature, crate::auras::RemoveReason::Evade, |aura| {
        !stays.contains(&aura.instance_id)
    });
}

/// A dying creature forgets its fight (`Creature::setDeathState` →
/// `ClearAllThreat`, `CombatStop`), so it respawns idle.
pub(crate) fn forget_combat(entity: &mut EntityWorldMut) {
    if let Some(mut threat) = entity.get_mut::<ThreatTable>() {
        threat.reset();
    }
    if let Some(mut state) = entity.get_mut::<CreatureState>() {
        *state = CreatureState::Idle;
    }
    if let Some(mut target) = entity.get_mut::<CombatTarget>() {
        target.0 = None;
    }
}

#[cfg(test)]
#[path = "creature_aggro_tests.rs"]
mod tests;

#[cfg(test)]
#[test]
fn idle_threat_full_population_is_bounded() {
    let mut app = App::new();
    app.add_systems(Update, replicate_threat_list_system);
    crate::test_data::populate_idle_world(app.world_mut(), || {
        (ThreatTable::default(), GridDormant)
    });
    crate::test_data::assert_idle_updates_are_bounded(
        &mut app,
        "idle_threat_full_population_is_bounded",
    );
}
