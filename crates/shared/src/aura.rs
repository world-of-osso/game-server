use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::SpellSchool;

/// Effect type carried by an aura. Each aura can have one or more effects.
///
/// Subset of AzerothCore `SpellAuraDefines.h`, covering the combat-relevant
/// effects needed for the Phase 4 aura system.
#[derive(
    Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq,
)]
pub enum AuraEffect {
    /// Increase damage dealt by a flat amount or percent.
    ModDamageDone { percent: f32 },
    /// Increase damage taken by a flat amount or percent.
    ModDamageTaken { percent: f32 },
    /// Absorb damage of a specific school. `remaining` tracks shield HP.
    SchoolAbsorb { school: SpellSchool, remaining: f32 },
    /// Periodic damage (DoT). Ticks every `interval` seconds for `damage` each.
    PeriodicDamage { damage: f32, interval: f32 },
    /// Periodic healing (HoT). Ticks every `interval` seconds for `heal` each.
    PeriodicHeal { heal: f32, interval: f32 },
    /// Modify a primary stat by a flat amount.
    ModStat {
        stamina: f32,
        strength: f32,
        agility: f32,
        intellect: f32,
        spirit: f32,
    },
    /// Modify melee/ranged attack speed (percent, e.g. 0.20 = 20% faster).
    ModAttackSpeed { percent: f32 },
    /// Modify spell cast speed (percent, e.g. 0.15 = 15% faster).
    ModCastSpeed { percent: f32 },
    /// Modify haste rating (flat addition to haste).
    ModHaste { percent: f32 },
    /// Modify crit chance (flat addition, e.g. 0.05 = 5%).
    ModCritChance { percent: f32 },
    /// Modify threat generated (e.g. 0.43 = +43% threat for Righteous Fury).
    /// Tank stances/auras use positive values; threat reduction uses negative.
    ModThreat { percent: f32 },
    /// Reduce cooldown of a specific spell by a flat amount (seconds).
    /// `spell_id` = 0 means all spells. Negative values increase cooldown.
    ModCooldown { spell_id: u32, reduction: f32 },
    /// Modify movement speed. Negative = snare (e.g. -0.50 = 50% slow).
    /// -1.0 = root (complete immobilization).
    ModMovementSpeed { percent: f32 },
}

/// A single buff or debuff active on a unit.
///
/// Auras are stored as a `Vec<Aura>` inside the `Auras` component on each
/// entity. Each aura tracks its source spell, caster, remaining duration,
/// and stack count.
#[derive(
    Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq,
)]
pub struct Aura {
    /// Spell ID that created this aura.
    pub spell_id: u32,
    /// Entity bits of the caster (for same-caster stacking rules).
    pub caster: u64,
    /// Total duration in seconds (for refresh logic).
    pub duration: f32,
    /// Time remaining in seconds. Ticks down each frame.
    pub remaining: f32,
    /// Current stack count.
    pub stacks: u8,
    /// Maximum stacks allowed (1 = non-stacking).
    pub max_stacks: u8,
    /// Up to 3 effects per aura (matches WoW's 3-effect-per-spell limit).
    pub effects: [Option<AuraEffect>; 3],
    /// Time accumulator for periodic effects. Fires a tick when >= `tick_interval`.
    pub tick_timer: f32,
    /// Haste-adjusted tick interval (computed at application, Pandaria+ snapshot).
    pub tick_interval: f32,
    /// Snapshotted caster spell power at application time.
    pub snapshot_spell_power: f32,
    /// Snapshotted caster attack power at application time.
    pub snapshot_attack_power: f32,
    /// Optional proc definition. If set, this aura can trigger procs.
    pub proc_def: Option<ProcDef>,
}

/// Collection of active auras on a unit.
#[derive(
    Component,
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    PartialEq,
    Default,
)]
pub struct Auras {
    pub active: Vec<Aura>,
}

impl Default for Aura {
    fn default() -> Self {
        Self {
            spell_id: 0,
            caster: 0,
            duration: 0.0,
            remaining: 0.0,
            stacks: 1,
            max_stacks: 1,
            effects: [None, None, None],
            tick_timer: 0.0,
            tick_interval: 0.0,
            snapshot_spell_power: 0.0,
            snapshot_attack_power: 0.0,
            proc_def: None,
        }
    }
}

/// A single periodic tick event produced by [`Aura::tick_periodic`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PeriodicTick {
    /// DoT tick: deal this much damage.
    Damage(f32),
    /// HoT tick: heal this much.
    Heal(f32),
}

impl Aura {
    /// Whether this aura has any periodic (DoT/HoT) effect.
    ///
    /// Periodic auras from different casters stack separately per caster,
    /// matching WoW's SPELL_ATTR3_DOT_STACKING_RULE behavior.
    pub fn has_periodic_effect(&self) -> bool {
        self.effects.iter().flatten().any(|e| {
            matches!(
                e,
                AuraEffect::PeriodicDamage { .. } | AuraEffect::PeriodicHeal { .. }
            )
        })
    }

    /// Advance the periodic tick timer by `dt` seconds.
    ///
    /// Returns a `PeriodicTick` each time the timer crosses the tick interval.
    /// Typically fires 0 or 1 times per frame, but can fire multiple if `dt`
    /// is large (e.g. catch-up after lag).
    ///
    /// Damage/heal values use the **snapshotted** stats from application time
    /// (Pandaria+ model: caster stats are locked in when the aura is applied).
    pub fn tick_periodic(&mut self, dt: f32) -> Vec<PeriodicTick> {
        if self.tick_interval <= 0.0 {
            return Vec::new();
        }

        self.tick_timer += dt;
        let mut ticks = Vec::new();

        while self.tick_timer >= self.tick_interval {
            self.tick_timer -= self.tick_interval;
            for effect in self.effects.iter().flatten() {
                match *effect {
                    AuraEffect::PeriodicDamage { damage, .. } => {
                        ticks.push(PeriodicTick::Damage(damage));
                    }
                    AuraEffect::PeriodicHeal { heal, .. } => {
                        ticks.push(PeriodicTick::Heal(heal));
                    }
                    _ => {}
                }
            }
        }

        ticks
    }
}

/// Result of attempting to apply an aura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuraApplyResult {
    /// New aura was added to the active list.
    Applied,
    /// Existing aura from the same caster was refreshed (duration reset, stack incremented).
    Refreshed,
    /// Aura was rejected (non-periodic from a different caster already exists).
    Rejected,
}

impl Auras {
    /// Apply an aura, following WoW stacking rules:
    ///
    /// 1. **Same caster, same spell_id**: Refresh duration, increment stacks (up to max).
    /// 2. **Different caster, periodic (DoT/HoT)**: Stack separately (new entry per caster).
    /// 3. **Different caster, non-periodic**: Reject (keep existing aura).
    ///
    /// Ref: AzerothCore `_TryStackingOrRefreshingExistingAura()`.
    pub fn apply(&mut self, aura: Aura) -> AuraApplyResult {
        let existing = self
            .active
            .iter_mut()
            .find(|a| a.spell_id == aura.spell_id && a.caster == aura.caster);

        if let Some(existing) = existing {
            existing.remaining = aura.duration;
            existing.stacks = existing.stacks.saturating_add(1).min(existing.max_stacks);
            return AuraApplyResult::Refreshed;
        }

        // Different caster (or first application): check if spell_id already present
        let has_from_other_caster = self.active.iter().any(|a| a.spell_id == aura.spell_id);

        if has_from_other_caster && !aura.has_periodic_effect() {
            return AuraApplyResult::Rejected;
        }

        self.active.push(aura);
        AuraApplyResult::Applied
    }

    /// Tick all aura durations by `dt` seconds and remove expired ones.
    ///
    /// Returns the number of auras that expired this tick.
    pub fn tick_and_expire(&mut self, dt: f32) -> usize {
        for aura in &mut self.active {
            aura.remaining -= dt;
        }
        let before = self.active.len();
        self.active.retain(|a| a.remaining > 0.0);
        before - self.active.len()
    }

    /// Remove auras by spell_id (dispel). Removes all instances regardless of caster.
    ///
    /// Returns the number of auras removed.
    pub fn dispel(&mut self, spell_id: u32) -> usize {
        let before = self.active.len();
        self.active.retain(|a| a.spell_id != spell_id);
        before - self.active.len()
    }

    /// Cancel an aura by spell_id from a specific caster (player self-cancel).
    ///
    /// Returns `true` if an aura was removed.
    pub fn cancel(&mut self, spell_id: u32, caster: u64) -> bool {
        let before = self.active.len();
        self.active
            .retain(|a| !(a.spell_id == spell_id && a.caster == caster));
        self.active.len() < before
    }

    /// Tick periodic effects on all active auras.
    ///
    /// Returns `(caster_entity_bits, PeriodicTick)` pairs for each tick fired.
    pub fn tick_all_periodic(&mut self, dt: f32) -> Vec<(u64, PeriodicTick)> {
        self.active
            .iter_mut()
            .flat_map(|aura| {
                let caster = aura.caster;
                aura.tick_periodic(dt)
                    .into_iter()
                    .map(move |tick| (caster, tick))
            })
            .collect()
    }

    /// Absorb incoming damage through active shields before it hits HP.
    ///
    /// Shields are consumed **weakest first** (lowest remaining value) to
    /// maximize the number of shields kept active. Only `SchoolAbsorb` effects
    /// matching the damage school are consumed.
    ///
    /// Returns the damage remaining after all absorbs are applied.
    /// Depleted shields (remaining ≤ 0) are cleaned up from their aura effects.
    /// Auras with no remaining effects are removed entirely.
    pub fn absorb_damage(&mut self, mut damage: f32, school: SpellSchool) -> f32 {
        if damage <= 0.0 {
            return 0.0;
        }

        let absorbs = self.matching_absorbs(school);
        self.consume_absorbs(&absorbs, &mut damage);
        self.remove_depleted_auras();

        damage
    }

    fn matching_absorbs(&self, school: SpellSchool) -> Vec<(usize, usize, f32)> {
        let mut absorbs: Vec<_> = self
            .active
            .iter()
            .enumerate()
            .flat_map(|(aura_idx, aura)| {
                aura.effects
                    .iter()
                    .enumerate()
                    .filter_map(move |(effect_idx, effect)| {
                        let Some(AuraEffect::SchoolAbsorb {
                            school: absorb_school,
                            remaining,
                        }) = effect
                        else {
                            return None;
                        };
                        (*absorb_school == school).then_some((aura_idx, effect_idx, *remaining))
                    })
            })
            .collect();
        absorbs.sort_by(|left, right| left.2.partial_cmp(&right.2).unwrap());
        absorbs
    }

    fn consume_absorbs(&mut self, absorbs: &[(usize, usize, f32)], damage: &mut f32) {
        for &(aura_idx, effect_idx, _) in absorbs {
            if *damage <= 0.0 {
                return;
            }
            self.consume_absorb_effect(aura_idx, effect_idx, damage);
        }
    }

    fn consume_absorb_effect(&mut self, aura_idx: usize, effect_idx: usize, damage: &mut f32) {
        let effect = &mut self.active[aura_idx].effects[effect_idx];
        let Some(AuraEffect::SchoolAbsorb { remaining, .. }) = effect else {
            return;
        };

        let absorbed = damage.min(*remaining);
        *remaining -= absorbed;
        *damage -= absorbed;
        if *remaining <= 0.0 {
            self.active[aura_idx].effects[effect_idx] = None;
        }
    }

    fn remove_depleted_auras(&mut self) {
        self.active.retain(has_active_effects);
    }

    /// Multiplicative modifier from all auras matching `extract`.
    ///
    /// Each matching effect contributes `(1 + percent)` as a factor, stacking
    /// multiplicatively (e.g. two +10% → 1.1 × 1.1 = 1.21×).
    fn multiplicative_modifier(&self, extract: fn(&AuraEffect) -> Option<f32>) -> f32 {
        self.active
            .iter()
            .flat_map(|a| a.effects.iter().flatten())
            .fold(1.0, |acc, effect| match extract(effect) {
                Some(percent) => acc * (1.0 + percent),
                None => acc,
            })
    }

    /// Multiplicative damage-done modifier from all active `ModDamageDone` auras.
    pub fn damage_done_multiplier(&self) -> f32 {
        self.multiplicative_modifier(|e| match e {
            AuraEffect::ModDamageDone { percent } => Some(*percent),
            _ => None,
        })
    }

    /// Multiplicative damage-taken modifier from all active `ModDamageTaken` auras.
    pub fn damage_taken_multiplier(&self) -> f32 {
        self.multiplicative_modifier(|e| match e {
            AuraEffect::ModDamageTaken { percent } => Some(*percent),
            _ => None,
        })
    }

    /// Multiplicative threat modifier from all active `ModThreat` auras.
    pub fn threat_multiplier(&self) -> f32 {
        self.multiplicative_modifier(|e| match e {
            AuraEffect::ModThreat { percent } => Some(*percent),
            _ => None,
        })
    }

    /// Compute the effective cooldown for a spell after CDR auras/talents.
    ///
    /// Sums all `ModCooldown` reductions that match the spell_id (or spell_id=0
    /// for global CDR). Result is clamped to a minimum of 0.
    pub fn effective_cooldown(&self, spell_id: u32, base_cooldown: f32) -> f32 {
        let total_reduction: f32 = self
            .active
            .iter()
            .flat_map(|a| a.effects.iter().flatten())
            .filter_map(|e| match e {
                AuraEffect::ModCooldown {
                    spell_id: sid,
                    reduction,
                } if *sid == spell_id || *sid == 0 => Some(reduction),
                _ => None,
            })
            .sum();
        (base_cooldown - total_reduction).max(0.0)
    }

    /// Compute effective movement speed multiplier from all active auras.
    ///
    /// Snares (negative) use the strongest single snare (not multiplicative),
    /// matching WoW behavior. Speed buffs (positive) stack additively.
    /// A root (`percent = -1.0`) overrides everything → returns 0.0.
    ///
    /// Returns a multiplier (1.0 = normal, 0.5 = 50% speed, 0.0 = rooted).
    pub fn movement_speed_multiplier(&self) -> f32 {
        let mut strongest_snare: f32 = 0.0;
        let mut total_buff: f32 = 0.0;
        let mut rooted = false;

        for effect in self.active.iter().flat_map(|a| a.effects.iter().flatten()) {
            if let AuraEffect::ModMovementSpeed { percent } = effect {
                if *percent <= -1.0 {
                    rooted = true;
                } else if *percent < 0.0 {
                    strongest_snare = strongest_snare.min(*percent);
                } else {
                    total_buff += percent;
                }
            }
        }

        if rooted {
            return 0.0;
        }

        // Minimum 10% speed for snares (only roots reach 0)
        (1.0 + strongest_snare + total_buff).clamp(0.1, f32::MAX)
    }

    /// Whether the entity is rooted (cannot move at all).
    pub fn is_rooted(&self) -> bool {
        self.movement_speed_multiplier() == 0.0
    }

    /// Remove all auras (on death). Clears the entire active list.
    pub fn clear_all(&mut self) {
        self.active.clear();
    }
}

fn has_active_effects(aura: &Aura) -> bool {
    aura.effects.iter().any(|effect| effect.is_some())
}

// --- Proc system ---

/// Combat events that can trigger a proc.
///
/// Ref: AzerothCore `PROC_FLAG_*` in `SpellMgr.h`.
#[derive(
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub enum ProcTrigger {
    /// Any successful melee or spell hit.
    OnHit,
    /// A critical strike (melee or spell).
    OnCrit,
    /// Healing done (direct or periodic).
    OnHeal,
    /// Damage taken (melee or spell).
    OnDamageTaken,
}

/// What happens when a proc fires.
#[derive(
    Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq,
)]
pub enum ProcAction {
    /// Apply a spell (buff, damage, etc.).
    ApplySpell { spell_id: u32 },
    /// Reset the cooldown of a specific spell.
    ResetCooldown { spell_id: u32 },
}

/// A proc definition attached to an aura.
///
/// When the trigger event fires, the proc rolls against `chance` (0.0–1.0).
/// If successful and ICD has elapsed, it produces a `ProcResult`.
///
/// Ref: AzerothCore `SpellProcEntry`, `HandleProc()`.
#[derive(
    Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq,
)]
pub struct ProcDef {
    /// Which event can trigger this proc.
    pub trigger: ProcTrigger,
    /// Probability of firing (0.0–1.0). 1.0 = always.
    pub chance: f32,
    /// Internal cooldown in seconds. 0.0 = no ICD.
    pub icd: f32,
    /// Time remaining on the internal cooldown (ticks down each frame).
    pub icd_remaining: f32,
    /// What happens when the proc fires.
    pub action: ProcAction,
}

/// A proc that fired, ready for the server to act on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcResult {
    /// The aura's spell_id that caused the proc.
    pub source_spell_id: u32,
    /// The caster entity bits of the source aura.
    pub caster: u64,
    /// What the server should do.
    pub action: ProcAction,
}

impl Auras {
    /// Tick ICD timers on all auras by `dt` seconds.
    pub fn tick_proc_cooldowns(&mut self, dt: f32) {
        for aura in &mut self.active {
            if let Some(ref mut proc_def) = aura.proc_def {
                proc_def.icd_remaining = (proc_def.icd_remaining - dt).max(0.0);
            }
        }
    }

    /// Check all auras for procs matching the given event.
    ///
    /// `roll` is a random value in 0.0–1.0 (caller provides RNG).
    /// Returns procs that fired. Each fired proc starts its ICD.
    pub fn check_procs(&mut self, event: ProcTrigger, roll: f32) -> Vec<ProcResult> {
        let mut results = Vec::new();

        for aura in &mut self.active {
            let Some(ref mut proc_def) = aura.proc_def else {
                continue;
            };
            if proc_def.trigger != event {
                continue;
            }
            if proc_def.icd_remaining > 0.0 {
                continue;
            }
            if roll > proc_def.chance {
                continue;
            }

            results.push(ProcResult {
                source_spell_id: aura.spell_id,
                caster: aura.caster,
                action: proc_def.action,
            });
            proc_def.icd_remaining = proc_def.icd;
        }

        results
    }
}

#[cfg(test)]
#[path = "aura_tests.rs"]
mod tests;
