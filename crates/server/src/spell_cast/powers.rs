//! Server power pools, regeneration and `UnitPowers`/`UnitLevel` replication.
//!
//! Units: every pool stores the DB2 raw value (`power_type` rows, `SpellPower`
//! costs and `ENERGIZE` base points share it). Rage, Runic Power, Soul Shards,
//! Lunar Power and Pain are tenths (`DisplayModifier` 10), Insanity hundredths.
//! `UnitPowers` exposes the same raw `current/max`; display scaling is the
//! client's job via `PowerType`.
//!
//! Mana is the exception to `PowerPools` storage: the legacy `Mana` component
//! (max derived by `stats.rs`) stays authoritative for mana so the client that
//! still reads it keeps working. Retirement: when the client reads
//! `UnitPowers`, move mana into `PowerPools` and delete `Mana`.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use rusqlite::Connection;
use shared::components::{
    CombatStatus, Level, Mana, Player, PowerEntry, PowerType, UnitLevel, UnitPowers,
};

use crate::spell_info::ClassSpells;

/// Alternate (encounter/quest/mount) bars are not class resources.
const ALTERNATE_POWERS: [i32; 4] = [10, 23, 24, 25];
/// `power_type` mana regen is 0; placeholder until mana regen stats exist
/// (not retail-derived): 1% of max mana per second, in and out of combat.
const MANA_REGEN_PCT_PER_SEC: f32 = 0.01;

/// One `power_type` row, raw DB2 values. Regen is raw units per second
/// (TrinityCore `Player::Regenerate`: `RegenPeace * 0.001 * elapsed_ms`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerTypeInfo {
    pub power: PowerType,
    pub min_power: i32,
    pub max_base_power: i32,
    pub center_power: i32,
    pub default_power: i32,
    pub display_modifier: i32,
    pub regen_interrupt_ms: u32,
    pub regen_peace: f32,
    pub regen_combat: f32,
}

/// `power_type` rows by `PowerTypeEnum`, plus `chr_classes.DisplayPower`.
#[derive(Resource, Debug, Default)]
pub struct PowerTypeTable {
    powers: HashMap<PowerType, PowerTypeInfo>,
    display_power: HashMap<u8, i32>,
}

impl PowerTypeTable {
    pub fn get(&self, power: PowerType) -> Option<&PowerTypeInfo> {
        self.powers.get(&power)
    }

    /// Class powers, primary (`DisplayPower`) first, then ascending DB ID.
    pub fn class_powers(&self, class_id: u8, class_power_types: &[i32]) -> Vec<PowerType> {
        let display = self.display_power.get(&class_id).copied();
        let mut ids: Vec<i32> = class_power_types
            .iter()
            .copied()
            .filter(|id| !ALTERNATE_POWERS.contains(id))
            .collect();
        if let Some(display) = display
            && !ids.contains(&display)
        {
            ids.push(display);
        }
        ids.sort_by_key(|&id| (Some(id) != display, id));
        ids.into_iter().filter_map(PowerType::from_db).collect()
    }
}

pub fn load_power_type_table(conn: &Connection) -> rusqlite::Result<PowerTypeTable> {
    let mut stmt = conn.prepare(
        "SELECT PowerTypeEnum, MinPower, MaxBasePower, CenterPower, DefaultPower,
                DisplayModifier, RegenInterruptTimeMS, RegenPeace, RegenCombat
         FROM power_type ORDER BY ID",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)? as i32,
            r.get::<_, i64>(1)? as i32,
            r.get::<_, i64>(2)? as i32,
            r.get::<_, i64>(3)? as i32,
            r.get::<_, i64>(4)? as i32,
            r.get::<_, i64>(5)? as i32,
            r.get::<_, i64>(6)? as u32,
            r.get::<_, f64>(7)? as f32,
            r.get::<_, f64>(8)? as f32,
        ))
    })?;
    let mut powers = HashMap::new();
    for row in rows {
        let (id, min, max, center, default, display, interrupt, peace, combat) = row?;
        let Some(power) = PowerType::from_db(id) else {
            continue;
        };
        // First row per enum wins (the three ALTERNATE rows share nothing we use).
        powers.entry(power).or_insert(PowerTypeInfo {
            power,
            min_power: min,
            max_base_power: max,
            center_power: center,
            default_power: default,
            display_modifier: display,
            regen_interrupt_ms: interrupt,
            regen_peace: peace,
            regen_combat: combat,
        });
    }
    let mut stmt = conn.prepare("SELECT ID, DisplayPower FROM chr_classes")?;
    let display_power = stmt
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)? as u8, r.get::<_, i64>(1)? as i32))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(PowerTypeTable {
        powers,
        display_power,
    })
}

pub(super) fn load_power_type_table_startup(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    let table = if path.exists() {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("Failed to open world.db for power types");
        load_power_type_table(&conn).expect("Failed to load power_type from world.db")
    } else {
        warn!("world.db not found, power type table empty");
        PowerTypeTable::default()
    };
    commands.insert_resource(table);
}

/// One non-mana pool in raw units; fractional regen accumulates in `current`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerPool {
    pub power: PowerType,
    pub current: f32,
    pub max: f32,
}

/// Server-authoritative powers of a unit. `order` lists every power (mana
/// included) primary first; `pools` holds all but mana (see module docs).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct PowerPools {
    pub order: Vec<PowerType>,
    pub pools: Vec<PowerPool>,
    /// When the unit last left combat (regen interrupt timer), seconds.
    pub out_of_combat_since: f64,
}

impl PowerPools {
    pub fn from_table(table: &PowerTypeTable, order: Vec<PowerType>) -> Self {
        let pools = order
            .iter()
            .filter(|&&power| power != PowerType::Mana)
            .map(|&power| {
                let info = table.get(power);
                PowerPool {
                    power,
                    current: info.map_or(0.0, |info| info.default_power as f32),
                    max: info.map_or(0.0, |info| info.max_base_power as f32),
                }
            })
            .collect();
        Self {
            order,
            pools,
            out_of_combat_since: 0.0,
        }
    }

    pub fn get(&self, power: PowerType) -> Option<&PowerPool> {
        self.pools.iter().find(|pool| pool.power == power)
    }

    pub fn get_mut(&mut self, power: PowerType) -> Option<&mut PowerPool> {
        self.pools.iter_mut().find(|pool| pool.power == power)
    }
}

/// `(current, max)` raw of `power` on `entity`; mana reads `Mana`.
pub fn power_value(world: &World, entity: Entity, power: PowerType) -> Option<(f32, f32)> {
    if power == PowerType::Mana {
        return world
            .get::<Mana>(entity)
            .map(|mana| (mana.current, mana.max));
    }
    world
        .get::<PowerPools>(entity)?
        .get(power)
        .map(|pool| (pool.current, pool.max))
}

/// Add `delta` raw units clamped to `[0, max]`; returns the applied change.
pub fn change_power(
    world: &mut World,
    entity: Entity,
    power: PowerType,
    delta: f32,
) -> Option<f32> {
    let (current, max) = power_value(world, entity, power)?;
    let next = (current + delta).clamp(0.0, max);
    if power == PowerType::Mana {
        world.get_mut::<Mana>(entity)?.current = next;
    } else {
        world.get_mut::<PowerPools>(entity)?.get_mut(power)?.current = next;
    }
    Some(next - current)
}

/// Give players their class pools once `ClassSpells` and the table are loaded.
pub(super) fn init_player_powers(
    mut commands: Commands,
    players: Query<(Entity, &Player), Without<PowerPools>>,
    table: Option<Res<PowerTypeTable>>,
    classes: Option<Res<ClassSpells>>,
) {
    let (Some(table), Some(classes)) = (table, classes) else {
        return;
    };
    for (entity, player) in &players {
        let class_power_types = classes
            .classes
            .iter()
            .find(|class| class.class_id == player.class)
            .map_or(&[][..], |class| class.power_types.as_slice());
        let order = table.class_powers(player.class, class_power_types);
        commands
            .entity(entity)
            .insert((PowerPools::from_table(&table, order), UnitPowers::default()));
    }
}

type RegenQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut PowerPools,
        Option<&'static mut Mana>,
        Option<&'static CombatStatus>,
    ),
>;

/// Out of combat: `RegenPeace` after `RegenInterruptTimeMS` since leaving
/// combat. Negative rates decay toward `CenterPower` and positive rates fill
/// toward max (below center, a negative rate refills toward center). In
/// combat: `RegenCombat`.
pub(super) fn regenerate_powers(
    time: Res<Time>,
    table: Option<Res<PowerTypeTable>>,
    mut units: RegenQuery,
) {
    let Some(table) = table else {
        return;
    };
    let dt = time.delta_secs();
    let now = time.elapsed_secs_f64();
    for (mut pools, mana, combat) in &mut units {
        let in_combat = combat.is_some_and(|status| status.0);
        if in_combat {
            pools.out_of_combat_since = now;
        }
        let out_of_combat_ms = ((now - pools.out_of_combat_since) * 1000.0) as u32;
        for pool in pools.bypass_change_detection().pools.iter_mut() {
            let Some(info) = table.get(pool.power) else {
                continue;
            };
            let next = regenerated(
                pool.current,
                pool.max,
                info,
                in_combat,
                out_of_combat_ms,
                dt,
            );
            pool.current = next;
        }
        if let Some(mut mana) = mana
            && mana.current < mana.max
        {
            mana.current = (mana.current + mana.max * MANA_REGEN_PCT_PER_SEC * dt).min(mana.max);
        }
    }
}

fn regenerated(
    current: f32,
    max: f32,
    info: &PowerTypeInfo,
    in_combat: bool,
    out_of_combat_ms: u32,
    dt: f32,
) -> f32 {
    if in_combat {
        return (current + info.regen_combat * dt).clamp(0.0, max);
    }
    if out_of_combat_ms < info.regen_interrupt_ms || info.regen_peace == 0.0 {
        return current;
    }
    let step = info.regen_peace.abs() * dt;
    let center = info.center_power as f32;
    if info.regen_peace > 0.0 {
        (current + step).min(max)
    } else if current > center {
        (current - step).max(center)
    } else {
        (current + step).min(center)
    }
}

/// Mirror pools (and `Mana`) into replicated `UnitPowers`; floor fractions.
pub(super) fn sync_unit_powers(
    mut units: Query<(Entity, &PowerPools, Option<&Mana>, &mut UnitPowers)>,
    dirty: Option<Res<ReplicationDirtyQueue>>,
) {
    for (entity, pools, mana, mut powers) in &mut units {
        let entries = pools
            .order
            .iter()
            .map(|&power| {
                let (current, max) = if power == PowerType::Mana {
                    mana.map_or((0.0, 0.0), |mana| (mana.current, mana.max))
                } else {
                    pools
                        .get(power)
                        .map_or((0.0, 0.0), |pool| (pool.current, pool.max))
                };
                PowerEntry {
                    power,
                    current: current.floor() as i32,
                    max: max.floor() as i32,
                }
            })
            .collect::<Vec<_>>();
        if powers.entries != entries {
            powers.entries = entries;
            if let Some(dirty) = &dirty {
                dirty.mark(entity);
            }
        }
    }
}

/// Replicate the player's `Level` as `UnitLevel`.
pub(super) fn sync_unit_level(
    mut commands: Commands,
    players: Query<(Entity, &Level, Option<&UnitLevel>), Or<(Changed<Level>, Without<UnitLevel>)>>,
    dirty: Option<Res<ReplicationDirtyQueue>>,
) {
    for (entity, level, unit_level) in &players {
        if unit_level.is_some_and(|unit_level| unit_level.0 == level.0) {
            continue;
        }
        commands.entity(entity).insert(UnitLevel(level.0));
        if let Some(dirty) = &dirty {
            dirty.mark(entity);
        }
    }
}
