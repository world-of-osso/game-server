use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::SpellSchool;

// --- Cast bar ---

/// Whether a cast is a normal cast or a channeled spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CastType {
    /// Normal cast: bar fills up, spell fires on completion.
    Normal,
    /// Channel: spell fires immediately, effects tick during the channel.
    Channel,
}

/// Active spell cast on an entity (cast bar / channel in progress).
///
/// Added as a component when a cast begins, removed on completion,
/// interruption, or cancellation.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct CastState {
    /// Spell being cast.
    pub spell_id: u32,
    /// Target entity bits (0 for self-cast).
    pub target: u64,
    /// Total cast/channel duration in seconds (after haste).
    pub duration: f32,
    /// Time elapsed so far.
    pub elapsed: f32,
    /// Normal or channeled cast.
    pub cast_type: CastType,
    /// Whether the cast can be interrupted by damage.
    pub interruptible: bool,
    /// Number of pushbacks applied (max 2 for normal casts).
    pub pushback_count: u8,
    /// For channels: tick interval in seconds.
    pub channel_tick_interval: f32,
    /// For channels: time accumulator for ticks.
    pub channel_tick_timer: f32,
    /// Whether this spell allows movement while casting.
    pub cast_while_moving: bool,
}

/// Result of ticking a cast bar by `dt` seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CastTickResult {
    /// Cast still in progress, no action needed.
    InProgress,
    /// Normal cast completed — apply spell effects.
    Completed,
    /// Channel tick fired — apply one tick of channel effects.
    ChannelTick,
    /// Channel finished (elapsed >= duration).
    ChannelComplete,
}

impl CastState {
    /// Create a new normal (non-channel) cast.
    pub fn normal(spell_id: u32, target: u64, duration: f32, interruptible: bool) -> Self {
        Self {
            spell_id,
            target,
            duration,
            elapsed: 0.0,
            cast_type: CastType::Normal,
            interruptible,
            pushback_count: 0,
            channel_tick_interval: 0.0,
            channel_tick_timer: 0.0,
            cast_while_moving: false,
        }
    }

    /// Create a new channeled cast.
    pub fn channel(
        spell_id: u32,
        target: u64,
        duration: f32,
        tick_interval: f32,
        interruptible: bool,
    ) -> Self {
        Self {
            spell_id,
            target,
            duration,
            elapsed: 0.0,
            cast_type: CastType::Channel,
            interruptible,
            pushback_count: 0,
            channel_tick_interval: tick_interval,
            channel_tick_timer: 0.0,
            cast_while_moving: false,
        }
    }

    /// Advance the cast by `dt` seconds. Returns what happened.
    pub fn tick(&mut self, dt: f32) -> CastTickResult {
        self.elapsed += dt;

        match self.cast_type {
            CastType::Normal => {
                if self.elapsed >= self.duration {
                    CastTickResult::Completed
                } else {
                    CastTickResult::InProgress
                }
            }
            CastType::Channel => {
                if self.elapsed >= self.duration {
                    return CastTickResult::ChannelComplete;
                }
                if self.channel_tick_interval > 0.0 {
                    self.channel_tick_timer += dt;
                    if self.channel_tick_timer >= self.channel_tick_interval {
                        self.channel_tick_timer -= self.channel_tick_interval;
                        return CastTickResult::ChannelTick;
                    }
                }
                CastTickResult::InProgress
            }
        }
    }

    /// Progress as a fraction 0.0–1.0.
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        (self.elapsed / self.duration).min(1.0)
    }

    /// Apply spell pushback from taking damage during a cast.
    ///
    /// - **Normal cast**: extends duration by 0.5s (max 2 pushbacks).
    /// - **Channel**: reduces remaining duration by 25% of total (max 2).
    /// - Non-interruptible casts ignore pushback entirely.
    ///
    /// Returns `true` if pushback was applied.
    ///
    /// Ref: AzerothCore `Spell::DelayedChannel()`, retail pushback rules.
    pub fn apply_pushback(&mut self) -> bool {
        if !self.interruptible || self.pushback_count >= MAX_PUSHBACKS {
            return false;
        }
        self.pushback_count += 1;
        match self.cast_type {
            CastType::Normal => {
                self.duration += PUSHBACK_DELAY;
            }
            CastType::Channel => {
                let reduction = self.duration * CHANNEL_PUSHBACK_FRACTION;
                self.duration = (self.duration - reduction).max(self.elapsed);
            }
        }
        true
    }
}

/// Maximum pushbacks per cast before the caster becomes immune.
const MAX_PUSHBACKS: u8 = 2;
/// Normal cast pushback: extends cast time by this many seconds.
const PUSHBACK_DELAY: f32 = 0.5;
/// Channel pushback: reduces remaining channel by this fraction of total.
const CHANNEL_PUSHBACK_FRACTION: f32 = 0.25;

// --- Global cooldown ---

/// Tracks the global cooldown (GCD) for an entity.
///
/// The GCD prevents casting any spell until it expires. Instant casts trigger
/// the GCD but bypass the cast bar. The GCD duration is haste-scaled
/// (base 1.5s, min 0.75s) — computed externally via `formulas::spell::hasted_gcd()`.
///
/// Ref: AzerothCore `Spell::TriggerGlobalCooldown()`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct GlobalCooldown {
    /// Time remaining on the GCD (seconds).
    pub remaining: f32,
}

impl Default for GlobalCooldown {
    fn default() -> Self {
        Self { remaining: 0.0 }
    }
}

impl GlobalCooldown {
    /// Start the GCD with a given duration.
    pub fn trigger(&mut self, duration: f32) {
        self.remaining = duration;
    }

    /// Whether the GCD is active (can't cast).
    pub fn is_active(&self) -> bool {
        self.remaining > 0.0
    }

    /// Tick the GCD timer. Returns `true` if GCD just expired this tick.
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.remaining <= 0.0 {
            return false;
        }
        self.remaining = (self.remaining - dt).max(0.0);
        self.remaining <= 0.0
    }
}

// --- School lockout ---

/// Tracks spell school lockouts from interrupts.
///
/// When a cast is interrupted, the spell's school is locked out for a duration.
/// The caster cannot cast any spell of that school until the lockout expires.
///
/// Ref: AzerothCore `Spell::TriggerGlobalCooldown()` / interrupt handling.
#[derive(Component, Debug, Clone, Default)]
pub struct SchoolLockouts {
    entries: Vec<SchoolLockoutEntry>,
}

#[derive(Debug, Clone, Copy)]
struct SchoolLockoutEntry {
    school: SpellSchool,
    remaining: f32,
}

impl SchoolLockouts {
    /// Apply a school lockout from an interrupt.
    pub fn lock(&mut self, school: SpellSchool, duration: f32) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.school == school) {
            // Refresh if new lockout is longer
            if duration > entry.remaining {
                entry.remaining = duration;
            }
        } else {
            self.entries.push(SchoolLockoutEntry {
                school,
                remaining: duration,
            });
        }
    }

    /// Check if a school is currently locked out.
    pub fn is_locked(&self, school: SpellSchool) -> bool {
        self.entries
            .iter()
            .any(|e| e.school == school && e.remaining > 0.0)
    }

    /// Tick all lockout timers and remove expired ones.
    pub fn tick(&mut self, dt: f32) {
        for entry in &mut self.entries {
            entry.remaining -= dt;
        }
        self.entries.retain(|e| e.remaining > 0.0);
    }
}

// --- Spell cooldowns ---

/// Tracks per-spell cooldown timers on an entity, with shared category support.
///
/// When a spell is cast, its cooldown (from `SpellData::cooldown`) is started.
/// Spells with the same `category` share a cooldown — casting one puts all
/// spells in that category on cooldown.
///
/// Category 0 means "no shared category" (spell has its own independent CD).
///
/// Ref: AzerothCore `SpellHistory.cpp`, `spell_category` DBC table.
#[derive(Component, Debug, Clone, Default)]
pub struct SpellCooldowns {
    entries: Vec<CooldownEntry>,
}

#[derive(Debug, Clone, Copy)]
struct CooldownEntry {
    spell_id: u32,
    /// Shared cooldown category (0 = independent, no shared CD).
    category: u32,
    remaining: f32,
}

impl SpellCooldowns {
    /// Start a cooldown for a spell (independent, no shared category).
    pub fn start(&mut self, spell_id: u32, duration: f32) {
        self.start_with_category(spell_id, 0, duration);
    }

    /// Start a cooldown for a spell with a shared category.
    ///
    /// All spells in the same non-zero category are put on cooldown.
    /// If a spell is already tracked, its timer is updated.
    pub fn start_with_category(&mut self, spell_id: u32, category: u32, duration: f32) {
        if duration <= 0.0 {
            return;
        }
        // Update or insert the triggering spell
        if let Some(entry) = self.entries.iter_mut().find(|e| e.spell_id == spell_id) {
            entry.remaining = duration;
            entry.category = category;
        } else {
            self.entries.push(CooldownEntry {
                spell_id,
                category,
                remaining: duration,
            });
        }
        // Put all other spells in the same category on cooldown
        if category != 0 {
            for entry in &mut self.entries {
                if entry.category == category && entry.spell_id != spell_id {
                    entry.remaining = entry.remaining.max(duration);
                }
            }
        }
    }

    /// Register a spell in a shared category without starting a cooldown.
    /// This allows `start_with_category` on another spell to affect this one.
    pub fn register_category(&mut self, spell_id: u32, category: u32) {
        if self.entries.iter().any(|e| e.spell_id == spell_id) {
            return;
        }
        self.entries.push(CooldownEntry {
            spell_id,
            category,
            remaining: 0.0,
        });
    }

    /// Get remaining cooldown for a spell (0.0 if ready).
    pub fn remaining(&self, spell_id: u32) -> f32 {
        self.entries
            .iter()
            .find(|e| e.spell_id == spell_id)
            .map_or(0.0, |e| e.remaining.max(0.0))
    }

    /// Whether a spell is on cooldown.
    pub fn is_on_cooldown(&self, spell_id: u32) -> bool {
        self.remaining(spell_id) > 0.0
    }

    /// Tick all cooldown timers and remove expired independent entries.
    /// Category entries with 0 remaining are kept (for future shared triggers).
    pub fn tick(&mut self, dt: f32) {
        for entry in &mut self.entries {
            entry.remaining -= dt;
        }
        // Remove expired entries that have no category (independent CDs)
        self.entries
            .retain(|e| e.remaining > 0.0 || e.category != 0);
    }

    /// Reset a specific spell's cooldown (e.g. from a proc or talent).
    pub fn reset(&mut self, spell_id: u32) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.spell_id == spell_id) {
            entry.remaining = 0.0;
        }
    }

    /// Clear all cooldowns (e.g. on arena start or GM command).
    pub fn clear_all(&mut self) {
        for entry in &mut self.entries {
            entry.remaining = 0.0;
        }
    }
}

// --- Charge-based abilities ---

/// A charge-based ability with individual recharge timers.
///
/// Some spells have 2+ charges (e.g. Roll has 2, Fire Blast has 3 with talent).
/// Using the ability consumes a charge. Charges recharge individually on a timer.
/// The ability is usable as long as `charges > 0`.
///
/// Ref: AzerothCore `SpellHistory::HandleCooldowns()` charge handling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChargeEntry {
    pub spell_id: u32,
    /// Current available charges.
    pub charges: u8,
    /// Maximum charges.
    pub max_charges: u8,
    /// Recharge time per charge in seconds.
    pub recharge_time: f32,
    /// Time accumulated toward the next charge (ticks up to recharge_time).
    pub recharge_timer: f32,
}

impl ChargeEntry {
    pub fn new(spell_id: u32, max_charges: u8, recharge_time: f32) -> Self {
        Self {
            spell_id,
            charges: max_charges,
            max_charges,
            recharge_time,
            recharge_timer: 0.0,
        }
    }

    /// Use one charge. Returns `true` if a charge was available.
    pub fn use_charge(&mut self) -> bool {
        if self.charges == 0 {
            return false;
        }
        self.charges -= 1;
        // Start recharging if we just went below max
        if self.charges < self.max_charges && self.recharge_timer == 0.0 {
            self.recharge_timer = 0.0; // timer starts from 0
        }
        true
    }

    /// Tick the recharge timer. Restores charges as timers complete.
    pub fn tick(&mut self, dt: f32) {
        if self.charges >= self.max_charges {
            self.recharge_timer = 0.0;
            return;
        }
        self.recharge_timer += dt;
        while self.recharge_timer >= self.recharge_time && self.charges < self.max_charges {
            self.recharge_timer -= self.recharge_time;
            self.charges += 1;
        }
        // If fully recharged, reset timer
        if self.charges >= self.max_charges {
            self.recharge_timer = 0.0;
        }
    }

    /// Whether the ability can be used (has charges available).
    pub fn available(&self) -> bool {
        self.charges > 0
    }
}

/// Component tracking all charge-based abilities on an entity.
#[derive(Component, Debug, Clone, Default)]
pub struct SpellCharges {
    entries: Vec<ChargeEntry>,
}

impl SpellCharges {
    /// Register a charge-based ability.
    pub fn add(&mut self, entry: ChargeEntry) {
        self.entries.push(entry);
    }

    /// Get a charge entry by spell ID.
    pub fn get(&self, spell_id: u32) -> Option<&ChargeEntry> {
        self.entries.iter().find(|e| e.spell_id == spell_id)
    }

    /// Use a charge of a spell. Returns `true` if successful.
    pub fn use_charge(&mut self, spell_id: u32) -> bool {
        self.entries
            .iter_mut()
            .find(|e| e.spell_id == spell_id)
            .is_some_and(|e| e.use_charge())
    }

    /// Tick all recharge timers.
    pub fn tick(&mut self, dt: f32) {
        for entry in &mut self.entries {
            entry.tick(dt);
        }
    }
}

// --- Spell batching ---

/// A queued spell cast awaiting batch processing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingSpellCast {
    /// Caster entity bits.
    pub caster: u64,
    /// Target entity bits (0 for self-cast).
    pub target: u64,
    /// Spell ID to cast.
    pub spell_id: u32,
}

/// Collects spell casts within a server tick for simultaneous resolution.
///
/// Spells queued during the same tick are processed together, allowing
/// simultaneous events (e.g. two players killing each other). The server
/// runs at a fixed tick rate (20Hz / 50ms by default via Bevy FixedUpdate),
/// which acts as the natural batch window.
///
/// Ref: WoW spell batching — retail uses ~10ms batches, Classic used ~400ms.
/// Our 50ms tick provides a middle ground.
#[derive(Resource, Debug, Clone, Default)]
pub struct SpellBatch {
    pending: Vec<PendingSpellCast>,
}

impl SpellBatch {
    /// Queue a spell cast for batch processing this tick.
    pub fn queue(&mut self, cast: PendingSpellCast) {
        self.pending.push(cast);
    }

    /// Drain all pending casts for processing. Returns them in queue order.
    pub fn drain(&mut self) -> Vec<PendingSpellCast> {
        std::mem::take(&mut self.pending)
    }

    /// Number of pending casts.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

// --- Spell projectiles ---

/// Default projectile speed in yards per second (matches WoW's common spell speed).
const DEFAULT_PROJECTILE_SPEED: f32 = 24.0;

/// An in-flight spell projectile traveling toward a target.
///
/// Created when a spell with travel time finishes casting. The projectile
/// travels at a fixed speed; effects apply on impact (remaining <= 0).
///
/// Ref: AzerothCore `Spell::m_spellSpeed`, projectile travel in `Spell::Update()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellProjectile {
    /// The spell that was cast.
    pub spell_id: u32,
    /// Caster entity bits (for effect attribution).
    pub caster: u64,
    /// Target entity bits.
    pub target: u64,
    /// Time remaining until impact (seconds).
    pub remaining: f32,
}

impl SpellProjectile {
    /// Create a projectile from distance and speed.
    ///
    /// `distance` is caster-to-target in yards. `speed` is yards/sec
    /// (0.0 uses the default 24 y/s).
    pub fn new(spell_id: u32, caster: u64, target: u64, distance: f32, speed: f32) -> Self {
        let effective_speed = if speed > 0.0 {
            speed
        } else {
            DEFAULT_PROJECTILE_SPEED
        };
        Self {
            spell_id,
            caster,
            target,
            remaining: distance / effective_speed,
        }
    }

    /// Tick the projectile by `dt` seconds. Returns `true` if it has impacted.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.remaining -= dt;
        self.remaining <= 0.0
    }
}

/// Collection of in-flight projectiles on the server.
///
/// Each frame, tick all projectiles and collect impacts for effect application.
#[derive(Component, Debug, Clone, Default)]
pub struct SpellProjectiles {
    pub active: Vec<SpellProjectile>,
}

impl SpellProjectiles {
    /// Add a new in-flight projectile.
    pub fn launch(&mut self, projectile: SpellProjectile) {
        self.active.push(projectile);
    }

    /// Tick all projectiles, returning those that impacted this frame.
    /// Impacted projectiles are removed from the active list.
    pub fn tick_and_collect_impacts(&mut self, dt: f32) -> Vec<SpellProjectile> {
        for proj in &mut self.active {
            proj.remaining -= dt;
        }
        let (impacted, still_flying): (Vec<_>, Vec<_>) =
            self.active.drain(..).partition(|p| p.remaining <= 0.0);
        self.active = still_flying;
        impacted
    }
}

#[cfg(test)]
#[path = "casting_tests.rs"]
mod tests;
