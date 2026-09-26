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
use shared::components::{Health, Player, Position, ThreatTable, UnitFactionTemplate, UnitLevel};

use crate::combat::DeathState;
use crate::creature_ai::{CreatureState, chase_system};
use crate::death::PlayerDeathState;
use crate::faction::{FactionTemplates, Reaction, UnitFaction};
use crate::grid_activity::GridDormant;
use crate::networking::CombatTarget;
use crate::taxi::TaxiFlight;

/// `MAX_AGGRO_RADIUS` (Unit.h:45).
const MAX_AGGRO_RADIUS: f32 = 45.0;
/// Minimum aggro radius of a non-pet creature (`GetAggroRange`).
const MIN_AGGRO_RADIUS: f32 = 5.0;
/// `GetAggroRange` caps the level difference at 25 below the creature.
const MAX_LEVEL_DIFF_BELOW: i32 = -25;
/// `ATTACK_DISTANCE` (ObjectDefines.h:25): melee range for the threat
/// swap threshold.
const ATTACK_DISTANCE: f32 = 5.0;

const CREATURE_TYPE_CRITTER: u32 = 8;
const CREATURE_TYPE_NON_COMBAT_PET: u32 = 12;
const CREATURE_FLAG_EXTRA_CIVILIAN: u32 = 0x2;
const CREATURE_FLAG_EXTRA_TRIGGER: u32 = 0x80;
const UNIT_FLAG_IMMUNE_TO_PC: u32 = 0x100;

pub struct CreatureAggroPlugin;

impl Plugin for CreatureAggroPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (aggro_on_sight_system, update_victim_system)
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

/// `creature_template.detection_range`: aggro radius against a same-level
/// target, before level scaling.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct AggroRange(pub f32);

/// Spawn-time combat components: a threat list for every non-passive
/// creature, an aggro radius for aggressive ones.
pub(crate) fn insert_aggro_components(
    entity: &mut EntityCommands,
    mode: AggroMode,
    detection_range: f32,
) {
    match mode {
        AggroMode::Passive => {}
        AggroMode::Reactive => {
            entity.insert((ThreatTable::default(), CreatureState::Idle));
        }
        AggroMode::Aggressive => {
            entity.insert((
                ThreatTable::default(),
                CreatureState::Idle,
                AggroRange(detection_range),
            ));
        }
    }
}

/// `Creature::GetAggroRange` with `Rate.Creature.Aggro` 1 and no detect
/// range auras: the detection range grows 1 yard per level the creature is
/// above the target, clamped to 5..45 yards; under 1 yard never aggroes.
pub fn aggro_radius(detection_range: f32, creature_level: u8, target_level: u8) -> f32 {
    if detection_range < 1.0 {
        return 0.0;
    }
    let level_diff =
        (i32::from(target_level) - i32::from(creature_level)).max(MAX_LEVEL_DIFF_BELOW);
    (detection_range - level_diff as f32).clamp(MIN_AGGRO_RADIUS, MAX_AGGRO_RADIUS)
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
    ),
>;

/// A unit a creature may keep attacking: alive (not a corpse or ghost), not
/// on a flight path, and attackable by the creature's faction
/// (`Creature::_IsTargetAcceptable` / `CanCreatureAttack`).
fn valid_victim(
    victims: &Victims,
    templates: &FactionTemplates,
    creature_faction: u32,
    victim: Entity,
) -> bool {
    let Ok((_, health, faction, _, death, is_player, flying, npc_dead)) = victims.get(victim)
    else {
        return false;
    };
    let alive = health.current > 0.0
        && !npc_dead
        && death.is_none_or(|death| death.state == shared::death::DeathState::Alive);
    alive
        && !flying
        && templates.can_attack(
            UnitFaction {
                template: creature_faction,
                is_player: false,
            },
            UnitFaction {
                template: faction.map_or(0, |faction| faction.0),
                is_player,
            },
        )
}

type Watchers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Position,
        &'static AggroRange,
        &'static UnitLevel,
        &'static UnitFactionTemplate,
        &'static CreatureState,
    ),
    (Without<DeathState>, Without<GridDormant>),
>;

/// `CreatureAI::MoveInLineOfSight` → `Creature::CanStartAttack`: an idle
/// aggressive creature attacks the nearest valid player it is hostile to
/// inside its level-scaled aggro radius. There is no line-of-sight or
/// Z-difference test: the server has no collision data and does not keep
/// creature heights on the ground while they move.
fn aggro_on_sight_system(
    watchers: Watchers,
    players: Query<Entity, With<Player>>,
    victims: Victims,
    templates: Res<FactionTemplates>,
    mut commands: Commands,
) {
    for (creature, position, range, level, faction, state) in &watchers {
        if !state.is_idle() {
            continue;
        }
        let nearest = players
            .iter()
            .filter_map(|player| {
                let (player_pos, _, player_faction, player_level, ..) = victims.get(player).ok()?;
                let distance = distance(position, player_pos);
                let radius = aggro_radius(range.0, level.0, player_level.map_or(1, |l| l.0));
                let hostile = templates.reaction(faction.0, player_faction.map_or(0, |f| f.0))
                    == Reaction::Hostile;
                (distance <= radius
                    && hostile
                    && valid_victim(&victims, &templates, faction.0, player))
                .then_some((player, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((player, _)) = nearest {
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
    ),
    (Without<DeathState>, Without<GridDormant>),
>;

/// `CreatureAI::UpdateVictim` / `Creature::SelectVictim`: drop threat
/// entries that are no longer valid targets, attack the top-threat unit
/// (a challenger needs 110% of the current target's threat in melee range,
/// 130% beyond it: `ThreatMgr::getHostileTarget`), and evade when no valid
/// target is left.
fn update_victim_system(
    mut engaged: Engaged,
    victims: Victims,
    templates: Res<FactionTemplates>,
    mut commands: Commands,
) {
    for (creature, position, faction, mut threat, mut state, combat_target) in &mut engaged {
        let CreatureState::Pursuing { target } = *state else {
            continue;
        };
        let valid = |unit: u64| {
            Entity::try_from_bits(unit)
                .is_some_and(|unit| valid_victim(&victims, &templates, faction.0, unit))
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
        let Some(victim) = threat.aggro_target(current, top_in_melee) else {
            continue;
        };
        if victim == target {
            continue;
        }
        *state = CreatureState::Pursuing { target: victim };
        let victim = Entity::from_bits(victim);
        match combat_target {
            Some(mut combat_target) => combat_target.0 = Some(victim),
            None => {
                commands.entity(creature).insert(CombatTarget(Some(victim)));
            }
        }
    }
}

fn distance(a: &Position, b: &Position) -> f32 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z).length()
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
    if creature == attacker || !can_engage(world, creature) {
        return;
    }
    if let Some(mut threat) = world.get_mut::<ThreatTable>(creature) {
        threat.add_threat(attacker.to_bits(), amount);
    }
    if world.get::<CreatureState>(creature) == Some(&CreatureState::Idle) {
        world.entity_mut(creature).insert((
            CreatureState::Pursuing {
                target: attacker.to_bits(),
            },
            CombatTarget(Some(attacker)),
        ));
    }
    crate::combat::enter_combat(world, creature);
    crate::combat::enter_combat(world, attacker);
}

/// `CreatureAI::EnterEvadeMode`: clear threat, stop combat (the creature
/// and every unit auto-attacking it stop swinging), restore full health
/// and run home (`MoveTargetedHome`).
pub(crate) fn enter_evade_mode(world: &mut World, creature: Entity) {
    if world.get::<DeathState>(creature).is_some() {
        return;
    }
    let mut entity = world.entity_mut(creature);
    entity.insert(CreatureState::Returning);
    if let Some(mut threat) = entity.get_mut::<ThreatTable>() {
        threat.reset();
    }
    if let Some(mut target) = entity.get_mut::<CombatTarget>() {
        target.0 = None;
    }
    if let Some(mut health) = entity.get_mut::<Health>()
        && health.current != health.max
    {
        health.current = health.max;
        if let Some(queue) = world.get_resource::<ReplicationDirtyQueue>() {
            queue.mark(creature);
        }
    }
    crate::combat::combat_stop(world, creature);
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
