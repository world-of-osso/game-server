use std::ops::AddAssign;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
    Default,
)]
pub struct CharacterAppearance {
    pub sex: u8,
    pub skin_color: u8,
    pub face: u8,
    pub eye_color: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub facial_style: u8,
}

#[derive(
    Component,
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
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(
    Component,
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
pub struct Rotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(
    Component,
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
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(
    Component,
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
pub struct Mana {
    pub current: f32,
    pub max: f32,
}

#[derive(
    Component,
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
pub struct MovementSpeed(pub f32);

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
)]
pub struct Player {
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub appearance: CharacterAppearance,
}

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
pub enum EquipmentVisualSlot {
    Head,
    Shoulder,
    Back,
    Chest,
    Shirt,
    Tabard,
    Wrist,
    Hands,
    Waist,
    Legs,
    Feet,
    MainHand,
    OffHand,
}

#[derive(
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
pub struct EquippedAppearanceEntry {
    pub slot: EquipmentVisualSlot,
    pub item_id: Option<u32>,
    pub display_info_id: Option<u32>,
    pub inventory_type: u8,
    pub hidden: bool,
}

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
    Eq,
    Default,
)]
pub struct EquipmentAppearance {
    pub entries: Vec<EquippedAppearanceEntry>,
}

#[derive(
    Component,
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
pub struct Npc {
    pub template_id: u32,
}

#[derive(
    Component,
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
pub struct Zone {
    pub id: u32,
}

#[derive(
    Component,
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
pub struct ModelDisplay {
    pub display_id: u32,
}

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
)]
pub struct GuildMembership {
    pub guild_id: u32,
    pub guild_name: String,
}

#[derive(
    Component,
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Default,
)]
pub struct UnitStats {
    pub stamina: f32,
    pub strength: f32,
    pub agility: f32,
    pub intellect: f32,
    pub spirit: f32,
}

impl AddAssign for UnitStats {
    fn add_assign(&mut self, rhs: Self) {
        self.stamina += rhs.stamina;
        self.strength += rhs.strength;
        self.agility += rhs.agility;
        self.intellect += rhs.intellect;
        self.spirit += rhs.spirit;
    }
}

#[derive(
    Component,
    Reflect,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Default,
)]
pub struct CombatRatings {
    pub crit: f32,
    pub haste: f32,
    pub mastery: f32,
    pub versatility: f32,
    pub armor: f32,
    pub dodge: f32,
    pub parry: f32,
    pub block: f32,
}

impl AddAssign for CombatRatings {
    fn add_assign(&mut self, rhs: Self) {
        self.crit += rhs.crit;
        self.haste += rhs.haste;
        self.mastery += rhs.mastery;
        self.versatility += rhs.versatility;
        self.armor += rhs.armor;
        self.dodge += rhs.dodge;
        self.parry += rhs.parry;
        self.block += rhs.block;
    }
}

#[derive(
    Component,
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
)]
pub struct Level(pub u8);

/// Stat contribution from a single equipped item.
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
    Default,
)]
pub struct ItemStatBlock {
    pub primary: UnitStats,
    pub secondary: CombatRatings,
}

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

    /// Remove all auras (on death). Clears the entire active list.
    pub fn clear_all(&mut self) {
        self.active.clear();
    }
}

/// Spell school for damage/healing classification.
///
/// In retail WoW, spell resistance per school is removed. Physical damage
/// is mitigated by armor; magic schools have no resistance stat (only
/// versatility provides damage reduction).
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
pub enum SpellSchool {
    Physical,
    Holy,
    Fire,
    Nature,
    Frost,
    Shadow,
    Arcane,
}

impl SpellSchool {
    /// Whether this school's damage is reduced by armor.
    pub fn is_physical(self) -> bool {
        self == Self::Physical
    }
}

/// Weapon category for normalized speed lookup.
///
/// WoW abilities use a fixed "normalized" speed instead of the actual weapon
/// speed to prevent slow weapons from being disproportionately better for
/// instant attacks. Ref: SimC `weapon_t`, Blizzard normalization rules.
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
)]
pub enum WeaponType {
    Dagger,
    OneHand,
    TwoHand,
    Ranged,
}

impl WeaponType {
    /// Normalized weapon speed in seconds, used by ability damage formulas.
    pub fn normalized_speed(self) -> f32 {
        match self {
            Self::Dagger => 1.7,
            Self::OneHand => 2.4,
            Self::TwoHand => 3.3,
            Self::Ranged => 2.8,
        }
    }
}

/// Weapon damage range and speed for an equipped weapon.
///
/// `min_damage` / `max_damage` are the base weapon damage values from item data.
/// `speed` is the weapon swing interval in seconds (e.g. 2.6 for a 2H sword).
#[derive(
    Component,
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
pub struct WeaponDamage {
    pub min_damage: f32,
    pub max_damage: f32,
    pub speed: f32,
    pub weapon_type: WeaponType,
}

impl WeaponDamage {
    /// Average damage per swing: `(min + max) / 2`.
    pub fn average_damage(&self) -> f32 {
        (self.min_damage + self.max_damage) / 2.0
    }

    /// Damage per second: `average_damage / speed`.
    pub fn dps(&self) -> f32 {
        if self.speed > 0.0 {
            self.average_damage() / self.speed
        } else {
            0.0
        }
    }

    /// Normalized speed for ability damage formulas.
    pub fn normalized_speed(&self) -> f32 {
        self.weapon_type.normalized_speed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_stats_default_is_zero() {
        let stats = UnitStats::default();
        assert_eq!(stats.stamina, 0.0);
        assert_eq!(stats.strength, 0.0);
        assert_eq!(stats.agility, 0.0);
        assert_eq!(stats.intellect, 0.0);
        assert_eq!(stats.spirit, 0.0);
    }

    #[test]
    fn unit_stats_roundtrip_bitcode() {
        let stats = UnitStats {
            stamina: 150.0,
            strength: 200.0,
            agility: 80.0,
            intellect: 50.0,
            spirit: 60.0,
        };
        let encoded = bitcode::encode(&stats);
        let decoded: UnitStats = bitcode::decode(&encoded).unwrap();
        assert_eq!(stats, decoded);
    }

    #[test]
    fn combat_ratings_default_is_zero() {
        let ratings = CombatRatings::default();
        assert_eq!(ratings.crit, 0.0);
        assert_eq!(ratings.haste, 0.0);
        assert_eq!(ratings.mastery, 0.0);
        assert_eq!(ratings.versatility, 0.0);
        assert_eq!(ratings.armor, 0.0);
        assert_eq!(ratings.dodge, 0.0);
        assert_eq!(ratings.parry, 0.0);
        assert_eq!(ratings.block, 0.0);
    }

    #[test]
    fn combat_ratings_roundtrip_bitcode() {
        let ratings = CombatRatings {
            crit: 25.0,
            haste: 18.5,
            mastery: 30.0,
            versatility: 12.0,
            armor: 1500.0,
            dodge: 5.0,
            parry: 8.0,
            block: 10.0,
        };
        let encoded = bitcode::encode(&ratings);
        let decoded: CombatRatings = bitcode::decode(&encoded).unwrap();
        assert_eq!(ratings, decoded);
    }

    #[test]
    fn spell_school_physical_check() {
        assert!(SpellSchool::Physical.is_physical());
        assert!(!SpellSchool::Fire.is_physical());
        assert!(!SpellSchool::Holy.is_physical());
        assert!(!SpellSchool::Shadow.is_physical());
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
        ];
        assert_eq!(effects.len(), 10);
    }

    // --- Aura stacking tests ---

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

    // --- Aura removal tests ---

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

    // --- Absorb shield tests ---

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

    #[test]
    fn absorb_fully_absorbed() {
        let mut auras = Auras::default();
        auras.active.push(make_shield(100, 1, SpellSchool::Fire, 500.0));
        let leftover = auras.absorb_damage(300.0, SpellSchool::Fire);
        assert_eq!(leftover, 0.0);
        // Shield still has 200 remaining
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
        // Shield depleted, aura removed
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
        // Strong shield (1000) and weak shield (200), same school
        auras.active.push(make_shield(100, 1, SpellSchool::Shadow, 1000.0));
        auras.active.push(make_shield(200, 2, SpellSchool::Shadow, 200.0));

        let leftover = auras.absorb_damage(300.0, SpellSchool::Shadow);
        assert_eq!(leftover, 0.0);
        // Weak shield (200) consumed first, then 100 from strong shield
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

        // 400 damage chews through both shields (250 total), leaving 150
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
        // Buff with a real effect (damage modifier)
        let mut buff = make_aura(300, 1, 1);
        buff.effects[0] = Some(AuraEffect::ModDamageDone { percent: 0.10 });
        auras.apply(buff);
        auras.active.push(make_shield(100, 2, SpellSchool::Fire, 50.0));
        let leftover = auras.absorb_damage(50.0, SpellSchool::Fire);
        assert_eq!(leftover, 0.0);
        // Shield depleted, but buff preserved
        assert_eq!(auras.active.len(), 1);
        assert_eq!(auras.active[0].spell_id, 300);
    }

    // --- Periodic tick tests ---

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
    fn periodic_tick_fires_at_interval() {
        let mut dot = make_dot_with_interval(100, 1, 200.0, 3.0);
        // Not enough time yet
        let ticks = dot.tick_periodic(2.0);
        assert!(ticks.is_empty());
        // Cross the threshold
        let ticks = dot.tick_periodic(1.5);
        assert_eq!(ticks, vec![PeriodicTick::Damage(200.0)]);
    }

    #[test]
    fn periodic_tick_fires_multiple_on_catchup() {
        let mut dot = make_dot_with_interval(100, 1, 150.0, 2.0);
        // 5 seconds = 2 full ticks (at 2s and 4s), 1s remainder
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
        // tick_interval defaults to 0.0 for non-periodic auras
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
        // Pandaria+ model: caster stats are snapshotted at application time
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
        // Snapshot values are stored and retrievable
        assert_eq!(aura.snapshot_spell_power, 3500.0);
        assert_eq!(aura.snapshot_attack_power, 1200.0);
    }

    #[test]
    fn haste_reduces_tick_interval_at_application() {
        // Haste is applied when computing tick_interval before creating the aura.
        // 3s base tick, 50% haste → 2s tick interval
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
        // 6 ticks in 12s instead of 4 ticks
        assert_eq!(haste.num_ticks, 6);
    }

    #[test]
    fn level_roundtrip_bitcode() {
        let level = Level(70);
        let encoded = bitcode::encode(&level);
        let decoded: Level = bitcode::decode(&encoded).unwrap();
        assert_eq!(level, decoded);
    }

    #[test]
    fn weapon_damage_average() {
        let weapon = WeaponDamage {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 2.6,
            weapon_type: WeaponType::TwoHand,
        };
        assert_eq!(weapon.average_damage(), 150.0);
    }

    #[test]
    fn weapon_damage_dps() {
        let weapon = WeaponDamage {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 2.0,
            weapon_type: WeaponType::OneHand,
        };
        assert_eq!(weapon.dps(), 75.0);
    }

    #[test]
    fn weapon_damage_dps_zero_speed() {
        let weapon = WeaponDamage {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 0.0,
            weapon_type: WeaponType::OneHand,
        };
        assert_eq!(weapon.dps(), 0.0);
    }

    #[test]
    fn weapon_damage_roundtrip_bitcode() {
        let weapon = WeaponDamage {
            min_damage: 85.0,
            max_damage: 159.0,
            speed: 3.3,
            weapon_type: WeaponType::TwoHand,
        };
        let encoded = bitcode::encode(&weapon);
        let decoded: WeaponDamage = bitcode::decode(&encoded).unwrap();
        assert_eq!(weapon, decoded);
    }

    #[test]
    fn normalized_speed_values() {
        assert_eq!(WeaponType::Dagger.normalized_speed(), 1.7);
        assert_eq!(WeaponType::OneHand.normalized_speed(), 2.4);
        assert_eq!(WeaponType::TwoHand.normalized_speed(), 3.3);
        assert_eq!(WeaponType::Ranged.normalized_speed(), 2.8);
    }

    #[test]
    fn weapon_normalized_speed_delegates() {
        let weapon = WeaponDamage {
            min_damage: 50.0,
            max_damage: 100.0,
            speed: 1.5,
            weapon_type: WeaponType::Dagger,
        };
        assert_eq!(weapon.normalized_speed(), 1.7);
    }
}
