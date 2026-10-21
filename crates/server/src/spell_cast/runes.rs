//! Death Knight runes as per-rune cooldowns (TrinityCore `Runes`, `Player::SetRuneCooldown`,
//! the rune branch of `Player::RegenerateAll`, `Spell::TakeRunePower`). The Runes pool
//! of `PowerPools` holds the count of ready runes (`SetRuneCooldown` → `SetPower`), so
//! cost checks read it like any pool.

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{PowerType, UnitRunes};

use super::powers::PowerPools;
use crate::grid_activity::GridDormant;

/// `RUNE_BASE_COOLDOWN` (Player.h), seconds.
pub const RUNE_BASE_COOLDOWN_SECS: f32 = 10.0;
/// `MAX_RECHARGING_RUNES` (Player.h): only the first depleted runes recharge.
pub const MAX_RECHARGING_RUNES: usize = 3;

/// Per-rune cooldowns of a unit with a Runes pool.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct Runes {
    /// Seconds left per rune index; 0 is ready.
    pub cooldowns: Vec<f32>,
    /// Depleted rune indexes in depletion order (`Runes::CooldownOrder`).
    pub order: Vec<u8>,
}

impl Runes {
    pub fn new(count: usize) -> Self {
        Self {
            cooldowns: vec![0.0; count],
            order: Vec::new(),
        }
    }

    pub fn ready(&self) -> usize {
        self.cooldowns.iter().filter(|&&secs| secs == 0.0).count()
    }

    /// `SetRuneCooldown` + `Runes::SetRuneState`: a depleted rune joins the back of
    /// the order, a ready one leaves it.
    fn set(&mut self, index: usize, secs: f32) {
        self.cooldowns[index] = secs;
        let position = self
            .order
            .iter()
            .position(|&rune| usize::from(rune) == index);
        match (secs == 0.0, position) {
            (true, Some(position)) => {
                self.order.remove(position);
            }
            (false, None) => self.order.push(index as u8),
            _ => {}
        }
    }

    /// `Spell::TakeRunePower`: the lowest-index ready runes start `cooldown`.
    /// Returns how many were spent.
    pub fn spend(&mut self, count: usize, cooldown: f32) -> usize {
        let ready: Vec<usize> = (0..self.cooldowns.len())
            .filter(|&index| self.cooldowns[index] == 0.0)
            .take(count)
            .collect();
        for &index in &ready {
            self.set(index, cooldown);
        }
        ready.len()
    }

    /// Finish the `count` depleted runes closest to ready (the front of the order).
    pub fn refill(&mut self, count: usize) -> usize {
        let front: Vec<usize> = self
            .order
            .iter()
            .take(count)
            .map(|&rune| usize::from(rune))
            .collect();
        for &index in &front {
            self.set(index, 0.0);
        }
        front.len()
    }

    /// The rune branch of `Player::RegenerateAll`: up to `MAX_RECHARGING_RUNES` runes
    /// from the front of the order lose `dt`; a finished rune leaves the order and the
    /// next one moves up, its leftover time dropped.
    pub(crate) fn regenerate(&mut self, dt: f32) {
        let mut regenerated = 0;
        let mut index = 0;
        while regenerated < MAX_RECHARGING_RUNES && index < self.order.len() {
            let rune = usize::from(self.order[index]);
            let left = self.cooldowns[rune];
            if left > dt {
                self.set(rune, left - dt);
                index += 1;
            } else {
                self.set(rune, 0.0);
            }
            regenerated += 1;
        }
    }

    /// The pool max changed (e.g. Runic Longevity): add ready runes or drop the last.
    fn resize(&mut self, count: usize) {
        if count == self.cooldowns.len() {
            return;
        }
        self.cooldowns.resize(count, 0.0);
        self.order.retain(|&rune| usize::from(rune) < count);
    }

    /// Seconds until each rune is ready if no more are spent: the recharging runes'
    /// cooldowns, then each waiting rune starts `duration` when the earliest recharge
    /// slot frees (retail `GetRuneCooldown` start times in the future).
    pub fn ready_in(&self, duration: f32) -> Vec<f32> {
        let mut ready_in = self.cooldowns.clone();
        let mut slots: Vec<f32> = self
            .order
            .iter()
            .take(MAX_RECHARGING_RUNES)
            .map(|&rune| self.cooldowns[usize::from(rune)])
            .collect();
        for &rune in self.order.iter().skip(MAX_RECHARGING_RUNES) {
            let (slot, start) = slots
                .iter()
                .copied()
                .enumerate()
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .expect("waiting runes imply recharging slots");
            ready_in[usize::from(rune)] = start + duration;
            slots[slot] = start + duration;
        }
        ready_in
    }
}

/// `Player::GetRuneBaseCooldown`: `RUNE_BASE_COOLDOWN` shortened by MOD_POWER_REGEN_PERCENT
/// on runes and by haste. Both are applied as rate multipliers (base / (mult × (1 +
/// haste))), like every other regen multiplier here; TrinityCore multiplies by (1 −
/// pct/100) instead.
pub fn rune_cooldown_secs(
    mods: Option<&crate::auras::AuraStatMods>,
    stats: Option<&crate::stats::CharacterStats>,
) -> f32 {
    let regen = mods.map_or(1.0, |mods| mods.power_regen(PowerType::Runes as i32));
    let haste = 1.0 + stats.map_or(0.0, |stats| stats.haste_pct) / 100.0;
    RUNE_BASE_COOLDOWN_SECS / (regen * haste).max(f32::EPSILON)
}

/// Recharge runes and mirror the ready count (and the pool max) into the Runes pool.
pub(super) fn regenerate_runes(
    mut commands: Commands,
    time: Res<Time>,
    mut units: Query<
        (
            Entity,
            &mut Runes,
            &mut PowerPools,
            Option<&crate::auras::ActiveAuras>,
        ),
        Without<GridDormant>,
    >,
) {
    let dt = time.delta_secs();
    for (unit, mut runes, mut pools, auras) in &mut units {
        let Some(pool) = pools.bypass_change_detection().get_mut(PowerType::Runes) else {
            continue;
        };
        runes.bypass_change_detection().resize(pool.max as usize);
        if !runes.order.is_empty() {
            runes.bypass_change_detection().regenerate(dt);
        }
        let old = pool.current;
        pool.current = runes.ready() as f32;
        crate::auras::power_amount_trigger::queue_regenerated(
            &mut commands,
            unit,
            auras,
            (PowerType::Runes, old, pool.current),
        );
    }
}

/// `(depletion order, duration ms, rune count)` of the last `UnitRunes` send.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct RunesSent(Vec<u8>, u32, usize);

type RuneSyncQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Runes,
        Option<&'static crate::auras::AuraStatMods>,
        Option<&'static crate::stats::CharacterStats>,
        Option<&'static RunesSent>,
    ),
    Without<GridDormant>,
>;

/// Replicate `UnitRunes` when a rune is spent or finishes (the depletion order changes)
/// or the rune cooldown changes; the client counts down `ready_in_ms` in between.
pub(super) fn sync_unit_runes(
    mut commands: Commands,
    units: RuneSyncQuery,
    dirty: Option<Res<ReplicationDirtyQueue>>,
) {
    for (entity, runes, mods, stats, sent) in &units {
        let duration = rune_cooldown_secs(mods, stats);
        let duration_ms = (duration * 1000.0).round() as u32;
        let key = RunesSent(runes.order.clone(), duration_ms, runes.cooldowns.len());
        if sent == Some(&key) {
            continue;
        }
        let ready_in_ms = runes
            .ready_in(duration)
            .into_iter()
            .map(|secs| (secs * 1000.0).round() as u32)
            .collect();
        commands.entity(entity).insert((
            UnitRunes {
                duration_ms,
                ready_in_ms,
            },
            key,
        ));
        if let Some(dirty) = &dirty {
            dirty.mark(entity);
        }
    }
}
