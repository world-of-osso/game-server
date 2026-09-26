//! Player character stats (retail 12.x model).
//!
//! `compute_character_stats` turns class/race/level base stats, gear and
//! optional aura `StatModifiers` into `CharacterStats`. `update_character_stats`
//! applies the result on world entry and whenever level, spec, gear or
//! modifiers change: it writes the effective `UnitStats` / `CombatRatings`,
//! `Health.max` and `Mana.max`, rescaling current values by their ratio.
//! Sources and formulas: `docs/wiki/systems/character-stats.md`.

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{CombatRatings, Health, ItemStatBlock, Level, Mana, Player, UnitStats};
use shared::formulas::sum_equipment_stats;
use shared::protocol::EquipmentSlot;

use crate::character_stats_data::{
    BASE_MANA, CLASS_BASE_STATS, CRIT_RATING, HASTE_RATING, HP_PER_STAMINA, MASTERY_RATING,
    RACE_STAT_MODS, VERSATILITY_RATING,
};
use crate::class_progression::ActiveSpec;
use crate::item_stats::{ItemScalingTables, WeaponDamage, item_stats};
use crate::item_templates::ItemTemplates;
use crate::spell_info::{ClassInfo, ClassSpells, SpellStore};

/// Every player has 5% base crit (SimC `dbc_t::all_crit_base`).
const BASE_CRIT_PCT: f32 = 5.0;
/// Every player has 8 base mastery points (SimC `player_t::init_base_stats`).
const BASE_MASTERY_POINTS: f32 = 8.0;

/// Item ids physically equipped, by slot. The inventory sets it on world
/// entry and after every item change.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct EquippedItems {
    pub items: Vec<(EquipmentSlot, u32)>,
}

/// Aura stat changes, folded in after base + gear. Absent means none.
/// Percent fields are additive with each other (10.0 = +10%).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct StatModifiers {
    /// Added to base + gear primary stats.
    pub primary_flat: UnitStats,
    /// Percent increase per primary stat, applied after `primary_flat`.
    pub primary_pct: UnitStats,
    /// Added to gear ratings before conversion.
    pub ratings_flat: CombatRatings,
    pub crit_pct: f32,
    pub haste_pct: f32,
    pub mastery_points: f32,
    pub versatility_pct: f32,
    pub max_health_pct: f32,
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
    /// gtBaseMP; 0 for classes without mana. Max mana equals it and
    /// `PowerCostPct` mana costs are a percentage of it.
    pub base_mana: f32,
    /// Intellect when the main stat is Intellect, else 0
    /// (Unit::SpellBaseDamageBonusDone, Unit.cpp:7118-7119).
    pub spell_power: f32,
    pub attack_power: f32,
    pub crit_pct: f32,
    pub haste_pct: f32,
    pub mastery_pct: f32,
    pub versatility_pct: f32,
    pub armor: f32,
    /// Main-hand swing data; unarmed without a weapon.
    pub main_hand: WeaponDamage,
    /// An equipped off-hand weapon swings too.
    pub off_hand: Option<WeaponDamage>,
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

/// Piecewise-linear lookup, clamped to the curve's ends.
fn secondary_dr(value: f32) -> f32 {
    let upper = SECONDARY_DR_CURVE
        .iter()
        .position(|&(x, _)| x >= value)
        .unwrap_or(SECONDARY_DR_CURVE.len() - 1);
    if upper == 0 || value >= SECONDARY_DR_CURVE[upper].0 {
        return SECONDARY_DR_CURVE[upper].1;
    }
    let (x0, y0) = SECONDARY_DR_CURVE[upper - 1];
    let (x1, y1) = SECONDARY_DR_CURVE[upper];
    y0 + (value - x0) / (x1 - x0) * (y1 - y0)
}

/// Rating → percent (mastery: points) after DR. `per_point` is the
/// gtCombatRatings value: rating for 1% or 1 mastery point (SimC `rating_t::init`).
fn rating_value(rating: f32, per_point: f32) -> f32 {
    secondary_dr(rating / per_point)
}

/// Retail stats for `input`, with aura `mods` folded in. `None` for a class
/// or level outside the tables (1..=13, 1..=90).
pub fn compute_character_stats(
    input: &StatInputs,
    mods: Option<&StatModifiers>,
) -> Option<CharacterStats> {
    let level = level_index(input.level)?;
    let mods = mods.copied().unwrap_or_default();
    let mut primary = base_primary_stats(input.class, input.race, input.level)?;
    primary += usable_gear_primary(input.gear_primary, input.primary_stat);
    primary += mods.primary_flat;
    let primary = apply_primary_pct(primary, mods.primary_pct);
    let mut ratings = input.gear_ratings;
    ratings += mods.ratings_flat;
    let mastery_points = BASE_MASTERY_POINTS
        + rating_value(ratings.mastery, MASTERY_RATING[level])
        + mods.mastery_points;
    Some(CharacterStats {
        primary,
        ratings,
        max_health: (primary.stamina.floor()
            * HP_PER_STAMINA[level]
            * (1.0 + mods.max_health_pct / 100.0))
            .floor(),
        base_mana: base_mana(input.class, input.level).unwrap_or(0.0),
        spell_power: if input.primary_stat == Some(PrimaryStat::Intellect) {
            primary.intellect.max(0.0)
        } else {
            0.0
        },
        attack_power: primary.strength * input.attack_power_per_strength
            + primary.agility * input.attack_power_per_agility,
        crit_pct: BASE_CRIT_PCT + rating_value(ratings.crit, CRIT_RATING[level]) + mods.crit_pct,
        haste_pct: rating_value(ratings.haste, HASTE_RATING[level]) + mods.haste_pct,
        mastery_pct: mastery_points * input.mastery_coefficient,
        versatility_pct: rating_value(ratings.versatility, VERSATILITY_RATING[level])
            + mods.versatility_pct,
        armor: ratings.armor,
        main_hand: WeaponDamage::UNARMED,
        off_hand: None,
    })
}

fn class_info(classes: Option<&ClassSpells>, class: u8) -> Option<&ClassInfo> {
    classes?.classes.iter().find(|info| info.class_id == class)
}

/// Main stat and mastery coefficient of the active spec; without one, the
/// class main stat and no mastery.
fn spec_inputs(
    class: Option<&ClassInfo>,
    spec: Option<&ActiveSpec>,
    spells: Option<&SpellStore>,
) -> (Option<PrimaryStat>, f32) {
    let Some(spec) = spec.and_then(|spec| class?.specs.iter().find(|info| info.spec_id == spec.0))
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
}

impl ItemData<'_> {
    /// Stat blocks of every equipped item and the main- and off-hand weapons.
    fn gear(&self, equipped: Option<&EquippedItems>) -> Gear {
        let mut gear = Gear {
            blocks: Vec::new(),
            main_hand: WeaponDamage::UNARMED,
            off_hand: None,
        };
        let (Some(templates), Some(tables), Some(equipped)) =
            (self.templates.as_deref(), self.tables.as_deref(), equipped)
        else {
            return gear;
        };
        for &(slot, item_id) in &equipped.items {
            let Some(template) = templates.get(item_id) else {
                continue;
            };
            let stats = item_stats(template, tables);
            gear.blocks.push(stats.block);
            match (slot, stats.weapon) {
                (EquipmentSlot::MainHand, Some(weapon)) => gear.main_hand = weapon,
                (EquipmentSlot::OffHand, Some(weapon)) => gear.off_hand = Some(weapon),
                _ => {}
            }
        }
        gear
    }
}

struct Gear {
    blocks: Vec<ItemStatBlock>,
    main_hand: WeaponDamage,
    off_hand: Option<WeaponDamage>,
}

/// Compute new current value preserving the current-to-max ratio.
fn rescaled_current(current: f32, old_max: f32, new_max: f32) -> f32 {
    let ratio = if old_max > 0.0 {
        current / old_max
    } else {
        1.0
    };
    (new_max * ratio).min(new_max)
}

type StatsQuery = (
    Entity,
    &'static Player,
    &'static Level,
    Option<&'static ActiveSpec>,
    Option<&'static EquippedItems>,
    Option<&'static StatModifiers>,
    &'static mut UnitStats,
    &'static mut CombatRatings,
    &'static mut Health,
    Option<&'static mut Mana>,
);

type StatsFilter = Or<(
    Added<Player>,
    Changed<Level>,
    Changed<ActiveSpec>,
    Changed<EquippedItems>,
    Changed<StatModifiers>,
)>;

/// Recompute `CharacterStats` and write stats, health and mana maxima.
pub(crate) fn update_character_stats(
    mut commands: Commands,
    mut players: Query<StatsQuery, StatsFilter>,
    classes: Option<Res<ClassSpells>>,
    spells: Option<Res<SpellStore>>,
    items: ItemData,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
) {
    for (
        entity,
        player,
        level,
        spec,
        equipped,
        mods,
        mut unit_stats,
        mut ratings,
        mut health,
        mana,
    ) in &mut players
    {
        let class = class_info(classes.as_deref(), player.class);
        let (primary_stat, mastery_coefficient) = spec_inputs(class, spec, spells.as_deref());
        let gear = items.gear(equipped);
        let (gear_primary, gear_ratings) = sum_equipment_stats(&gear.blocks);
        let input = StatInputs {
            class: player.class,
            race: player.race,
            level: level.0,
            primary_stat,
            mastery_coefficient,
            attack_power_per_strength: class.map_or(0.0, |class| class.attack_power_per_strength),
            attack_power_per_agility: class.map_or(0.0, |class| class.attack_power_per_agility),
            gear_primary,
            gear_ratings,
        };
        let Some(mut stats) = compute_character_stats(&input, mods) else {
            warn!(
                "no stat data for class {} level {}; stats unchanged",
                player.class, level.0
            );
            continue;
        };
        stats.main_hand = gear.main_hand;
        stats.off_hand = gear.off_hand;
        let mut changed = unit_stats.set_if_neq(stats.primary);
        changed |= ratings.set_if_neq(stats.ratings);
        changed |= health.set_if_neq(Health {
            current: rescaled_current(health.current, health.max, stats.max_health),
            max: stats.max_health,
        });
        if let Some(mut mana) = mana {
            changed |= mana.set_if_neq(Mana {
                current: rescaled_current(mana.current, mana.max, stats.base_mana),
                max: stats.base_mana,
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
