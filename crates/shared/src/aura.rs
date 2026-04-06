use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::SpellSchool;

/// Effect type carried by an aura. Each aura can have one or more effects.
///
/// Subset of AzerothCore `SpellAuraDefines.h`, covering the combat-relevant
/// effects needed for the Phase 4 aura system.
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
    ModStat { stamina: f32, strength: f32, agility: f32, intellect: f32, spirit: f32 },
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
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
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
        let has_from_other_caster = self
            .active
            .iter()
            .any(|a| a.spell_id == aura.spell_id);

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

        // Collect indices of matching absorb effects, sorted weakest first
        let mut absorbs: Vec<(usize, usize, f32)> = self
            .active
            .iter()
            .enumerate()
            .flat_map(|(ai, aura)| {
                aura.effects.iter().enumerate().filter_map(move |(ei, eff)| {
                    if let Some(AuraEffect::SchoolAbsorb {
                        school: s,
                        remaining,
                    }) = eff
                        && *s == school
                    {
                        return Some((ai, ei, *remaining));
                    }
                    None
                })
            })
            .collect();

        absorbs.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

        for (aura_idx, effect_idx, _) in absorbs {
            if damage <= 0.0 {
                break;
            }
            let effect = &mut self.active[aura_idx].effects[effect_idx];
            if let Some(AuraEffect::SchoolAbsorb { remaining, .. }) = effect {
                let absorbed = damage.min(*remaining);
                *remaining -= absorbed;
                damage -= absorbed;
                if *remaining <= 0.0 {
                    self.active[aura_idx].effects[effect_idx] = None;
                }
            }
        }

        // Remove auras whose only effects were absorbs that are now depleted.
        // An aura is removed if it had absorb effects and now has no effects left.
        self.active.retain(|a| a.effects.iter().any(|e| e.is_some()));

        damage
    }

    /// Multiplicative damage-done modifier from all active `ModDamageDone` auras.
    ///
    /// Each aura contributes `(1 + percent)` as a factor. Multiple auras stack
    /// multiplicatively: two +10% buffs → 1.1 × 1.1 = 1.21×.
    ///
    /// Returns a multiplier to apply to outgoing damage (1.0 = no change).
    pub fn damage_done_multiplier(&self) -> f32 {
        self.active
            .iter()
            .flat_map(|a| a.effects.iter().flatten())
            .fold(1.0, |acc, effect| match effect {
                AuraEffect::ModDamageDone { percent } => acc * (1.0 + percent),
                _ => acc,
            })
    }

    /// Multiplicative damage-taken modifier from all active `ModDamageTaken` auras.
    ///
    /// Each aura contributes `(1 + percent)` as a factor. A +20% damage taken
    /// debuff uses `percent = 0.20`, a -10% reduction uses `percent = -0.10`.
    ///
    /// Returns a multiplier to apply to incoming damage (1.0 = no change).
    pub fn damage_taken_multiplier(&self) -> f32 {
        self.active
            .iter()
            .flat_map(|a| a.effects.iter().flatten())
            .fold(1.0, |acc, effect| match effect {
                AuraEffect::ModDamageTaken { percent } => acc * (1.0 + percent),
                _ => acc,
            })
    }

    /// Multiplicative threat modifier from all active `ModThreat` auras.
    ///
    /// Tank stances (e.g. Righteous Fury +43%, Defensive Stance +30%) stack
    /// multiplicatively. Returns a multiplier (1.0 = no change).
    pub fn threat_multiplier(&self) -> f32 {
        self.active
            .iter()
            .flat_map(|a| a.effects.iter().flatten())
            .fold(1.0, |acc, effect| match effect {
                AuraEffect::ModThreat { percent } => acc * (1.0 + percent),
                _ => acc,
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

        (1.0 + strongest_snare + total_buff).max(0.0)
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
    Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode,
    Debug, Clone, Copy, PartialEq,
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
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
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
mod tests {
    use super::*;

    fn make_aura(spell_id: u32, caster: u64, max_stacks: u8) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 10.0,
            remaining: 10.0,
            max_stacks,
            ..Default::default()
        }
    }

    fn make_dot(spell_id: u32, caster: u64) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 15.0,
            remaining: 15.0,
            effects: [
                Some(AuraEffect::PeriodicDamage {
                    damage: 100.0,
                    interval: 3.0,
                }),
                None,
                None,
            ],
            tick_interval: 3.0,
            ..Default::default()
        }
    }

    fn make_shield(spell_id: u32, caster: u64, school: SpellSchool, absorb: f32) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 30.0,
            remaining: 30.0,
            effects: [
                Some(AuraEffect::SchoolAbsorb { school, remaining: absorb }),
                None,
                None,
            ],
            ..Default::default()
        }
    }

    fn make_damage_done_buff(spell_id: u32, caster: u64, percent: f32) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 30.0,
            remaining: 30.0,
            effects: [
                Some(AuraEffect::ModDamageDone { percent }),
                None,
                None,
            ],
            ..Default::default()
        }
    }

    fn make_damage_taken_debuff(spell_id: u32, caster: u64, percent: f32) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 30.0,
            remaining: 30.0,
            effects: [
                Some(AuraEffect::ModDamageTaken { percent }),
                None,
                None,
            ],
            ..Default::default()
        }
    }

    fn make_proc_aura(spell_id: u32, caster: u64, trigger: ProcTrigger, chance: f32, icd: f32, triggered: u32) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 60.0,
            remaining: 60.0,
            effects: [
                Some(AuraEffect::ModCritChance { percent: 0.05 }),
                None,
                None,
            ],
            proc_def: Some(ProcDef {
                trigger,
                chance,
                icd,
                icd_remaining: 0.0,
                action: ProcAction::ApplySpell { spell_id: triggered },
            }),
            ..Default::default()
        }
    }

    fn make_threat_aura(spell_id: u32, caster: u64, percent: f32) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 60.0,
            remaining: 60.0,
            effects: [
                Some(AuraEffect::ModThreat { percent }),
                None,
                None,
            ],
            ..Default::default()
        }
    }

    fn make_cdr_aura(spell_id: u32, caster: u64, target_spell: u32, reduction: f32) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 60.0,
            remaining: 60.0,
            effects: [
                Some(AuraEffect::ModCooldown {
                    spell_id: target_spell,
                    reduction,
                }),
                None,
                None,
            ],
            ..Default::default()
        }
    }

    fn make_speed_aura(spell_id: u32, caster: u64, percent: f32) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 10.0,
            remaining: 10.0,
            effects: [
                Some(AuraEffect::ModMovementSpeed { percent }),
                None,
                None,
            ],
            ..Default::default()
        }
    }

    fn make_dot_with_interval(spell_id: u32, caster: u64, damage: f32, interval: f32) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 15.0,
            remaining: 15.0,
            effects: [
                Some(AuraEffect::PeriodicDamage { damage, interval }),
                None,
                None,
            ],
            tick_interval: interval,
            ..Default::default()
        }
    }

    fn make_hot(spell_id: u32, caster: u64, heal: f32, interval: f32) -> Aura {
        Aura {
            spell_id,
            caster,
            duration: 12.0,
            remaining: 12.0,
            effects: [
                Some(AuraEffect::PeriodicHeal { heal, interval }),
                None,
                None,
            ],
            tick_interval: interval,
            ..Default::default()
        }
    }

    #[test]
    fn aura_roundtrip_bitcode() {
        let aura = Aura {
            spell_id: 12345,
            caster: 99,
            duration: 30.0,
            remaining: 25.5,
            stacks: 2,
            max_stacks: 5,
            effects: [
                Some(AuraEffect::ModDamageDone { percent: 0.10 }),
                Some(AuraEffect::ModCritChance { percent: 0.05 }),
                None,
            ],
            ..Default::default()
        };
        let encoded = bitcode::encode(&aura);
        let decoded: Aura = bitcode::decode(&encoded).unwrap();
        assert_eq!(aura, decoded);
    }

    #[test]
    fn auras_default_is_empty() {
        let auras = Auras::default();
        assert!(auras.active.is_empty());
    }

    #[test]
    fn auras_collection_roundtrip_bitcode() {
        let auras = Auras {
            active: vec![
                Aura {
                    spell_id: 1,
                    caster: 10,
                    duration: 10.0,
                    remaining: 8.0,
                    effects: [
                        Some(AuraEffect::PeriodicDamage {
                            damage: 500.0,
                            interval: 3.0,
                        }),
                        None,
                        None,
                    ],
                    tick_interval: 3.0,
                    ..Default::default()
                },
                Aura {
                    spell_id: 2,
                    caster: 20,
                    duration: 60.0,
                    remaining: 45.0,
                    stacks: 3,
                    max_stacks: 5,
                    effects: [
                        Some(AuraEffect::SchoolAbsorb {
                            school: SpellSchool::Holy,
                            remaining: 10000.0,
                        }),
                        None,
                        None,
                    ],
                    ..Default::default()
                },
            ],
        };
        let encoded = bitcode::encode(&auras);
        let decoded: Auras = bitcode::decode(&encoded).unwrap();
        assert_eq!(auras, decoded);
    }

    #[test]
    fn aura_effect_variants() {
        // Verify all variants can be constructed
        let effects: Vec<AuraEffect> = vec![
            AuraEffect::ModDamageDone { percent: 0.1 },
            AuraEffect::ModDamageTaken { percent: -0.2 },
            AuraEffect::SchoolAbsorb { school: SpellSchool::Fire, remaining: 5000.0 },
            AuraEffect::PeriodicDamage { damage: 100.0, interval: 3.0 },
            AuraEffect::PeriodicHeal { heal: 200.0, interval: 2.0 },
            AuraEffect::ModStat { stamina: 50.0, strength: 0.0, agility: 0.0, intellect: 0.0, spirit: 0.0 },
            AuraEffect::ModAttackSpeed { percent: 0.2 },
            AuraEffect::ModCastSpeed { percent: 0.15 },
            AuraEffect::ModHaste { percent: 0.1 },
            AuraEffect::ModCritChance { percent: 0.05 },
            AuraEffect::ModThreat { percent: 0.43 },
            AuraEffect::ModCooldown { spell_id: 100, reduction: 2.0 },
            AuraEffect::ModMovementSpeed { percent: -0.50 },
        ];
        assert_eq!(effects.len(), 13);
    }

    #[test]
    fn aura_has_periodic_effect_dot() {
        let dot = make_dot(1, 10);
        assert!(dot.has_periodic_effect());
    }

    #[test]
    fn aura_has_periodic_effect_buff() {
        let buff = make_aura(1, 10, 1);
        assert!(!buff.has_periodic_effect());
    }

    #[test]
    fn apply_aura_first_time() {
        let mut auras = Auras::default();
        let result = auras.apply(make_aura(100, 1, 3));
        assert_eq!(result, AuraApplyResult::Applied);
        assert_eq!(auras.active.len(), 1);
    }

    #[test]
    fn apply_aura_same_caster_refreshes_duration() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 3));
        // Simulate time passing
        auras.active[0].remaining = 3.0;

        let result = auras.apply(make_aura(100, 1, 3));
        assert_eq!(result, AuraApplyResult::Refreshed);
        assert_eq!(auras.active.len(), 1);
        assert_eq!(auras.active[0].remaining, 10.0, "duration should reset");
    }

    #[test]
    fn apply_aura_same_caster_increments_stacks() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 5));
        assert_eq!(auras.active[0].stacks, 1);

        auras.apply(make_aura(100, 1, 5));
        assert_eq!(auras.active[0].stacks, 2);

        auras.apply(make_aura(100, 1, 5));
        assert_eq!(auras.active[0].stacks, 3);
    }

    #[test]
    fn apply_aura_same_caster_caps_at_max_stacks() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 2));
        auras.apply(make_aura(100, 1, 2));
        auras.apply(make_aura(100, 1, 2));
        assert_eq!(auras.active[0].stacks, 2, "should not exceed max_stacks");
    }

    #[test]
    fn apply_aura_different_caster_non_periodic_rejected() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 1));
        let result = auras.apply(make_aura(100, 2, 1));
        assert_eq!(result, AuraApplyResult::Rejected);
        assert_eq!(auras.active.len(), 1);
        assert_eq!(auras.active[0].caster, 1, "original caster kept");
    }

    #[test]
    fn apply_dot_different_casters_stack() {
        let mut auras = Auras::default();
        auras.apply(make_dot(200, 1));
        let result = auras.apply(make_dot(200, 2));
        assert_eq!(result, AuraApplyResult::Applied);
        assert_eq!(auras.active.len(), 2, "DOTs from different casters stack");
    }

    #[test]
    fn apply_dot_same_caster_refreshes() {
        let mut auras = Auras::default();
        auras.apply(make_dot(200, 1));
        auras.active[0].remaining = 5.0;

        let result = auras.apply(make_dot(200, 1));
        assert_eq!(result, AuraApplyResult::Refreshed);
        assert_eq!(auras.active.len(), 1);
        assert_eq!(auras.active[0].remaining, 15.0);
    }

    #[test]
    fn apply_different_spells_always_stack() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 1));
        let result = auras.apply(make_aura(200, 1, 1));
        assert_eq!(result, AuraApplyResult::Applied);
        assert_eq!(auras.active.len(), 2);
    }

    #[test]
    fn tick_expires_aura_at_zero() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 1)); // 10s duration
        let expired = auras.tick_and_expire(10.1);
        assert_eq!(expired, 1);
        assert!(auras.active.is_empty());
    }

    #[test]
    fn tick_does_not_expire_with_time_left() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 1));
        let expired = auras.tick_and_expire(5.0);
        assert_eq!(expired, 0);
        assert_eq!(auras.active.len(), 1);
        assert!((auras.active[0].remaining - 5.0).abs() < 0.01);
    }

    #[test]
    fn tick_expires_some_keeps_others() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 1)); // 10s
        auras.apply(make_dot(200, 2)); // 15s
        let expired = auras.tick_and_expire(12.0);
        assert_eq!(expired, 1);
        assert_eq!(auras.active.len(), 1);
        assert_eq!(auras.active[0].spell_id, 200, "DOT should survive");
    }

    #[test]
    fn dispel_removes_all_instances_of_spell() {
        let mut auras = Auras::default();
        auras.apply(make_dot(200, 1));
        auras.apply(make_dot(200, 2)); // different caster DOT
        auras.apply(make_aura(300, 1, 1));
        let removed = auras.dispel(200);
        assert_eq!(removed, 2);
        assert_eq!(auras.active.len(), 1);
        assert_eq!(auras.active[0].spell_id, 300);
    }

    #[test]
    fn dispel_nonexistent_spell_removes_nothing() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 1));
        let removed = auras.dispel(999);
        assert_eq!(removed, 0);
        assert_eq!(auras.active.len(), 1);
    }

    #[test]
    fn cancel_removes_specific_caster_aura() {
        let mut auras = Auras::default();
        auras.apply(make_dot(200, 1));
        auras.apply(make_dot(200, 2));
        let removed = auras.cancel(200, 1);
        assert!(removed);
        assert_eq!(auras.active.len(), 1);
        assert_eq!(auras.active[0].caster, 2, "other caster's DOT kept");
    }

    #[test]
    fn cancel_wrong_caster_returns_false() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 1));
        let removed = auras.cancel(100, 999);
        assert!(!removed);
        assert_eq!(auras.active.len(), 1);
    }

    #[test]
    fn clear_all_removes_everything() {
        let mut auras = Auras::default();
        auras.apply(make_aura(100, 1, 1));
        auras.apply(make_dot(200, 2));
        auras.apply(make_aura(300, 3, 5));
        auras.clear_all();
        assert!(auras.active.is_empty());
    }

    #[test]
    fn absorb_fully_absorbed() {
        let mut auras = Auras::default();
        auras.active.push(make_shield(100, 1, SpellSchool::Fire, 500.0));
        let leftover = auras.absorb_damage(300.0, SpellSchool::Fire);
        assert_eq!(leftover, 0.0);
        if let Some(AuraEffect::SchoolAbsorb { remaining, .. }) =
            auras.active[0].effects[0]
        {
            assert!((remaining - 200.0).abs() < 0.01);
        } else {
            panic!("shield effect missing");
        }
    }

    #[test]
    fn absorb_partially_absorbed() {
        let mut auras = Auras::default();
        auras.active.push(make_shield(100, 1, SpellSchool::Fire, 200.0));
        let leftover = auras.absorb_damage(500.0, SpellSchool::Fire);
        assert!((leftover - 300.0).abs() < 0.01);
        assert!(auras.active.is_empty());
    }

    #[test]
    fn absorb_wrong_school_ignored() {
        let mut auras = Auras::default();
        auras.active.push(make_shield(100, 1, SpellSchool::Fire, 500.0));
        let leftover = auras.absorb_damage(300.0, SpellSchool::Frost);
        assert_eq!(leftover, 300.0);
        assert_eq!(auras.active.len(), 1);
    }

    #[test]
    fn absorb_weakest_first() {
        let mut auras = Auras::default();
        auras.active.push(make_shield(100, 1, SpellSchool::Shadow, 1000.0));
        auras.active.push(make_shield(200, 2, SpellSchool::Shadow, 200.0));

        let leftover = auras.absorb_damage(300.0, SpellSchool::Shadow);
        assert_eq!(leftover, 0.0);
        assert_eq!(auras.active.len(), 1, "weak shield should be depleted");
        assert_eq!(auras.active[0].spell_id, 100);
        if let Some(AuraEffect::SchoolAbsorb { remaining, .. }) =
            auras.active[0].effects[0]
        {
            assert!((remaining - 900.0).abs() < 0.01);
        } else {
            panic!("strong shield effect missing");
        }
    }

    #[test]
    fn absorb_multiple_shields_chained() {
        let mut auras = Auras::default();
        auras.active.push(make_shield(100, 1, SpellSchool::Holy, 100.0));
        auras.active.push(make_shield(200, 2, SpellSchool::Holy, 150.0));

        let leftover = auras.absorb_damage(400.0, SpellSchool::Holy);
        assert!((leftover - 150.0).abs() < 0.01);
        assert!(auras.active.is_empty());
    }

    #[test]
    fn absorb_zero_damage_returns_zero() {
        let mut auras = Auras::default();
        auras.active.push(make_shield(100, 1, SpellSchool::Fire, 500.0));
        let leftover = auras.absorb_damage(0.0, SpellSchool::Fire);
        assert_eq!(leftover, 0.0);
    }

    #[test]
    fn absorb_preserves_non_absorb_auras() {
        let mut auras = Auras::default();
        let mut buff = make_aura(300, 1, 1);
        buff.effects[0] = Some(AuraEffect::ModDamageDone { percent: 0.10 });
        auras.apply(buff);
        auras.active.push(make_shield(100, 2, SpellSchool::Fire, 50.0));
        let leftover = auras.absorb_damage(50.0, SpellSchool::Fire);
        assert_eq!(leftover, 0.0);
        assert_eq!(auras.active.len(), 1);
        assert_eq!(auras.active[0].spell_id, 300);
    }

    #[test]
    fn damage_done_no_auras() {
        let auras = Auras::default();
        assert_eq!(auras.damage_done_multiplier(), 1.0);
    }

    #[test]
    fn damage_done_single_buff() {
        let mut auras = Auras::default();
        auras.active.push(make_damage_done_buff(100, 1, 0.10));
        assert!((auras.damage_done_multiplier() - 1.10).abs() < 0.001);
    }

    #[test]
    fn damage_done_multiplicative_stacking() {
        let mut auras = Auras::default();
        auras.active.push(make_damage_done_buff(100, 1, 0.10)); // +10%
        auras.active.push(make_damage_done_buff(200, 2, 0.20)); // +20%
        // 1.1 * 1.2 = 1.32, NOT 1.3 (additive)
        assert!((auras.damage_done_multiplier() - 1.32).abs() < 0.001);
    }

    #[test]
    fn damage_done_negative_reduces() {
        let mut auras = Auras::default();
        auras.active.push(make_damage_done_buff(100, 1, -0.20)); // -20%
        assert!((auras.damage_done_multiplier() - 0.80).abs() < 0.001);
    }

    #[test]
    fn damage_taken_single_debuff() {
        let mut auras = Auras::default();
        auras.active.push(make_damage_taken_debuff(100, 1, 0.20));
        assert!((auras.damage_taken_multiplier() - 1.20).abs() < 0.001);
    }

    #[test]
    fn damage_taken_reduction() {
        let mut auras = Auras::default();
        auras.active.push(make_damage_taken_debuff(100, 1, -0.10));
        assert!((auras.damage_taken_multiplier() - 0.90).abs() < 0.001);
    }

    #[test]
    fn damage_taken_multiplicative_stacking() {
        let mut auras = Auras::default();
        auras.active.push(make_damage_taken_debuff(100, 1, -0.10));
        auras.active.push(make_damage_taken_debuff(200, 2, -0.20));
        // 0.9 * 0.8 = 0.72
        assert!((auras.damage_taken_multiplier() - 0.72).abs() < 0.001);
    }

    #[test]
    fn damage_modifiers_ignore_other_effects() {
        let mut auras = Auras::default();
        auras.active.push(make_dot(100, 1));
        auras.active.push(make_shield(200, 1, SpellSchool::Fire, 500.0));
        assert_eq!(auras.damage_done_multiplier(), 1.0);
        assert_eq!(auras.damage_taken_multiplier(), 1.0);
    }

    #[test]
    fn proc_fires_on_matching_event() {
        let mut auras = Auras::default();
        auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnCrit, 1.0, 0.0, 999));
        let results = auras.check_procs(ProcTrigger::OnCrit, 0.5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_spell_id, 100);
        assert_eq!(results[0].action, ProcAction::ApplySpell { spell_id: 999 });
    }

    #[test]
    fn proc_ignores_wrong_event() {
        let mut auras = Auras::default();
        auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnCrit, 1.0, 0.0, 999));
        let results = auras.check_procs(ProcTrigger::OnHit, 0.5);
        assert!(results.is_empty());
    }

    #[test]
    fn proc_fails_roll() {
        let mut auras = Auras::default();
        auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 0.3, 0.0, 999));
        let results = auras.check_procs(ProcTrigger::OnHit, 0.5);
        assert!(results.is_empty());
    }

    #[test]
    fn proc_passes_roll() {
        let mut auras = Auras::default();
        auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 0.3, 0.0, 999));
        let results = auras.check_procs(ProcTrigger::OnHit, 0.2);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn proc_icd_prevents_refire() {
        let mut auras = Auras::default();
        auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 10.0, 999));

        let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
        assert_eq!(results.len(), 1);

        let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
        assert!(results.is_empty());
    }

    #[test]
    fn proc_icd_expires_after_tick() {
        let mut auras = Auras::default();
        auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 5.0, 999));

        auras.check_procs(ProcTrigger::OnHit, 0.0);
        auras.tick_proc_cooldowns(5.1);

        let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn proc_multiple_auras_checked() {
        let mut auras = Auras::default();
        auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 0.0, 999));
        auras.active.push(make_proc_aura(200, 2, ProcTrigger::OnHit, 1.0, 0.0, 888));
        let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn proc_no_proc_def_skipped() {
        let mut auras = Auras::default();
        auras.active.push(make_aura(100, 1, 1));
        let results = auras.check_procs(ProcTrigger::OnHit, 0.0);
        assert!(results.is_empty());
    }

    #[test]
    fn tick_proc_cooldowns_decrements() {
        let mut auras = Auras::default();
        auras.active.push(make_proc_aura(100, 1, ProcTrigger::OnHit, 1.0, 10.0, 999));
        auras.check_procs(ProcTrigger::OnHit, 0.0);
        auras.tick_proc_cooldowns(3.0);
        let icd = auras.active[0].proc_def.as_ref().unwrap().icd_remaining;
        assert!((icd - 7.0).abs() < 0.01);
    }

    #[test]
    fn proc_cooldown_reset() {
        use crate::casting::SpellCooldowns;
        let aura = Aura {
            spell_id: 500,
            caster: 1,
            duration: 60.0,
            remaining: 60.0,
            effects: [None, None, None],
            proc_def: Some(ProcDef {
                trigger: ProcTrigger::OnCrit,
                chance: 1.0,
                icd: 0.0,
                icd_remaining: 0.0,
                action: ProcAction::ResetCooldown { spell_id: 47486 },
            }),
            ..Default::default()
        };
        let mut auras = Auras::default();
        auras.active.push(aura);

        let results = auras.check_procs(ProcTrigger::OnCrit, 0.0);
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].action,
            ProcAction::ResetCooldown { spell_id: 47486 }
        );

        let mut cds = SpellCooldowns::default();
        cds.start(47486, 6.0);
        assert!(cds.is_on_cooldown(47486));

        if let ProcAction::ResetCooldown { spell_id } = results[0].action {
            cds.reset(spell_id);
        }
        assert!(!cds.is_on_cooldown(47486));
    }

    #[test]
    fn proc_action_variants() {
        let apply = ProcAction::ApplySpell { spell_id: 100 };
        let reset = ProcAction::ResetCooldown { spell_id: 200 };
        assert_ne!(apply, reset);
    }

    #[test]
    fn threat_multiplier_no_auras() {
        let auras = Auras::default();
        assert_eq!(auras.threat_multiplier(), 1.0);
    }

    #[test]
    fn threat_multiplier_righteous_fury() {
        let mut auras = Auras::default();
        auras.active.push(make_threat_aura(100, 1, 0.43));
        assert!((auras.threat_multiplier() - 1.43).abs() < 0.001);
    }

    #[test]
    fn threat_multiplier_stacks_multiplicatively() {
        let mut auras = Auras::default();
        auras.active.push(make_threat_aura(100, 1, 0.30));
        auras.active.push(make_threat_aura(200, 1, 0.10));
        assert!((auras.threat_multiplier() - 1.43).abs() < 0.001);
    }

    #[test]
    fn threat_multiplier_reduction() {
        let mut auras = Auras::default();
        auras.active.push(make_threat_aura(100, 1, -0.30));
        assert!((auras.threat_multiplier() - 0.70).abs() < 0.001);
    }

    #[test]
    fn cdr_no_auras_returns_base() {
        let auras = Auras::default();
        assert_eq!(auras.effective_cooldown(100, 10.0), 10.0);
    }

    #[test]
    fn cdr_specific_spell_reduction() {
        let mut auras = Auras::default();
        auras.active.push(make_cdr_aura(1000, 1, 100, 2.0));
        assert_eq!(auras.effective_cooldown(100, 10.0), 8.0);
        assert_eq!(auras.effective_cooldown(200, 10.0), 10.0);
    }

    #[test]
    fn cdr_global_reduction() {
        let mut auras = Auras::default();
        auras.active.push(make_cdr_aura(1000, 1, 0, 1.0));
        assert_eq!(auras.effective_cooldown(100, 8.0), 7.0);
        assert_eq!(auras.effective_cooldown(200, 5.0), 4.0);
    }

    #[test]
    fn cdr_stacks_additively() {
        let mut auras = Auras::default();
        auras.active.push(make_cdr_aura(1000, 1, 100, 2.0));
        auras.active.push(make_cdr_aura(1001, 1, 100, 1.5));
        assert!((auras.effective_cooldown(100, 10.0) - 6.5).abs() < 0.001);
    }

    #[test]
    fn cdr_clamps_to_zero() {
        let mut auras = Auras::default();
        auras.active.push(make_cdr_aura(1000, 1, 100, 20.0));
        assert_eq!(auras.effective_cooldown(100, 8.0), 0.0);
    }

    #[test]
    fn cdr_global_and_specific_stack() {
        let mut auras = Auras::default();
        auras.active.push(make_cdr_aura(1000, 1, 0, 1.0));
        auras.active.push(make_cdr_aura(1001, 1, 100, 2.0));
        assert_eq!(auras.effective_cooldown(100, 10.0), 7.0);
        assert_eq!(auras.effective_cooldown(200, 10.0), 9.0);
    }

    #[test]
    fn speed_no_auras_normal() {
        let auras = Auras::default();
        assert_eq!(auras.movement_speed_multiplier(), 1.0);
        assert!(!auras.is_rooted());
    }

    #[test]
    fn speed_snare_50_percent() {
        let mut auras = Auras::default();
        auras.active.push(make_speed_aura(100, 1, -0.50));
        assert!((auras.movement_speed_multiplier() - 0.50).abs() < 0.001);
    }

    #[test]
    fn speed_root_is_zero() {
        let mut auras = Auras::default();
        auras.active.push(make_speed_aura(100, 1, -1.0));
        assert_eq!(auras.movement_speed_multiplier(), 0.0);
        assert!(auras.is_rooted());
    }

    #[test]
    fn speed_strongest_snare_wins() {
        let mut auras = Auras::default();
        auras.active.push(make_speed_aura(100, 1, -0.30));
        auras.active.push(make_speed_aura(200, 2, -0.70));
        assert!((auras.movement_speed_multiplier() - 0.30).abs() < 0.001);
    }

    #[test]
    fn speed_buff_stacks_additively() {
        let mut auras = Auras::default();
        auras.active.push(make_speed_aura(100, 1, 0.30));
        auras.active.push(make_speed_aura(200, 2, 0.20));
        assert!((auras.movement_speed_multiplier() - 1.50).abs() < 0.001);
    }

    #[test]
    fn speed_buff_and_snare_combined() {
        let mut auras = Auras::default();
        auras.active.push(make_speed_aura(100, 1, -0.50));
        auras.active.push(make_speed_aura(200, 2, 0.20));
        assert!((auras.movement_speed_multiplier() - 0.70).abs() < 0.001);
    }

    #[test]
    fn speed_root_overrides_buff() {
        let mut auras = Auras::default();
        auras.active.push(make_speed_aura(100, 1, -1.0));
        auras.active.push(make_speed_aura(200, 2, 0.50));
        assert_eq!(auras.movement_speed_multiplier(), 0.0);
    }

    #[test]
    fn periodic_tick_fires_at_interval() {
        let mut dot = make_dot_with_interval(100, 1, 200.0, 3.0);
        let ticks = dot.tick_periodic(2.0);
        assert!(ticks.is_empty());
        let ticks = dot.tick_periodic(1.5);
        assert_eq!(ticks, vec![PeriodicTick::Damage(200.0)]);
    }

    #[test]
    fn periodic_tick_fires_multiple_on_catchup() {
        let mut dot = make_dot_with_interval(100, 1, 150.0, 2.0);
        let ticks = dot.tick_periodic(5.0);
        assert_eq!(ticks, vec![PeriodicTick::Damage(150.0), PeriodicTick::Damage(150.0)]);
        assert!((dot.tick_timer - 1.0).abs() < 0.001);
    }

    #[test]
    fn periodic_hot_produces_heal_ticks() {
        let mut hot = make_hot(200, 1, 500.0, 2.0);
        let ticks = hot.tick_periodic(2.5);
        assert_eq!(ticks, vec![PeriodicTick::Heal(500.0)]);
    }

    #[test]
    fn periodic_no_tick_if_interval_zero() {
        let mut aura = make_aura(100, 1, 1);
        let ticks = aura.tick_periodic(10.0);
        assert!(ticks.is_empty());
    }

    #[test]
    fn tick_all_periodic_aggregates_across_auras() {
        let mut auras = Auras::default();
        auras.active.push(make_dot_with_interval(100, 1, 100.0, 2.0));
        auras.active.push(make_hot(200, 2, 300.0, 2.0));

        let ticks = auras.tick_all_periodic(2.5);
        assert_eq!(ticks.len(), 2);
        assert_eq!(ticks[0], (1, PeriodicTick::Damage(100.0)));
        assert_eq!(ticks[1], (2, PeriodicTick::Heal(300.0)));
    }

    #[test]
    fn snapshot_stats_preserved_on_aura() {
        let aura = Aura {
            spell_id: 100,
            caster: 1,
            duration: 15.0,
            remaining: 15.0,
            effects: [
                Some(AuraEffect::PeriodicDamage { damage: 200.0, interval: 3.0 }),
                None,
                None,
            ],
            tick_interval: 3.0,
            snapshot_spell_power: 3500.0,
            snapshot_attack_power: 1200.0,
            ..Default::default()
        };
        assert_eq!(aura.snapshot_spell_power, 3500.0);
        assert_eq!(aura.snapshot_attack_power, 1200.0);
    }

    #[test]
    fn haste_reduces_tick_interval_at_application() {
        use crate::formulas::spell::hasted_periodic;
        let haste = hasted_periodic(3.0, 4, 0.5);
        let aura = Aura {
            spell_id: 100,
            caster: 1,
            duration: haste.duration,
            remaining: haste.duration,
            effects: [
                Some(AuraEffect::PeriodicDamage { damage: 100.0, interval: 3.0 }),
                None,
                None,
            ],
            tick_interval: haste.tick_interval,
            ..Default::default()
        };
        assert!((aura.tick_interval - 2.0).abs() < 0.001);
        assert_eq!(haste.num_ticks, 6);
    }
}
