//! Creature-vs-creature aggro (docs/specs/creature-aggro.md): TrinityCore's AI relocation
//! notifies. A creature that moved, spawned or woke up is offered once per notify period
//! to every creature near it, and each of them to it (`Map::ProcessRelocationNotifies` →
//! `CreatureRelocationNotifier`, `CreatureUnitRelocationWorker`, GridNotifiers.cpp:126-201;
//! `Creature::Heartbeat` → `CreatureAggroGracePeriodExpiredNotifier`); an idle aggressive
//! creature attacks a creature it may start attacking (`CreatureAI::MoveInLineOfSight`,
//! `Creature::CanStartAttack`, Creature.cpp:2111-2148).
//!
//! Players are not part of this pass: `creature_aggro::aggro_on_sight_system` checks them.
//!
//! Neighbours come from `NpcGrid`, a hash of creatures by `MAX_AGGRO_RADIUS` cells per map
//! copy, updated from the interest queue's relocations, so a pass visits only the moved
//! creatures and their 3x3 cells, never every creature.
//!
//! Line of sight is the server search of `crate::line_of_sight`, run only for a pair that
//! may otherwise start a fight.
//!
//! Not modelled: the Z range (`CREATURE_Z_ATTACK_RANGE`; no collision
//! data, as for players), `IsNeutralToAll` (needs `Faction.ReputationIndex`, not imported;
//! no imported creature template is both hostile-by-default and group-less),
//! `IsEngagedBy`, stealth detection, no-gray-aggro (off by default).

use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use shared::components::{
    Health, Npc, Position, ThreatTable, UnitFactionTemplate, UnitFlags, UnitLevel,
};
use shared::level_scaling::{LevelScaling, level_for_viewer};

use crate::combat::DeathState;
use crate::creature_aggro::{AggroRange, LastDamagedTime, MAX_AGGRO_RADIUS};
use crate::creature_ai::{CreatureState, SpawnPoint};
use crate::faction::{FactionTemplates, Reaction, UnitFaction};
use crate::grid_activity::GridDormant;
use crate::interest::InterestDirtyQueue;
use crate::maps::InMap;
use crate::scripted_ai::ReactState;
use crate::world_data::WorldData;

/// `Visibility.Notify.Period.OnContinents` (worldserver.conf.dist:2382): relocations are
/// processed once a second.
const NOTIFY_PERIOD_SECS: f32 = 1.0;
/// `UNIT_FLAG_NON_ATTACKABLE`.
const UNIT_FLAG_NON_ATTACKABLE: u32 = 0x2;
/// `UNIT_FLAG_IMMUNE_TO_NPC`.
const UNIT_FLAG_IMMUNE_TO_NPC: u32 = 0x200;
/// `CREATURE_TYPE_NON_COMBAT_PET`.
const CREATURE_TYPE_NON_COMBAT_PET: u8 = 12;

pub struct CreatureSightPlugin;

impl Plugin for CreatureSightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InterestDirtyQueue>()
            .init_resource::<NpcGrid>()
            .init_resource::<SightPass>()
            .add_observer(index_new_npc)
            .add_observer(forget_npc)
            .add_observer(notify_woken)
            .add_systems(
                FixedUpdate,
                (resize_npcs, relocate_npcs, creature_sight_system)
                    .chain()
                    .before(crate::creature_aggro::update_victim_system),
            );
    }
}

/// A square of `MAX_AGGRO_RADIUS` yards of one map copy.
type Cell = (InMap, i32, i32);

fn distance(a: &Position, b: &Position) -> f32 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z).length()
}

fn cell_of(map: InMap, position: &Position) -> Cell {
    (
        map,
        (position.x / MAX_AGGRO_RADIUS).floor() as i32,
        (position.z / MAX_AGGRO_RADIUS).floor() as i32,
    )
}

/// Every creature in the world by cell.
#[derive(Resource, Default)]
pub(crate) struct NpcGrid {
    cells: HashMap<Cell, Vec<Entity>>,
    cell_of: EntityHashMap<Cell>,
    max_reach: HashMap<InMap, f32>,
}

impl NpcGrid {
    fn place(&mut self, npc: Entity, cell: Cell, reach: f32) {
        let maximum = self.max_reach.entry(cell.0).or_default();
        *maximum = maximum.max(reach);
        match self.cell_of.insert(npc, cell) {
            Some(previous) if previous == cell => return,
            Some(previous) => self.remove_from_cell(npc, previous),
            None => {}
        }
        self.cells.entry(cell).or_default().push(npc);
    }

    fn remove(&mut self, npc: Entity) {
        if let Some(cell) = self.cell_of.remove(&npc) {
            self.remove_from_cell(npc, cell);
        }
    }

    fn remove_from_cell(&mut self, npc: Entity, cell: Cell) {
        if let Some(npcs) = self.cells.get_mut(&cell) {
            npcs.retain(|&other| other != npc);
            if npcs.is_empty() {
                self.cells.remove(&cell);
            }
        }
    }

    /// GetAttackDistance clamps before IsWithinDist adds both model reaches.
    /// A conservative per-copy maximum keeps large/scaled bodies in the search.
    fn near(&self, map: InMap, position: &Position) -> impl Iterator<Item = Entity> + '_ {
        let radius = MAX_AGGRO_RADIUS + 2.0 * self.max_reach.get(&map).copied().unwrap_or(0.0);
        let min_x = ((position.x - radius) / MAX_AGGRO_RADIUS).floor() as i32;
        let max_x = ((position.x + radius) / MAX_AGGRO_RADIUS).floor() as i32;
        let min_z = ((position.z - radius) / MAX_AGGRO_RADIUS).floor() as i32;
        let max_z = ((position.z + radius) / MAX_AGGRO_RADIUS).floor() as i32;
        (min_x..=max_x)
            .flat_map(move |x| (min_z..=max_z).map(move |z| (map, x, z)))
            .filter_map(|cell| self.cells.get(&cell))
            .flatten()
            .copied()
    }
}

/// Creatures to offer to their neighbours at the next notify, and the time to it.
#[derive(Resource, Default)]
struct SightPass {
    pending: EntityHashSet,
    elapsed: f32,
}

/// A new creature is placed in the grid and seen at the next notify
/// (`Map::AddToMap` → `AIRelocationNotifier`, `Creature::Heartbeat` grace period).
fn index_new_npc(
    trigger: On<Add, Npc>,
    units: Query<(&Position, &InMap)>,
    attack_distance: crate::creature_aggro::AggroDistance,
    mut grid: ResMut<NpcGrid>,
    mut pass: ResMut<SightPass>,
) {
    let npc = trigger.entity;
    if let Ok((position, map)) = units.get(npc)
        && let Some(reach) = attack_distance.combat_reach(npc)
    {
        grid.place(npc, cell_of(*map, position), reach);
        pass.pending.insert(npc);
    }
}

fn forget_npc(trigger: On<Remove, Npc>, mut grid: ResMut<NpcGrid>, mut pass: ResMut<SightPass>) {
    grid.remove(trigger.entity);
    pass.pending.remove(&trigger.entity);
}

/// A creature whose grid a player woke is seen again (its grid is loaded and its
/// creatures added to the map in TrinityCore).
fn notify_woken(
    trigger: On<Remove, GridDormant>,
    units: Query<(&Position, &InMap), With<Npc>>,
    attack_distance: crate::creature_aggro::AggroDistance,
    mut grid: ResMut<NpcGrid>,
    mut pass: ResMut<SightPass>,
) {
    let npc = trigger.entity;
    if let Ok((position, map)) = units.get(npc)
        && let Some(reach) = attack_distance.combat_reach(npc)
    {
        grid.place(npc, cell_of(*map, position), reach);
    }
    pass.pending.insert(npc);
}

/// Display/scale changes (including Polymorph) change reach without movement.
fn resize_npcs(
    units: Query<
        (Entity, &Position, &InMap),
        (
            With<Npc>,
            Without<GridDormant>,
            Or<(
                Changed<shared::components::ModelDisplay>,
                Changed<shared::components::UnitScale>,
            )>,
        ),
    >,
    attack_distance: crate::creature_aggro::AggroDistance,
    mut grid: ResMut<NpcGrid>,
    mut pass: ResMut<SightPass>,
) {
    for (unit, position, map) in &units {
        if let Some(reach) = attack_distance.combat_reach(unit) {
            grid.place(unit, cell_of(*map, position), reach);
            pass.pending.insert(unit);
        }
    }
}

/// Each tick: creatures that moved take their new cell and wait for the next notify.
fn relocate_npcs(
    queue: Res<InterestDirtyQueue>,
    npcs: Query<(&Position, &InMap), With<Npc>>,
    attack_distance: crate::creature_aggro::AggroDistance,
    mut grid: ResMut<NpcGrid>,
    mut pass: ResMut<SightPass>,
) {
    for unit in queue.drain_relocated() {
        if let Ok((position, map)) = npcs.get(unit)
            && let Some(reach) = attack_distance.combat_reach(unit)
        {
            grid.place(unit, cell_of(*map, position), reach);
            pass.pending.insert(unit);
        }
    }
}

/// What `CanStartAttack` reads of a creature that might attack.
type Watchers<'w, 's> = Query<
    'w,
    's,
    (
        &'static AggroRange,
        &'static CreatureState,
        Option<&'static ReactState>,
        Option<&'static SpawnPoint>,
        Option<&'static LastDamagedTime>,
    ),
    With<ThreatTable>,
>;

/// What `CanStartAttack` reads of any creature.
type Units<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static InMap,
        &'static Health,
        &'static UnitLevel,
        Option<&'static LevelScaling>,
        Option<&'static UnitFactionTemplate>,
        Option<&'static UnitFlags>,
        Option<&'static CreatureState>,
        &'static Npc,
        Option<&'static crate::auras::Invisibility>,
        (Has<DeathState>, Has<GridDormant>),
    ),
>;

#[derive(bevy::ecs::system::SystemParam)]
struct SightParams<'w, 's> {
    watchers: Watchers<'w, 's>,
    units: Units<'w, 's>,
    templates: Res<'w, FactionTemplates>,
    attack_distance: crate::creature_aggro::AggroDistance<'w, 's>,
    world_data: Option<Res<'w, WorldData>>,
    maps: Option<Res<'w, crate::maps::MapStore>>,
    time: Res<'w, Time>,
    bodies: Query<'w, 's, crate::line_of_sight::BodyParts<'static>>,
    los: Option<Res<'w, crate::line_of_sight::LineOfSight>>,
}

/// One unit as `CanStartAttack` sees it.
struct UnitView<'a> {
    entity: Entity,
    position: &'a Position,
    map: InMap,
    level: UnitLevel,
    scaling: Option<&'a LevelScaling>,
    faction: u32,
    flags: u32,
    state: Option<CreatureState>,
    template: u32,
    targetable: bool,
    invisibility: crate::auras::Invisibility,
}

impl SightParams<'_, '_> {
    /// A living, active creature in the world.
    fn view(&self, unit: Entity) -> Option<UnitView<'_>> {
        let (
            position,
            map,
            health,
            level,
            scaling,
            faction,
            flags,
            state,
            npc,
            invisibility,
            (dead, dormant),
        ) = self.units.get(unit).ok()?;
        (health.current > 0.0 && !dead && !dormant).then(|| UnitView {
            entity: unit,
            position,
            map: *map,
            level: *level,
            scaling,
            faction: faction.map_or(0, |faction| faction.0),
            flags: flags.map_or(0, |flags| flags.0),
            state: state.copied(),
            template: npc.template_id,
            targetable: flags.is_none_or(|flags| flags.is_attackable() && flags.is_selectable()),
            invisibility: invisibility.cloned().unwrap_or_default(),
        })
    }

    fn evading(view: &UnitView) -> bool {
        view.state == Some(CreatureState::Returning)
    }

    fn non_combat_pet(&self, template: u32) -> bool {
        self.world_data
            .as_deref()
            .and_then(|data| data.templates.get(&template))
            .is_some_and(|template| template.creature_type == CREATURE_TYPE_NON_COMBAT_PET)
    }

    fn dungeon(&self, map: InMap) -> bool {
        self.maps
            .as_deref()
            .and_then(|maps| maps.map(map.map_id))
            .is_some_and(|entry| entry.is_dungeon())
    }

    /// `CreatureAI::MoveInLineOfSight(who)` of creature `watcher` (seen as `me`) for a
    /// creature `who` `distance` yards away: whether `watcher` attacks it.
    fn sight(&self, watcher: Entity, me: &UnitView, who: &UnitView, distance: f32) -> bool {
        let Ok((_, state, react, home, last_damaged)) = self.watchers.get(watcher) else {
            return false;
        };
        // `me->IsEngaged()`, `HasReactState(REACT_AGGRESSIVE)`.
        if !state.is_idle() || react.is_some_and(|react| *react != ReactState::Aggressive) {
            return false;
        }
        self.can_start_attack(me, who, distance)
            && crate::creature_aggro::within_reach_of_home(
                home,
                last_damaged,
                self.time.elapsed_secs_f64(),
                self.dungeon(me.map),
                who.position,
            )
            && self.sees(watcher, who.entity, me.map)
    }

    /// `IsWithinLOSInMap` (`CanStartAttack`), checked last: it casts rays.
    fn sees(&self, watcher: Entity, who: Entity, map: InMap) -> bool {
        match (self.bodies.get(watcher), self.bodies.get(who)) {
            (Ok(me), Ok(them)) => {
                crate::line_of_sight::unit_sees(self.los.as_deref(), map, me, them)
            }
            _ => false,
        }
    }

    /// `Creature::CanStartAttack(who, force = false)` for a creature `who`, with the
    /// faction, evade and map parts of `CanCreatureAttack`. The caller checked the home
    /// distance.
    fn can_start_attack(&self, me: &UnitView, who: &UnitView, distance: f32) -> bool {
        // IsImmuneToNPC (who is not player-controlled); no non-combat pets.
        if me.flags & UNIT_FLAG_IMMUNE_TO_NPC != 0 || self.non_combat_pet(who.template) {
            return false;
        }
        // _IsTargetAcceptable: not friendly, targetable (`isTargetableForAttack`), and
        // hostile; IsValidAttackTarget refuses a target immune to NPCs.
        let hostile = self.templates.reaction(me.faction, who.faction) == Reaction::Hostile;
        let attackable = who.targetable
            && me.invisibility.detects(&who.invisibility, true)
            && who.flags & (UNIT_FLAG_IMMUNE_TO_NPC | UNIT_FLAG_NON_ATTACKABLE) == 0
            && self.templates.can_attack(
                UnitFaction {
                    template: me.faction,
                    is_player: false,
                },
                UnitFaction {
                    template: who.faction,
                    is_player: false,
                },
            );
        // GetAttackDistance: each level against the other.
        let my_level = level_for_viewer(me.level, me.scaling, who.level.0);
        let its_level = level_for_viewer(who.level, who.scaling, me.level.0);
        let Some(radius) = self
            .attack_distance
            .center_radius(me.entity, who.entity, my_level, its_level)
        else {
            return false;
        };
        hostile
            && attackable
            && me.map == who.map
            && distance < radius
            && !Self::evading(me)
            && !Self::evading(who)
    }
}

/// `Map::ProcessRelocationNotifies`, every `NOTIFY_PERIOD_SECS`: each pending creature
/// and each creature near it see each other (`CreatureRelocationNotifier::Visit`); an idle
/// aggressive creature that may start attacking one of the creatures it saw attacks the
/// nearest.
fn creature_sight_system(
    mut pass: ResMut<SightPass>,
    grid: Res<NpcGrid>,
    params: SightParams,
    mut commands: Commands,
) {
    pass.elapsed += params.time.delta_secs();
    if pass.elapsed < NOTIFY_PERIOD_SECS {
        return;
    }
    pass.elapsed = 0.0;
    let mut nearest: EntityHashMap<(Entity, f32)> = EntityHashMap::default();
    let mut offer = |watcher: Entity, who: Entity, distance: f32| {
        let best = nearest.entry(watcher).or_insert((who, distance));
        if distance < best.1 {
            *best = (who, distance);
        }
    };
    for moved in std::mem::take(&mut pass.pending) {
        let Some(mover) = params.view(moved) else {
            continue;
        };
        for other in grid
            .near(mover.map, mover.position)
            .filter(|&other| other != moved)
        {
            let Some(seen) = params.view(other) else {
                continue;
            };
            let gap = distance(mover.position, seen.position);
            if params.sight(moved, &mover, &seen, gap) {
                offer(moved, other, gap);
            }
            if params.sight(other, &seen, &mover, gap) {
                offer(other, moved, gap);
            }
        }
    }
    for (watcher, (target, center_distance)) in nearest {
        debug!(
            creature = watcher.to_bits(),
            target = target.to_bits(),
            creature_position = ?params.view(watcher).map(|view| *view.position),
            target_position = ?params.view(target).map(|view| *view.position),
            map = ?params.view(watcher).map(|view| view.map),
            center_distance,
            "creature proximity acquisition"
        );
        commands.queue(move |world: &mut World| {
            crate::creature_aggro::attack_start(world, watcher, target);
        });
    }
}

#[cfg(test)]
#[path = "creature_sight_tests.rs"]
mod tests;
