//! Server power pools, regeneration and `UnitPowers` replication.
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
use shared::components::{CombatStatus, Health, Mana, Player, PowerEntry, PowerType, UnitPowers};

use crate::grid_activity::GridDormant;
use crate::spell_info::ClassSpells;

/// Alternate (encounter/quest/mount) bars are not class resources.
const ALTERNATE_POWERS: [i32; 4] = [10, 23, 24, 25];
/// Retail mana regen: 1% of gtBaseMP per second, in and out of combat
/// (TrinityCore `Player::UpdatePowerRegen` `basemana / 100`; SimC
/// `init_resources` "1% of base mana as mana regen per second").
const MANA_REGEN_BASE_PCT_PER_SEC: f32 = 0.01;

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
    /// `PowerTypeFlags::SetToMaxOnLevelUp` (0x1000): `Player::GiveLevel` fills it.
    pub set_to_max_on_level_up: bool,
}

/// `PowerTypeFlags::SetToMaxOnLevelUp` (DBCEnums.h).
const POWER_TYPE_FLAG_SET_TO_MAX_ON_LEVEL_UP: i64 = 0x1000;

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

    #[cfg(test)]
    pub fn fixture(infos: impl IntoIterator<Item = PowerTypeInfo>) -> Self {
        Self {
            powers: infos.into_iter().map(|info| (info.power, info)).collect(),
            display_power: HashMap::new(),
        }
    }

    /// `ChrClasses.DisplayPower` of `class_id`.
    pub fn display_power(&self, class_id: u8) -> Option<PowerType> {
        self.display_power
            .get(&class_id)
            .copied()
            .and_then(PowerType::from_db)
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
                DisplayModifier, RegenInterruptTimeMS, RegenPeace, RegenCombat, Flags
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
            r.get::<_, i64>(9)?,
        ))
    })?;
    let mut powers = HashMap::new();
    for row in rows {
        let (id, min, max, center, default, display, interrupt, peace, combat, flags) = row?;
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
            set_to_max_on_level_up: flags & POWER_TYPE_FLAG_SET_TO_MAX_ON_LEVEL_UP != 0,
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
    /// Signed raw units per second the last regen tick applied (0 idle).
    pub regen_per_sec: f32,
}

/// Server-authoritative powers of a unit. `order` lists every power (mana
/// included) primary first; `pools` holds all but mana (see module docs).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct PowerPools {
    pub order: Vec<PowerType>,
    pub pools: Vec<PowerPool>,
    /// When the unit last left combat (regen interrupt timer), seconds.
    pub out_of_combat_since: f64,
    /// Mana per second the last regen tick applied (0 idle or full).
    pub mana_regen_per_sec: f32,
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
                    regen_per_sec: 0.0,
                }
            })
            .collect();
        Self {
            order,
            pools,
            out_of_combat_since: 0.0,
            mana_regen_per_sec: 0.0,
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
    if power == PowerType::Runes && world.get::<super::runes::Runes>(entity).is_some() {
        return Some(change_runes(world, entity, delta));
    }
    let next = (current + delta).clamp(0.0, max);
    if power == PowerType::Mana {
        world.get_mut::<Mana>(entity)?.current = next;
    } else {
        world.get_mut::<PowerPools>(entity)?.get_mut(power)?.current = next;
    }
    Some(next - current)
}

/// Spend (negative) or refill (positive) whole runes; returns the applied change.
fn change_runes(world: &mut World, entity: Entity, delta: f32) -> f32 {
    let cooldown = super::runes::rune_cooldown_secs(
        world.get::<crate::auras::AuraStatMods>(entity),
        world.get::<crate::stats::CharacterStats>(entity),
    );
    let mut runes = world
        .get_mut::<super::runes::Runes>(entity)
        .expect("checked by change_power");
    let count = delta.abs().round() as usize;
    let changed = if delta < 0.0 {
        -(runes.spend(count, cooldown) as f32)
    } else {
        runes.refill(count) as f32
    };
    let ready = runes.ready() as f32;
    if let Some(pool) = world
        .get_mut::<PowerPools>(entity)
        .and_then(|pools| pools.into_inner().get_mut(PowerType::Runes))
    {
        pool.current = ready;
    }
    changed
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
        let pools = PowerPools::from_table(&table, order);
        if let Some(runes) = pools.get(PowerType::Runes) {
            commands
                .entity(entity)
                .insert(super::runes::Runes::new(runes.max as usize));
        }
        commands.entity(entity).insert((
            pools,
            UnitPowers::default(),
            PlayerHealthRegen::default(),
        ));
    }
}

/// `Player::UpdateMaxPower`: `MaxBasePower` plus MOD_INCREASE_ENERGY and MOD_MAX_POWER
/// (TOTAL_VALUE flat), rounded; `SetMaxPower` clamps the current value.
pub(super) fn update_max_powers(
    table: Option<Res<PowerTypeTable>>,
    mut units: Query<(&mut PowerPools, &crate::auras::AuraStatMods), Without<GridDormant>>,
) {
    let Some(table) = table else {
        return;
    };
    for (mut pools, mods) in &mut units {
        for pool in pools.bypass_change_detection().pools.iter_mut() {
            let Some(info) = table.get(pool.power) else {
                continue;
            };
            let max = (info.max_base_power as f32 + mods.max_power_flat(pool.power as i32)).round();
            if pool.max != max {
                pool.max = max.max(0.0);
                pool.current = pool.current.min(pool.max);
            }
        }
    }
}

type RegenQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut PowerPools,
        Option<&'static mut Mana>,
        Option<&'static CombatStatus>,
        Option<&'static crate::auras::AuraStatMods>,
        Option<&'static crate::stats::CharacterStats>,
        Option<&'static crate::death::PlayerDeathState>,
    ),
    Without<GridDormant>,
>;

/// Out of combat: `RegenPeace` after `RegenInterruptTimeMS` since leaving
/// combat. Negative rates decay toward `CenterPower` and positive rates fill
/// toward max (below center, a negative rate refills toward center). In
/// combat: `RegenCombat`. Rates (regen and decay) scale by the aura power
/// regen multiplier (MOD_POWER_REGEN_PERCENT, MOD_MANA_REGEN_PCT). Mana
/// (`power_type` rate 0) regenerates `MANA_REGEN_BASE_PCT_PER_SEC` of base
/// mana, scaled by the same multiplier. Dead players do not regenerate
/// (`Player::Update` calls `RegenerateAll` only `IsAlive()`, Player.cpp:1026-1030).
pub(crate) fn regenerate_powers(
    time: Res<Time>,
    table: Option<Res<PowerTypeTable>>,
    mut units: RegenQuery,
) {
    let Some(table) = table else {
        return;
    };
    let dt = time.delta_secs();
    let now = time.elapsed_secs_f64();
    for (mut pools, mana, combat, mods, stats, death) in &mut units {
        if death.is_some_and(|death| death.state != shared::death::DeathState::Alive) {
            continue;
        }
        let regen_mult = |power: PowerType| mods.map_or(1.0, |mods| mods.power_regen(power as i32));
        let in_combat = combat.is_some_and(|status| status.0);
        if in_combat {
            pools.out_of_combat_since = now;
        }
        let out_of_combat_ms = ((now - pools.out_of_combat_since) * 1000.0) as u32;
        // Runes act as cooldowns (`runes::regenerate_runes`).
        for pool in pools
            .bypass_change_detection()
            .pools
            .iter_mut()
            .filter(|pool| pool.power != PowerType::Runes)
        {
            let Some(info) = table.get(pool.power) else {
                continue;
            };
            let step = regen_step(pool.current, pool.max, info, in_combat, out_of_combat_ms);
            let Some((rate, limit)) = step else {
                pool.regen_per_sec = 0.0;
                continue;
            };
            let rate = rate * regen_mult(pool.power);
            let next = pool.current + rate * dt;
            pool.current = if rate > 0.0 {
                next.min(limit)
            } else {
                next.max(limit)
            };
            pool.regen_per_sec = if pool.current == limit { 0.0 } else { rate };
        }
        let mut mana_rate = 0.0;
        if let Some(mut mana) = mana
            && mana.current < mana.max
        {
            let base_mana = stats.map_or(0.0, |stats| stats.base_mana);
            mana_rate = base_mana * MANA_REGEN_BASE_PCT_PER_SEC * regen_mult(PowerType::Mana);
            mana.current = (mana.current + mana_rate * dt).min(mana.max);
            if mana.current == mana.max {
                mana_rate = 0.0;
            }
        }
        pools.bypass_change_detection().mana_regen_per_sec = mana_rate;
    }
}

/// Signed rate (raw units per second, before aura multipliers) and the value it
/// stops at, or `None` while the pool does not move.
fn regen_step(
    current: f32,
    max: f32,
    info: &PowerTypeInfo,
    in_combat: bool,
    out_of_combat_ms: u32,
) -> Option<(f32, f32)> {
    let (rate, limit) = if in_combat {
        let rate = info.regen_combat;
        (rate, if rate > 0.0 { max } else { 0.0 })
    } else {
        if out_of_combat_ms < info.regen_interrupt_ms {
            return None;
        }
        let center = info.center_power as f32;
        let peace = info.regen_peace;
        if peace > 0.0 {
            (peace, max)
        } else if current > center {
            (peace, center)
        } else {
            (-peace, center)
        }
    };
    (rate != 0.0 && current != limit).then_some((rate, limit))
}

/// Mirror pools (and `Mana`) into replicated `UnitPowers`: whole units floored, the
/// fraction as `partial` thousandths, the regen rate and the charged combo points.
/// Resent only when a whole value, max, rate or charged point changes; the client
/// predicts the fraction from the rate in between (retail sends power updates, not
/// per-tick fractions, and `UnitPartialPower` is client-side).
pub(crate) fn sync_unit_powers(
    mut units: Query<
        (
            Entity,
            &PowerPools,
            Option<&Mana>,
            Option<&crate::auras::AuraStatMods>,
            &mut UnitPowers,
        ),
        Without<GridDormant>,
    >,
    dirty: Option<Res<ReplicationDirtyQueue>>,
) {
    for (entity, pools, mana, mods, mut powers) in &mut units {
        let entry = |power: PowerType| {
            let (current, max, rate) = if power == PowerType::Mana {
                mana.map_or((0.0, 0.0, 0.0), |mana| {
                    (mana.current, mana.max, pools.mana_regen_per_sec)
                })
            } else {
                pools.get(power).map_or((0.0, 0.0, 0.0), |pool| {
                    (pool.current, pool.max, pool.regen_per_sec)
                })
            };
            PowerEntry {
                power,
                current: current.floor() as i32,
                max: max.floor() as i32,
                partial: ((current - current.floor()) * 1000.0).floor() as u16,
                regen_per_sec: rate,
            }
        };
        let entries: Vec<PowerEntry> = pools.order.iter().map(|&power| entry(power)).collect();
        let charged_points = mods.map_or_else(Vec::new, |mods| mods.charged_power_points.clone());
        let same_except_partial = |a: &PowerEntry, b: &PowerEntry| {
            (a.power, a.current, a.max, a.regen_per_sec)
                == (b.power, b.current, b.max, b.regen_per_sec)
        };
        let unchanged = powers.charged_points == charged_points
            && powers.entries.len() == entries.len()
            && powers
                .entries
                .iter()
                .zip(&entries)
                .all(|(sent, next)| same_except_partial(sent, next));
        if unchanged {
            continue;
        }
        *powers = UnitPowers {
            entries,
            charged_points,
        };
        if let Some(dirty) = &dirty {
            dirty.mark(entity);
        }
    }
}

/// `Player::RegenerateAll` regenerates health whenever `m_regenTimerCount` reaches 2000 ms
/// (Player.cpp:1623-1629).
const PLAYER_HEALTH_REGEN_INTERVAL_MS: u32 = 2000;

/// `m_regenTimerCount` (ms) and `m_healthFraction` of a player.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerHealthRegen {
    pub timer_ms: u32,
    pub fraction: f32,
}

/// What `Player::RegenerateHealth` reads of a player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HealthRegenInputs {
    pub level: u8,
    pub max: f32,
    pub in_combat: bool,
    pub polymorphed: bool,
    /// `IsStandState()`: not sitting, asleep or kneeling.
    pub standing: bool,
    /// MOD_HEALTH_REGEN_PERCENT 88 multiplier.
    pub regen_mult: f32,
    /// MOD_REGEN 84, health per 5 s.
    pub mod_regen: f32,
    /// MOD_REGEN_DURING_COMBAT 116 percent, while any is applied.
    pub regen_during_combat_pct: Option<f32>,
    /// MOD_HEALTH_REGEN_IN_COMBAT 161, health per tick, while any is applied.
    pub regen_in_combat: Option<f32>,
    /// `m_baseHealthRegen`: gear `ITEM_MOD_HEALTH_REGEN`, health per 5 s.
    pub base_health_regen: f32,
}

impl HealthRegenInputs {
    fn of(world: &World, player: Entity, max: f32) -> Self {
        let mods = world.get::<crate::auras::AuraStatMods>(player);
        Self {
            level: world
                .get::<shared::components::Level>(player)
                .map_or(1, |level| level.0),
            max,
            in_combat: world
                .get::<CombatStatus>(player)
                .is_some_and(|status| status.0),
            polymorphed: crate::auras::is_polymorphed(world, player),
            standing: crate::stand_state::stand_state(world, player).is_stand(),
            regen_mult: mods.map_or(1.0, |mods| mods.health_regen_mult),
            mod_regen: mods.map_or(0.0, |mods| mods.mod_regen),
            regen_during_combat_pct: mods.and_then(|mods| mods.regen_during_combat_pct),
            regen_in_combat: mods.and_then(|mods| mods.health_regen_in_combat),
            base_health_regen: world
                .get::<crate::stats::CharacterStats>(player)
                .map_or(0.0, |stats| stats.health_regen),
        }
    }

    /// `Player::RegenerateAll` (Player.cpp:1626) runs `RegenerateHealth` out of combat,
    /// or polymorphed, or with gear health regen, MOD_REGEN_DURING_COMBAT or
    /// MOD_HEALTH_REGEN_IN_COMBAT.
    fn regenerates(&self) -> bool {
        !self.in_combat
            || self.polymorphed
            || self.base_health_regen != 0.0
            || self.regen_during_combat_pct.is_some()
            || self.regen_in_combat.is_some()
    }
}

/// Health one `Player::RegenerateHealth` tick adds before `m_healthFraction`
/// (Player.cpp:1742-1781, `Rate.Health` 1): a third of max while polymorphed. Out of
/// combat 20% of max / level below level 15, else 1.5% of max, times
/// MOD_HEALTH_REGEN_PERCENT, plus 0.4 × MOD_REGEN (per 5 s to per 2 s); in combat with
/// MOD_REGEN_DURING_COMBAT `Rate.Health` × its percent. Either way ×1.5 while not
/// standing. Always plus MOD_HEALTH_REGEN_IN_COMBAT and `m_baseHealthRegen` / 2.5.
pub(super) fn player_health_regen(inputs: &HealthRegenInputs) -> f32 {
    let mut add = 0.0;
    if inputs.polymorphed {
        add = inputs.max / 3.0;
    } else if !inputs.in_combat || inputs.regen_during_combat_pct.is_some() {
        add = 1.0;
        if !inputs.in_combat {
            add = if inputs.level < 15 {
                0.20 * inputs.max / f32::from(inputs.level.max(1))
            } else {
                0.015 * inputs.max
            };
            add *= inputs.regen_mult;
            add += inputs.mod_regen * 0.4;
        } else if let Some(pct) = inputs.regen_during_combat_pct {
            add *= pct / 100.0;
        }
        if !inputs.standing {
            add *= 1.5;
        }
    }
    add + inputs.regen_in_combat.unwrap_or(0.0) + inputs.base_health_regen / 2.5
}

/// `Player::RegenerateAll` health: while alive (`Player::Update` `IsAlive()`) the timer
/// counts up and every 2 s `RegenerateHealth` adds the whole part of the tick's health plus
/// `m_healthFraction`, keeping the remainder; reaching max sets max and clears it. A
/// negative tick adds nothing and keeps the fraction.
pub(crate) fn regenerate_player_health(world: &mut World) {
    let dt_ms = world.resource::<Time>().delta().as_millis() as u32;
    let mut players = world.query_filtered::<(
        Entity,
        &mut PlayerHealthRegen,
        &Health,
        Option<&crate::death::PlayerDeathState>,
    ), Without<GridDormant>>();
    let due: Vec<Entity> = players
        .iter_mut(world)
        .filter(|(_, _, health, death)| {
            health.current > 0.0
                && death.is_none_or(|death| death.state == shared::death::DeathState::Alive)
        })
        .filter_map(|(entity, mut regen, ..)| {
            regen.timer_ms += dt_ms;
            (regen.timer_ms >= PLAYER_HEALTH_REGEN_INTERVAL_MS).then(|| {
                regen.timer_ms -= PLAYER_HEALTH_REGEN_INTERVAL_MS;
                entity
            })
        })
        .collect();
    for player in due {
        let Some(health) = world.get::<Health>(player).copied() else {
            continue;
        };
        let inputs = HealthRegenInputs::of(world, player, health.max);
        if health.current >= health.max || !inputs.regenerates() {
            continue;
        }
        let tick = player_health_regen(&inputs);
        if tick < 0.0 {
            continue;
        }
        let mut regen = world
            .get_mut::<PlayerHealthRegen>(player)
            .expect("queried above");
        let add = tick + regen.fraction;
        let whole = add.trunc();
        if whole == 0.0 {
            continue;
        }
        let current = if health.current + whole < health.max {
            regen.fraction = add - whole;
            health.current + whole
        } else {
            regen.fraction = 0.0;
            health.max
        };
        world.get_mut::<Health>(player).expect("read above").current = current;
        if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
            dirty.mark(player);
        }
    }
}

/// `CREATURE_REGEN_INTERVAL` (Creature.h), ms.
const CREATURE_REGEN_INTERVAL_SECS: f32 = 2.0;
/// `Creature::Regenerate(POWER_MANA)` for a creature without an owner:
/// `(27 / 5 + 17) × Rate.Power.Mana` (1) mana per interval, in and out of combat.
const CREATURE_MANA_PER_INTERVAL: f32 = 22.0;

/// `m_regenTimer` of a creature with `RegenHealth`, for its health.
#[derive(Component, Debug, Clone, Copy)]
pub struct CreatureHealthRegenTimer(pub f32);

impl Default for CreatureHealthRegenTimer {
    fn default() -> Self {
        Self(CREATURE_REGEN_INTERVAL_SECS)
    }
}

/// Polymorphed regeneration interval and share of max health per interval. The user's
/// Retail observation (2026-09-29): a sheep regains 5% of its max health every second,
/// empty to full in 20 s. TrinityCore instead gives `maxValue / 3` per
/// `CREATURE_REGEN_INTERVAL` (Creature.cpp:1023), which heals a sheep in 6 s.
const POLYMORPH_REGEN_INTERVAL_SECS: f32 = 1.0;
const POLYMORPH_REGEN_SHARE: f32 = 0.05;

/// `Creature::RegenerateHealth` for a creature without an owner: none while evading;
/// while polymorphed 5% of its max health every second (above); otherwise, while not
/// engaged (`!IsEngaged()`), a third of its max health every `CREATURE_REGEN_INTERVAL`.
/// The timer restarts at the interval of the state the tick found. MOD_HEALTH_REGEN_PERCENT
/// and MOD_REGEN are not modelled.
type ActiveHealthRegen =
    QueryState<(Entity, &'static mut CreatureHealthRegenTimer), Without<GridDormant>>;

pub(super) fn regenerate_creature_health(world: &mut World, creatures: &mut ActiveHealthRegen) {
    let dt = world.resource::<Time>().delta_secs();
    let due: Vec<Entity> = creatures
        .iter_mut(world)
        .filter_map(|(entity, mut timer)| {
            timer.0 -= dt;
            (timer.0 <= 0.0).then_some(entity)
        })
        .collect();
    for creature in due {
        let polymorphed = crate::auras::is_polymorphed(world, creature);
        let interval = if polymorphed {
            POLYMORPH_REGEN_INTERVAL_SECS
        } else {
            CREATURE_REGEN_INTERVAL_SECS
        };
        if let Some(mut timer) = world.get_mut::<CreatureHealthRegenTimer>(creature) {
            timer.0 += interval;
        }
        let state = world.get::<crate::creature_ai::CreatureState>(creature);
        if state == Some(&crate::creature_ai::CreatureState::Returning) {
            continue;
        }
        let engaged = matches!(
            state,
            Some(crate::creature_ai::CreatureState::Pursuing { .. })
        );
        if engaged && !polymorphed {
            continue;
        }
        let Some(mut health) = world.get_mut::<Health>(creature) else {
            continue;
        };
        if health.current <= 0.0 || health.current >= health.max {
            continue;
        }
        let gain = if polymorphed {
            (health.max * POLYMORPH_REGEN_SHARE).floor()
        } else {
            (health.max / 3.0).floor()
        };
        health.current = (health.current + gain).min(health.max);
        if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
            dirty.mark(creature);
        }
    }
}

/// `m_regenTimer` of a creature with `UNIT_FLAG2_REGENERATE_POWER`.
#[derive(Component, Debug, Clone, Copy)]
pub struct CreatureRegenTimer(pub f32);

impl Default for CreatureRegenTimer {
    fn default() -> Self {
        Self(CREATURE_REGEN_INTERVAL_SECS)
    }
}

/// Creature mana regen every `CREATURE_REGEN_INTERVAL`, scaled by
/// MOD_POWER_REGEN_PERCENT; none while evading (`!IsInEvadeMode()`).
pub(super) fn regenerate_creature_mana(
    time: Res<Time>,
    mut creatures: Query<
        (
            &mut CreatureRegenTimer,
            &mut Mana,
            Option<&crate::creature_ai::CreatureState>,
            Option<&crate::auras::AuraStatMods>,
        ),
        Without<GridDormant>,
    >,
) {
    let dt = time.delta_secs();
    for (mut timer, mut mana, state, mods) in &mut creatures {
        timer.0 -= dt;
        if timer.0 > 0.0 {
            continue;
        }
        timer.0 += CREATURE_REGEN_INTERVAL_SECS;
        let evading = state == Some(&crate::creature_ai::CreatureState::Returning);
        if evading || mana.current >= mana.max {
            continue;
        }
        let gain = CREATURE_MANA_PER_INTERVAL
            * mods.map_or(1.0, |mods| mods.power_regen(PowerType::Mana as i32));
        mana.current = (mana.current + gain).min(mana.max);
    }
}

#[cfg(test)]
#[test]
fn idle_health_regen_full_population_is_bounded() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.add_systems(Update, regenerate_creature_health);
    crate::test_data::populate_idle_world(app.world_mut(), || {
        (
            CreatureHealthRegenTimer::default(),
            GridDormant,
            Health {
                current: 100.0,
                max: 100.0,
            },
        )
    });
    crate::test_data::assert_idle_updates_are_bounded(
        &mut app,
        "idle_health_regen_full_population_is_bounded",
    );
}
