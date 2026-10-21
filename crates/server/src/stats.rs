//! Player and approved follower-bridge character stats (retail 12.x model).
//!
//! `compute_character_stats` turns class/race/level base stats, gear and
//! optional aura `StatModifiers` into `CharacterStats`. `update_character_stats`
//! applies the result on world entry and whenever level, spec, gear or
//! modifiers change: it writes the effective `UnitStats` / `CombatRatings` /
//! `DerivedStats`, `Health.max` and `Mana.max`, rescaling current values by their ratio.
//! Sources and formulas: `docs/wiki/systems/character-stats.md`.

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{
    CombatRatings, DerivedStats, Health, ItemStatBlock, Level, Mana, Player, UnitStats,
};
use shared::formulas::sum_equipment_stats;
use shared::protocol::EquipmentSlot;

use crate::character_stats_data::{
    AVOIDANCE_RATING, BASE_MANA, BLOCK_RATING, CLASS_BASE_STATS, CRIT_RATING, DODGE_RATING,
    HASTE_RATING, HP_PER_STAMINA, LEECH_RATING, MASTERY_RATING, PARRY_RATING, RACE_STAT_MODS,
    SPEED_RATING, VERSATILITY_RATING,
};
use crate::class_progression::{ActiveSpec, KnownSpells};
use crate::item_stats::{ForeverItemScalingTables, ItemScalingTables, WeaponDamage, item_stats};
use crate::item_templates::ItemTemplates;
use crate::melee::PlayerDefense;
use crate::spell_info::{ClassInfo, ClassSpells, SpellStore};

/// Every player has 5% base crit (SimC `dbc_t::all_crit_base`).
const BASE_CRIT_PCT: f32 = 5.0;
/// Every player has 8 base mastery points (SimC `player_t::init_base_stats`).
const BASE_MASTERY_POINTS: f32 = 8.0;

/// Item ids physically equipped, by slot. The inventory sets it on world
/// entry and after every item change.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct EquippedItems {
    pub items: Vec<(EquipmentSlot, shared::item_data::ItemDefinitionSource, u32)>,
}

/// Instance attributes follow the equipped GUID independently of its catalog identity.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct EquippedItemModifiers {
    pub items: Vec<(EquipmentSlot, server::item_instance::ItemInstanceModifiers)>,
}

#[derive(Resource, Default)]
pub struct NativeItemInstances {
    pub bonuses: server::item_instance::bonuses::ItemBonusTables,
    pub enchants: server::item_instance::enchants::EnchantmentTables,
    pub effects: server::item_instance::effects::ItemEffectTables,
}

/// Aura stat changes across create/base, bonus and total stages. Absent means none.
/// Percent fields store 10.0 for +10%; aura handlers compose their modifiers.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct StatModifiers {
    /// Player-only aura80: create/base percentage, before flat/gear bonuses and137.
    pub primary_base_pct: UnitStats,
    /// Flat primary bonuses, added alongside gear after create/base scaling.
    pub primary_flat: UnitStats,
    /// Percent increase per primary stat, applied after `primary_flat`.
    pub primary_pct: UnitStats,
    /// Aura467: additive percent of gear/flat bonuses, excluding create/base stats.
    pub primary_bonus_pct: UnitStats,
    /// Added to gear ratings before conversion.
    pub ratings_flat: CombatRatings,
    /// Aura405: ordered rating mask/percent pairs; truncation occurs per effect.
    pub rating_percent_mods: Vec<(u32, f32)>,
    /// Aura463: composed percentage of base melee crit rating added to parry.
    pub crit_to_parry_pct: f32,
    /// Aura419: composed BASE_PCT mana modifier; never changes create-mana costs.
    pub base_mana_pct: f32,
    /// Aura404: presence replaces both AP bases, including an authored zero.
    pub attack_power_from_sp_pct: Option<f32>,
    pub crit_pct: f32,
    pub haste_pct: f32,
    pub mastery_points: f32,
    pub versatility_pct: f32,
    pub max_health_pct: f32,
    /// MOD_INCREASE_HEALTH: fixed current/max health gained on application.
    pub max_health_flat: f32,
    /// OVERRIDE_SPELL_POWER_BY_AP_PCT: when positive, spell and healing power are
    /// this percent of attack power.
    pub spell_power_from_ap_pct: f32,
    /// Composed independent aura166/167 percentages, kept separate.
    pub attack_power_pct: f32,
    pub ranged_attack_power_pct: f32,
    /// MOD_DODGE_PERCENT: nondiminishing percentage points.
    pub dodge_pct: f32,
    /// MOD_PARRY_PERCENT: nondiminishing percentage points.
    pub parry_pct: f32,
    /// MOD_BLOCK_PERCENT: added to block chance (`player_defense`).
    pub block_pct: f32,
    /// MOD_ARMOR_PCT_FROM_STAT268: additive percentages of live primary stats.
    pub armor_from_stat_pct: UnitStats,
    /// MiscValue -2 selects the specialization's primary stat.
    pub armor_from_primary_stat_pct: f32,
    /// Aura142: composed base/gear armor percentage, excluding stat-derived armor.
    pub base_armor_pct: f32,
    /// Aura466: composed final armor percentage, applied after stat-derived armor.
    pub bonus_armor_pct: f32,
    /// Aura101 physical: composed armor TOTAL_PCT, applied after stat-derived armor.
    pub total_armor_pct: f32,
}

/// The spec's main stat: gear Str/Agi/Int count only when they match it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryStat {
    Strength,
    Agility,
    Intellect,
}

impl PrimaryStat {
    /// `ChrSpecialization.PrimaryStatPriority`: 0-1 Int, 2-3 Agi, 4-5 Str.
    pub fn from_priority(priority: u32) -> Self {
        match priority {
            0 | 1 => Self::Intellect,
            2 | 3 => Self::Agility,
            _ => Self::Strength,
        }
    }
}

/// Everything `compute_character_stats` reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatInputs {
    pub class: u8,
    pub race: u8,
    pub level: u8,
    /// The spec's main stat, else the class's (Player::GetPrimaryStat,
    /// StatSystem.cpp:297-314). `None` without class data: all gear stats
    /// count and there is no spell power.
    pub primary_stat: Option<PrimaryStat>,
    /// Mastery spell effect 0 `EffectBonusCoefficient`; 0 without mastery.
    pub mastery_coefficient: f32,
    /// `chr_classes.AttackPowerPerStrength` / `AttackPowerPerAgility`.
    pub attack_power_per_strength: f32,
    pub attack_power_per_agility: f32,
    pub ranged_attack_power_per_agility: f32,
    pub gear_primary: UnitStats,
    pub gear_ratings: CombatRatings,
}

/// Derived stats of a player (server-only).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct CharacterStats {
    /// Effective primary stats (base + gear + modifiers).
    pub primary: UnitStats,
    /// Effective ratings (gear + modifiers).
    pub ratings: CombatRatings,
    pub max_health: f32,
    /// Create mana (gtBaseMP), 0 for classes without mana. `PowerCostPct`
    /// costs use this unmodified basis, not the effective maximum.
    pub base_mana: f32,
    /// Effective mana maximum after aura419's BASE_PCT modifier.
    pub max_mana: f32,
    /// Damage spell power: Intellect when the main stat is Intellect, else 0
    /// (Unit::SpellBaseDamageBonusDone, Unit.cpp:7118-7119).
    pub spell_power: f32,
    /// Healing and absorb spell power: Intellect for every class that uses mana
    /// (Unit::SpellBaseHealingBonusDone, Unit.cpp:7622-7624), so a Strength
    /// paladin heals with its Intellect.
    pub healing_power: f32,
    /// Melee AP; ranged casts use the separate source RangedAttackPower field.
    pub attack_power: f32,
    pub ranged_attack_power: f32,
    pub crit_pct: f32,
    pub haste_pct: f32,
    pub mastery_pct: f32,
    /// Raw points before the active specialization's display coefficient.
    pub mastery_points: f32,
    pub versatility_pct: f32,
    /// Tertiary ratings after their diminishing returns.
    pub leech_pct: f32,
    pub avoidance_pct: f32,
    pub speed_pct: f32,
    pub armor: f32,
    /// Main-hand swing data; unarmed without a weapon.
    pub main_hand: WeaponDamage,
    /// An equipped off-hand weapon swings too.
    pub off_hand: Option<WeaponDamage>,
    /// Dodge, parry and block (`player_defense`).
    pub defense: PlayerDefense,
    /// `m_baseHealthRegen`: the gear's `ITEM_MOD_HEALTH_REGEN`, health per 5 s.
    pub health_regen: f32,
}

impl CharacterStats {
    /// Unit.cpp7036/8229: incoming damage uses half the combined rating and
    /// aura471 percentage; damage cannot fall below zero.
    pub(crate) fn versatility_damage_taken_multiplier(&self) -> f32 {
        (1.0 - self.versatility_pct / 200.0).max(0.0)
    }

    /// What the owner's character sheet and spell tooltips read. TrinityCore keeps the
    /// same values in `ActivePlayerData`: spell power (`Unit::SpellBaseDamageBonusDone`,
    /// Unit.cpp:7101-7119), crit (`UpdateCritPercentage` / `UpdateSpellCritChance`,
    /// StatSystem.cpp:509-546, 717-729), mastery (`UpdateMastery`, StatSystem.cpp:549-559)
    /// and versatility (`GetRatingBonusValue(CR_VERSATILITY_DAMAGE_DONE)`,
    /// Player.cpp:5172-5178). Leech, avoidance and speed fill `ActivePlayerData`
    /// `Lifesteal`, `Avoidance` and `Speed` (UpdateFields.h:417, 1399-1400).
    pub fn derived(&self) -> DerivedStats {
        DerivedStats {
            spell_power: self.spell_power,
            attack_power: self.attack_power,
            ranged_attack_power: self.ranged_attack_power,
            crit_pct: self.crit_pct,
            haste_pct: self.haste_pct,
            mastery_pct: self.mastery_pct,
            versatility_pct: self.versatility_pct,
            leech_pct: self.leech_pct,
            avoidance_pct: self.avoidance_pct,
            speed_pct: self.speed_pct,
        }
    }
}

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_character_stats);
    }
}

fn level_index(level: u8) -> Option<usize> {
    (1..=HP_PER_STAMINA.len())
        .contains(&usize::from(level))
        .then(|| usize::from(level) - 1)
}

/// Class base + racial modifier; `None` for unknown class or level.
pub fn base_primary_stats(class: u8, race: u8, level: u8) -> Option<UnitStats> {
    let [str, agi, sta, int] = CLASS_BASE_STATS
        .get(usize::from(class))
        .filter(|_| class > 0)?[level_index(level)?];
    let [race_str, race_agi, race_sta, race_int] = RACE_STAT_MODS
        .iter()
        .find(|(id, _)| *id == race)
        .map_or([0; 4], |(_, mods)| *mods);
    let stat = |base: u16, racial: i16| f32::from(base) + f32::from(racial);
    Some(UnitStats {
        strength: stat(str, race_str),
        agility: stat(agi, race_agi),
        stamina: stat(sta, race_sta),
        intellect: stat(int, race_int),
        spirit: 0.0,
    })
}

/// gtBaseMP for the class; `None` for classes without mana.
pub fn base_mana(class: u8, level: u8) -> Option<f32> {
    let mana = BASE_MANA.get(usize::from(class))?[level_index(level)?];
    (mana > 0).then_some(mana as f32)
}

/// Gear primary stats the spec uses: stamina plus its main stat.
fn usable_gear_primary(gear: UnitStats, primary: Option<PrimaryStat>) -> UnitStats {
    let Some(primary) = primary else {
        return gear;
    };
    UnitStats {
        stamina: gear.stamina,
        strength: if primary == PrimaryStat::Strength {
            gear.strength
        } else {
            0.0
        },
        agility: if primary == PrimaryStat::Agility {
            gear.agility
        } else {
            0.0
        },
        intellect: if primary == PrimaryStat::Intellect {
            gear.intellect
        } else {
            0.0
        },
        spirit: 0.0,
    }
}

fn apply_primary_pct(stats: UnitStats, pct: UnitStats) -> UnitStats {
    let scale = |value: f32, pct: f32| value * (1.0 + pct / 100.0);
    UnitStats {
        stamina: scale(stats.stamina, pct.stamina),
        strength: scale(stats.strength, pct.strength),
        agility: scale(stats.agility, pct.agility),
        intellect: scale(stats.intellect, pct.intellect),
        spirit: scale(stats.spirit, pct.spirit),
    }
}

/// Secondary-stat diminishing returns, curve 21024 `(input, output)` in
/// percent (mastery: points). SimC `DIMINISHING_RETURN_SECONDARY_CR_CURVE`,
/// `item_scaling.inc` for 12.0.1.66709; `player_t::apply_combat_rating_dr`.
const SECONDARY_DR_CURVE: [(f32, f32); 8] = [
    (0.0, 0.0),
    (30.0, 30.0),
    (40.0, 39.0),
    (50.0, 47.0),
    (60.0, 54.0),
    (80.0, 66.0),
    (100.0, 76.0),
    (200.0, 126.0),
];

/// Leech, avoidance and speed diminishing returns, curve 21025 (TrinityCore
/// `GlobalCurve::SpeedDiminishing` / `AvoidanceDiminishing` / `LifestealDiminishing`,
/// DBCEnums.h:1178-1181); SimC `DIMINISHING_RETURN_TERTIARY_CR_CURVE`, `item_scaling.inc`.
const TERTIARY_DR_CURVE: [(f32, f32); 7] = [
    (0.0, 0.0),
    (0.5, 0.5),
    (10.0, 10.0),
    (15.0, 14.0),
    (20.0, 17.0),
    (25.0, 19.0),
    (100.0, 49.0),
];

/// Piecewise-linear lookup, clamped to the curve's ends.
fn curve_value(curve: &[(f32, f32)], value: f32) -> f32 {
    let upper = curve
        .iter()
        .position(|&(x, _)| x >= value)
        .unwrap_or(curve.len() - 1);
    if upper == 0 || value >= curve[upper].0 {
        return curve[upper].1;
    }
    let (x0, y0) = curve[upper - 1];
    let (x1, y1) = curve[upper];
    y0 + (value - x0) / (x1 - x0) * (y1 - y0)
}

/// Rating → percent (mastery: points) after DR. `per_point` is the
/// gtCombatRatings value: rating for 1% or 1 mastery point (SimC `rating_t::init`).
fn rating_value(rating: f32, per_point: f32) -> f32 {
    curve_value(&SECONDARY_DR_CURVE, rating / per_point)
}

/// Tertiary rating → percent after DR (Player::ApplyRatingDiminishing, Player.cpp:5180-5225).
fn tertiary_rating_value(rating: f32, per_point: f32) -> f32 {
    curve_value(&TERTIARY_DR_CURVE, rating / per_point)
}

/// TC a352b1fa StatSystem.cpp:255-276, Player::UpdateArmor: live stat
/// percentages add to armor, rather than snapshotting agility at aura apply.
fn armor_from_stats(primary: &UnitStats, main: Option<PrimaryStat>, mods: &StatModifiers) -> f32 {
    let pct = &mods.armor_from_stat_pct;
    let explicit = primary.strength * pct.strength
        + primary.agility * pct.agility
        + primary.stamina * pct.stamina
        + primary.intellect * pct.intellect;
    let main_value = match main {
        Some(PrimaryStat::Strength) => primary.strength,
        Some(PrimaryStat::Agility) => primary.agility,
        Some(PrimaryStat::Intellect) => primary.intellect,
        None => 0.0,
    };
    (explicit + main_value * mods.armor_from_primary_stat_pct) / 100.0
}

/// TC Player.cpp5271-5279: sequential integer rating changes, then base crit conversion.
fn modified_ratings(base: CombatRatings, mods: &StatModifiers) -> CombatRatings {
    let mut ratings = base;
    for &(mask, percent) in &mods.rating_percent_mods {
        for (bits, value) in rating_fields(&mut ratings) {
            if mask & bits != 0 {
                *value += (*value * percent / 100.0).trunc();
            }
        }
    }
    ratings.parry += (base.crit * mods.crit_to_parry_pct / 100.0).trunc();
    for (_, value) in rating_fields(&mut ratings) {
        *value = value.max(0.0);
    }
    ratings
}

/// TC Unit.h329-365. Shared melee/ranged/spell ratings select a field only once.
fn rating_fields(ratings: &mut CombatRatings) -> [(u32, &mut f32); 10] {
    [
        (1 << 2, &mut ratings.dodge),
        (1 << 3, &mut ratings.parry),
        (1 << 4, &mut ratings.block),
        (0x700, &mut ratings.crit),
        (0xE0000, &mut ratings.haste),
        (1 << 25, &mut ratings.mastery),
        (0x7000_0000, &mut ratings.versatility),
        (1 << 13, &mut ratings.speed),
        (1 << 16, &mut ratings.leech),
        (1 << 20, &mut ratings.avoidance),
    ]
}

/// Retail stats for `input`, with aura `mods` folded in. `None` outside
/// the class (1..=13) and level (1..=90) tables.
pub fn compute_character_stats(
    input: &StatInputs,
    mods: Option<&StatModifiers>,
) -> Option<CharacterStats> {
    let level = level_index(input.level)?;
    let default_mods = StatModifiers::default();
    let mods = mods.unwrap_or(&default_mods);
    let mut primary = apply_primary_pct(
        base_primary_stats(input.class, input.race, input.level)?,
        mods.primary_base_pct,
    );
    let mut bonus = usable_gear_primary(input.gear_primary, input.primary_stat);
    bonus += mods.primary_flat;
    primary += apply_primary_pct(bonus, mods.primary_bonus_pct);
    let primary = apply_primary_pct(primary, mods.primary_pct);
    let mut ratings = input.gear_ratings;
    ratings += mods.ratings_flat;
    let ratings = modified_ratings(ratings, mods);
    let base_mana = base_mana(input.class, input.level);
    let base_attack_power = primary.strength * input.attack_power_per_strength
        + primary.agility * input.attack_power_per_agility;
    let attack_power = base_attack_power * (1.0 + mods.attack_power_pct / 100.0);
    let base_ranged_attack_power =
        (f32::from(input.level) + primary.agility.max(0.0)) * input.ranged_attack_power_per_agility;
    let ranged_attack_power =
        base_ranged_attack_power * (1.0 + mods.ranged_attack_power_pct / 100.0);
    let (spell_power, healing_power) = if mods.spell_power_from_ap_pct > 0.0 {
        // Unit::SpellBaseDamageBonusDone / SpellBaseHealingBonusDone return
        // int32(AP × pct + 0.5) before anything else (Unit.cpp:7103-7108, 7602-7607).
        let power = (attack_power * mods.spell_power_from_ap_pct / 100.0 + 0.5).floor();
        (power, power)
    } else {
        let intellect = primary.intellect.max(0.0);
        (
            if input.primary_stat == Some(PrimaryStat::Intellect) {
                intellect
            } else {
                0.0
            },
            if base_mana.is_some() { intellect } else { 0.0 },
        )
    };
    // TC StatSystem.cpp362-385 replaces both AP bases with minimum spell power.
    let (attack_power, ranged_attack_power) = if let Some(percent) = mods.attack_power_from_sp_pct {
        let power = spell_power.min(healing_power) * (percent / 100.0);
        (
            power * (1.0 + mods.attack_power_pct / 100.0),
            power * (1.0 + mods.ranged_attack_power_pct / 100.0),
        )
    } else {
        (attack_power, ranged_attack_power)
    };
    let mastery_points = BASE_MASTERY_POINTS
        + rating_value(ratings.mastery, MASTERY_RATING[level])
        + mods.mastery_points;
    Some(CharacterStats {
        primary,
        ratings,
        max_health: (primary.stamina.floor()
            * HP_PER_STAMINA[level]
            * (1.0 + mods.max_health_pct / 100.0)
            + mods.max_health_flat)
            .floor(),
        base_mana: base_mana.unwrap_or(0.0),
        max_mana: (base_mana.unwrap_or(0.0) * (1.0 + mods.base_mana_pct / 100.0)).round(),
        spell_power,
        healing_power,
        attack_power,
        ranged_attack_power,
        crit_pct: BASE_CRIT_PCT + rating_value(ratings.crit, CRIT_RATING[level]) + mods.crit_pct,
        haste_pct: rating_value(ratings.haste, HASTE_RATING[level]) + mods.haste_pct,
        mastery_pct: mastery_points * input.mastery_coefficient,
        mastery_points,
        versatility_pct: rating_value(ratings.versatility, VERSATILITY_RATING[level])
            + mods.versatility_pct,
        leech_pct: tertiary_rating_value(ratings.leech, LEECH_RATING[level]),
        avoidance_pct: tertiary_rating_value(ratings.avoidance, AVOIDANCE_RATING[level]),
        speed_pct: tertiary_rating_value(ratings.speed, SPEED_RATING[level]),
        armor: ((ratings.armor * (1.0 + mods.base_armor_pct / 100.0)
            + armor_from_stats(&primary, input.primary_stat, &mods))
            * (1.0 + mods.total_armor_pct / 100.0)
            * (1.0 + mods.bonus_armor_pct / 100.0))
            .floor(),
        main_hand: WeaponDamage::UNARMED,
        off_hand: None,
        defense: PlayerDefense::default(),
        health_regen: 0.0,
    })
}

/// SPELL_EFFECT_PARRY 22 and SPELL_EFFECT_BLOCK 23: a player who knows such a spell
/// can parry or block (Spell::EffectParry / EffectBlock, SpellEffects.cpp:3703-3719).
const EFFECT_PARRY: u32 = 22;
/// SPELL_EFFECT_DUAL_WIELD (Spell::EffectDualWield → `Player::SetCanDualWield`).
pub(crate) const EFFECT_DUAL_WIELD: u32 = 40;
const EFFECT_BLOCK: u32 = 23;
/// `ActivePlayerData` base parry and block (StatSystem.cpp:497, 664).
const BASE_PARRY_PCT: f32 = 5.0;
const BASE_BLOCK_PCT: f32 = 5.0;
/// `ShieldBlock` per point of shield armor (Player.cpp:8144).
const SHIELD_BLOCK_PER_ARMOR: f32 = 2.5;
/// Diminishing-return `k`, dodge and parry caps by class 1-13 (StatSystem.cpp:596-697).
const DIMINISHING_K: [f32; 13] = [
    0.956, 0.956, 0.988, 0.988, 0.983, 0.956, 0.988, 0.983, 0.983, 0.983, 0.972, 0.983, 0.988,
];
const DODGE_CAP: [f32; 13] = [
    65.63144, 65.63144, 145.560408, 145.560408, 150.37594, 65.63144, 145.560408, 150.37594,
    150.37594, 145.560408, 116.890707, 145.560408, 145.560408,
];
const PARRY_CAP: [f32; 13] = [
    65.63144, 65.63144, 145.560408, 145.560408, 0.0, 65.63144, 145.560408, 0.0, 0.0, 90.6425, 0.0,
    65.63144, 0.0,
];

/// What a player's avoidance depends on besides its class, level and ratings.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DefenseInputs {
    pub can_parry: bool,
    pub can_block: bool,
    /// A main- or off-hand weapon is equipped (Unit::GetUnitParryChance, Unit.cpp:2795-2799).
    pub has_weapon: bool,
    /// The equipped shield's armor, if any.
    pub shield_armor: Option<f32>,
    /// Σ MOD_DODGE_PERCENT (`StatModifiers::dodge_pct`).
    pub dodge_pct: f32,
    /// Σ MOD_PARRY_PERCENT, outside rating diminishing returns.
    pub parry_pct: f32,
    /// Σ MOD_BLOCK_PERCENT (`StatModifiers::block_pct`).
    pub block_pct: f32,
}

/// TrinityCore master's player avoidance: `Player::UpdateDodgePercentage`,
/// `UpdateParryPercentage` and `UpdateBlockPercentage` (StatSystem.cpp:490-507,
/// 657-715). Dodge is MOD_DODGE_PERCENT plus its rating after diminishing returns
/// (`GetDodgeFromAgility` gives nothing, Player.cpp:5031); parry 5% plus its rating after
/// diminishing returns, for a class with a parry cap that can parry and holds a weapon;
/// block 5% plus MOD_BLOCK_PERCENT plus its rating, for one that can block and holds a
/// shield (StatSystem.cpp:496-501, Unit.cpp:2843-2846).
pub fn player_defense(
    class: u8,
    level: u8,
    ratings: &CombatRatings,
    inputs: DefenseInputs,
) -> PlayerDefense {
    let (Some(level), Some(class_index)) = (level_index(level), usize::from(class).checked_sub(1))
    else {
        return PlayerDefense::default();
    };
    let (Some(&k), Some(&dodge_cap), Some(&parry_cap)) = (
        DIMINISHING_K.get(class_index),
        DODGE_CAP.get(class_index),
        PARRY_CAP.get(class_index),
    ) else {
        return PlayerDefense::default();
    };
    // CalculateDiminishingReturns (StatSystem.cpp:616-636): c·x / (x + c·k) + nondiminishing.
    let diminished = |cap: f32, nondiminishing: f32, diminishing: f32| {
        if diminishing + cap * k == 0.0 {
            return nondiminishing;
        }
        cap * diminishing / (diminishing + cap * k) + nondiminishing
    };
    let parry = if inputs.can_parry && parry_cap > 0.0 && inputs.has_weapon {
        diminished(
            parry_cap,
            BASE_PARRY_PCT + inputs.parry_pct,
            ratings.parry / PARRY_RATING[level],
        )
    } else {
        0.0
    };
    let shield_armor = inputs.shield_armor.filter(|_| inputs.can_block);
    PlayerDefense {
        // TrinityCore master, StatSystem.cpp Player::UpdateDodgePercentage:
        // https://github.com/TrinityCore/TrinityCore/blob/master/src/server/game/Entities/Unit/StatSystem.cpp
        // Aura points are not halved or subject to rating diminishing returns.
        dodge: diminished(
            dodge_cap,
            inputs.dodge_pct,
            ratings.dodge / DODGE_RATING[level],
        ),
        parry,
        block: shield_armor.map_or(0.0, |_| {
            BASE_BLOCK_PCT + inputs.block_pct + ratings.block / BLOCK_RATING[level]
        }),
        shield_block: inputs
            .shield_armor
            .map_or(0.0, |armor| armor * SHIELD_BLOCK_PER_ARMOR),
        sitting: false,
    }
}

/// Whether `known` holds a spell with `effect` (`Player::m_canParry` / `m_canBlock`).
pub(crate) fn knows_effect(
    known: Option<&KnownSpells>,
    spells: Option<&SpellStore>,
    effect: u32,
) -> bool {
    let (Some(known), Some(spells)) = (known, spells) else {
        return false;
    };
    known.iter().any(|id| {
        spells
            .get(id)
            .is_some_and(|spell| spell.effects.iter().any(|e| e.effect == effect))
    })
}

fn class_info(classes: Option<&ClassSpells>, class: u8) -> Option<&ClassInfo> {
    classes?.classes.iter().find(|info| info.class_id == class)
}

/// Main stat and mastery coefficient of the active spec; without one, the
/// class main stat and no mastery.
fn spec_inputs(
    class: Option<&ClassInfo>,
    spec: Option<u32>,
    spells: Option<&SpellStore>,
) -> (Option<PrimaryStat>, f32) {
    let Some(spec) = spec.and_then(|spec| class?.specs.iter().find(|info| info.spec_id == spec))
    else {
        let class_stat = class.map(|class| PrimaryStat::from_priority(class.primary_stat_priority));
        return (class_stat, 0.0);
    };
    let mastery = spec
        .mastery_spells
        .first()
        .and_then(|&id| spells?.get(id))
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.bonus_coefficient);
    (
        Some(PrimaryStat::from_priority(spec.primary_stat_priority)),
        mastery,
    )
}

/// Item data `update_character_stats` resolves equipped item ids with.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ItemData<'w> {
    templates: Option<Res<'w, ItemTemplates>>,
    tables: Option<Res<'w, ItemScalingTables>>,
    forever_tables: Option<Res<'w, ForeverItemScalingTables>>,
    instances: Option<Res<'w, NativeItemInstances>>,
}

impl ItemData<'_> {
    fn scaling_tables(
        &self,
        source: shared::item_data::ItemDefinitionSource,
    ) -> &ItemScalingTables {
        match source {
            shared::item_data::ItemDefinitionSource::Retail => self
                .tables
                .as_deref()
                .expect("Retail gear requires Retail scaling tables"),
            shared::item_data::ItemDefinitionSource::Forever70205 => {
                &self
                    .forever_tables
                    .as_deref()
                    .expect("Forever70205 gear requires Forever70205 scaling tables")
                    .0
            }
        }
    }

    /// Stat blocks of every equipped item and the main- and off-hand weapons.
    fn gear(
        &self,
        equipped: Option<&EquippedItems>,
        modifiers: Option<&EquippedItemModifiers>,
        level: u8,
    ) -> Gear {
        let mut gear = Gear::default();
        let Some(equipped) = equipped else {
            return gear;
        };
        if equipped.items.is_empty() {
            return gear;
        }
        let templates = self
            .templates
            .as_deref()
            .expect("equipped items require definition catalogs");
        for &(slot, source, item_id) in &equipped.items {
            let template = templates
                .get(source, item_id)
                .unwrap_or_else(|| panic!("equipped item {item_id} has no {source:?} definition"));
            let properties = modifiers.and_then(|modifiers| {
                modifiers
                    .items
                    .iter()
                    .find(|(key, _)| *key == slot)
                    .map(|(_, state)| state)
            });
            let stats = self.equipped_stats(template, source, properties, level);
            gear.apply_item(slot, template, stats);
        }
        gear
    }

    fn equipped_stats(
        &self,
        template: &crate::item_templates::ItemTemplate,
        source: shared::item_data::ItemDefinitionSource,
        properties: Option<&server::item_instance::ItemInstanceModifiers>,
        level: u8,
    ) -> crate::item_stats::ItemStats {
        let tables = self.scaling_tables(source);
        let Some(properties) = properties else {
            return item_stats(template, tables);
        };
        assert_eq!(
            source,
            shared::item_data::ItemDefinitionSource::Retail,
            "Retail instance modifiers cannot use another product's item tables"
        );
        let native = self
            .instances
            .as_deref()
            .expect("item instance modifiers require native bonus/enchantment tables");
        compute_native_instance_stats(template, properties, level, native, tables)
    }
}

fn compute_native_instance_stats(
    template: &crate::item_templates::ItemTemplate,
    properties: &server::item_instance::ItemInstanceModifiers,
    level: u8,
    native: &NativeItemInstances,
    tables: &ItemScalingTables,
) -> crate::item_stats::ItemStats {
    let modified = server::item_instance::bonuses::apply_item_bonuses(
        &template.scaling,
        &properties.bonus_list,
        &native.bonuses,
    )
    .unwrap_or_else(|error| panic!("resolve equipped item bonuses: {error}"));
    let mut stats = server::item_instance::item_instance_stats(
        &modified.scaling,
        template.inventory_type,
        properties,
        tables,
    )
    .unwrap_or_else(|error| panic!("resolve equipped item stats: {error}"));
    let mut blocks = vec![stats.block];
    blocks.extend(instance_enchant_blocks(properties, level, &native.enchants));
    let (primary, secondary) = sum_equipment_stats(&blocks);
    stats.block = ItemStatBlock { primary, secondary };
    stats
}

fn instance_enchant_blocks(
    properties: &server::item_instance::ItemInstanceModifiers,
    level: u8,
    tables: &server::item_instance::enchants::EnchantmentTables,
) -> Vec<ItemStatBlock> {
    use server::item_instance::enchants::{enchantment_stats, gem_stats};
    let enchant = enchantment_stats(properties.enchant, level, tables)
        .unwrap_or_else(|error| panic!("resolve equipped enchant: {error}"));
    let mut blocks = vec![enchant.stats];
    for &gem in &properties.gems {
        let contribution = gem_stats(gem, level, tables)
            .unwrap_or_else(|error| panic!("resolve equipped gem {gem}: {error}"));
        blocks.push(contribution.stats);
    }
    blocks
}

#[derive(Default)]
struct Gear {
    blocks: Vec<ItemStatBlock>,
    main_hand: WeaponDamage,
    off_hand: Option<WeaponDamage>,
    has_weapon: bool,
    shield_armor: Option<f32>,
    health_regen: f32,
}

impl Gear {
    fn apply_item(
        &mut self,
        slot: EquipmentSlot,
        template: &crate::item_templates::ItemTemplate,
        stats: crate::item_stats::ItemStats,
    ) {
        self.blocks.push(stats.block);
        self.health_regen += stats.health_regen;
        if template.inventory_type == INVTYPE_SHIELD && slot == EquipmentSlot::OffHand {
            self.shield_armor = Some(stats.block.secondary.armor);
        }
        match (slot, stats.weapon) {
            (EquipmentSlot::MainHand, Some(weapon)) => {
                self.main_hand = weapon;
                self.has_weapon = true;
            }
            (EquipmentSlot::OffHand, Some(weapon)) => {
                self.off_hand = Some(weapon);
                self.has_weapon = true;
            }
            _ => {}
        }
    }
}

/// `InventoryType` Shield.
const INVTYPE_SHIELD: u8 = 14;

/// Compute new current value preserving the current-to-max ratio.
///
/// A deliberate deviation from TrinityCore, whose `Unit::SetMaxHealth` only clamps the
/// current value (and whose login sets `min(saved, max)`, Player.cpp:18736): every
/// max-health or max-mana change here keeps the percentage, at login and in session.
/// Retail keeps the percentage when maximum health changes in session (TrinityCore
/// does too for MOD_INCREASE_HEALTH_PERCENT), and characters saved with the old
/// 100-health placeholder rely on the ratio to log in at their saved share.
/// Truncated to whole units as TrinityCore's integer health and power fields
/// (`CalculatePct(GetMaxHealth(), percent)` into `uint32`, SpellAuraEffects.cpp:4204).
fn rescaled_current(current: f32, old_max: f32, new_max: f32) -> f32 {
    // Multiply first: whole-unit products are exact, so an exact share stays whole.
    let scaled = if old_max > 0.0 {
        current * new_max / old_max
    } else {
        new_max
    };
    scaled.min(new_max).trunc()
}

type StatsQuery = (
    Entity,
    Option<&'static Player>,
    Option<&'static crate::followers::Follower>,
    &'static Level,
    Option<&'static ActiveSpec>,
    Option<&'static EquippedItems>,
    Option<&'static EquippedItemModifiers>,
    Option<&'static KnownSpells>,
    Option<&'static StatModifiers>,
    Option<&'static CharacterStats>,
    &'static mut UnitStats,
    &'static mut CombatRatings,
    &'static mut DerivedStats,
    &'static mut Health,
    Option<&'static mut Mana>,
);

type StatsFilter = (
    Or<(With<Player>, With<crate::followers::Follower>)>,
    Or<(
        Added<Player>,
        Changed<crate::followers::Follower>,
        Changed<Level>,
        Changed<ActiveSpec>,
        Changed<EquippedItems>,
        Changed<EquippedItemModifiers>,
        Changed<KnownSpells>,
        Changed<StatModifiers>,
    )>,
);

/// Followers retain the bridge's source-neutral race0 and labelled spec. They
/// remain NPCs; no synthetic Player or persistent-character identity is inserted.
fn stat_identity(
    player: Option<&Player>,
    follower: Option<&crate::followers::Follower>,
    spec: Option<&ActiveSpec>,
) -> Option<(u8, u8, Option<u32>)> {
    match (player, follower) {
        (Some(player), _) => Some((player.class, player.race, spec.map(|spec| spec.0))),
        (None, Some(follower)) => {
            Some((follower.profile.class_id, 0, Some(follower.profile.spec_id)))
        }
        (None, None) => None,
    }
}

/// Recompute `CharacterStats` and write stats, health and mana maxima for
/// players and approved NPC followers through the same modifier/ratio path.
pub(crate) fn update_character_stats(
    mut commands: Commands,
    mut characters: Query<StatsQuery, StatsFilter>,
    classes: Option<Res<ClassSpells>>,
    spells: Option<Res<SpellStore>>,
    items: ItemData,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
) {
    for (
        entity,
        player,
        follower,
        level,
        spec,
        equipped,
        instance_modifiers,
        known,
        mods,
        previous_stats,
        mut unit_stats,
        mut ratings,
        mut derived,
        mut health,
        mana,
    ) in &mut characters
    {
        let Some((class_id, race, spec_id)) = stat_identity(player, follower, spec) else {
            continue;
        };
        let class = class_info(classes.as_deref(), class_id);
        let (primary_stat, mastery_coefficient) = spec_inputs(class, spec_id, spells.as_deref());
        let gear = items.gear(equipped, instance_modifiers, level.0);
        let (gear_primary, gear_ratings) = sum_equipment_stats(&gear.blocks);
        let input = StatInputs {
            class: class_id,
            race,
            level: level.0,
            primary_stat,
            mastery_coefficient,
            attack_power_per_strength: class.map_or(0.0, |class| class.attack_power_per_strength),
            attack_power_per_agility: class.map_or(0.0, |class| class.attack_power_per_agility),
            ranged_attack_power_per_agility: class
                .map_or(0.0, |class| class.ranged_attack_power_per_agility),
            gear_primary,
            gear_ratings,
        };
        let Some(mut stats) = compute_character_stats(&input, mods) else {
            warn!(
                "no stat data for class {} level {}; stats unchanged",
                class_id, level.0
            );
            continue;
        };
        stats.main_hand = gear.main_hand;
        stats.off_hand = gear.off_hand;
        stats.health_regen = gear.health_regen;
        stats.defense = player_defense(
            class_id,
            level.0,
            &stats.ratings,
            DefenseInputs {
                can_parry: knows_effect(known, spells.as_deref(), EFFECT_PARRY),
                can_block: knows_effect(known, spells.as_deref(), EFFECT_BLOCK),
                has_weapon: gear.has_weapon,
                shield_armor: gear.shield_armor,
                dodge_pct: mods.map_or(0.0, |mods| mods.dodge_pct),
                parry_pct: mods.map_or(0.0, |mods| mods.parry_pct),
                block_pct: mods.map_or(0.0, |mods| mods.block_pct),
            },
        );
        if *unit_stats != stats.primary {
            debug!(
                "{entity} primary stats {:?} -> {:?}, spell power {}",
                *unit_stats, stats.primary, stats.spell_power
            );
        }
        let mut changed = unit_stats.set_if_neq(stats.primary);
        changed |= ratings.set_if_neq(stats.ratings);
        changed |= derived.set_if_neq(stats.derived());
        changed |= health.set_if_neq(Health {
            current: rescaled_current(health.current, health.max, stats.max_health),
            max: stats.max_health,
        });
        if let Some(mut mana) = mana {
            // Aura419 changes BASE_PCT, not create mana. SetMaxPower clamps
            // current power instead of granting/draining a proportional amount.
            let aura_maximum_changed = previous_stats.is_some_and(|previous| {
                previous.base_mana == stats.base_mana && previous.max_mana != stats.max_mana
            });
            let current = if aura_maximum_changed {
                mana.current.min(stats.max_mana)
            } else {
                rescaled_current(mana.current, mana.max, stats.max_mana)
            };
            changed |= mana.set_if_neq(Mana {
                current,
                max: stats.max_mana,
            });
        }
        commands.entity(entity).insert(stats);
        if changed && let Some(dirty) = replication_dirty.as_deref() {
            dirty.mark(entity);
        }
    }
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "stats_instance_tests.rs"]
mod instance_tests;
