//! Reachable native item auras and TC SpellHistory.cpp1119–1144 cooldown sources.
use crate::auras::handlers;
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};
use server::item_instance::effects::ItemEffect;
use std::collections::{BTreeMap, BTreeSet};

const ON_USE: u8 = 0;
const ON_EQUIP: u8 = 1;
const DUMMY_EFFECT: u32 = 3;
const PRIMARY_STATS: &[(i32, &str)] = &[
    (0, "strength"),
    (1, "agility"),
    (2, "other"),
    (3, "intellect"),
];
// TC Unit.h combat-rating masks; utility-only ratings do not qualify as any_dps.
const CRIT: u32 = 0x700;
const HASTE: u32 = 0xe0000;
const MASTERY: u32 = 1 << 25;
const VERSATILITY: u32 = 0x7000_0000;
const UTILITY_RATINGS: u32 = (1 << 13) | (1 << 16) | (1 << 20);

// Pinned SimC76f8118 unique_gear_midnight.cpp: Gaze2020, Vaelgor2747,
// Seastone3533, Vial3567, Heart3748, Font3806. These exact native drivers
// build stat buffs without setting buff.default_value, or are disabled/damage
// drivers with no matching stat buff. buff.hpp327's unset default is -DBL_MIN;
// unique_gear.cpp3776 takes max with0. This is NOT their stat magnitude.
const SIMC_UNSET_PROC_DEFAULT_DRIVERS: &[u32] = &[
    1256896, 1259293, 1260459, 1295058, 1293316, 1295179, 1297760, 1297761, 1297911, 1297908,
];

#[derive(Clone, Default)]
struct Buffs {
    stats: BTreeSet<&'static str>,
    duration: f64,
    damage: bool,
}

impl Buffs {
    fn merge(&mut self, other: &Self) {
        self.stats.extend(&other.stats);
        self.duration = self.duration.max(other.duration);
        self.damage |= other.damage;
    }
}

pub(super) struct Cooldown {
    pub spell: SpellInfo,
    pub proc: bool,
    pub duration: f64,
}

#[derive(Default)]
pub(super) struct Metadata {
    drivers: BTreeSet<u32>,
    pub stats: BTreeSet<&'static str>,
    pub duration: f64,
    pub cast_time: f64,
    pub use_buff: bool,
    pub use_damage: bool,
    pub cooldowns: Vec<Cooldown>,
}

impl Metadata {
    pub fn load(store: &SpellStore, effects: &[ItemEffect]) -> Result<Self, String> {
        let mut metadata = Self::default();
        let mut active = BTreeSet::new();
        let mut completed = BTreeMap::new();
        for effect in effects
            .iter()
            .filter(|effect| matches!(effect.trigger, ON_USE | ON_EQUIP))
        {
            metadata.drivers.insert(effect.spell_id);
            let buffs = read_buffs(store, effect.spell_id, &mut active, &mut completed)?;
            metadata.stats.extend(&buffs.stats);
            metadata.duration = metadata.duration.max(buffs.duration);
            metadata.use_buff |= effect.trigger == ON_USE && !buffs.stats.is_empty();
            metadata.use_damage |= effect.trigger == ON_USE && buffs.damage;
            let cooldown = read_cooldown(store, effect)?;
            if cooldown.duration > 0.0 {
                metadata.cooldowns.push(cooldown);
            }
        }
        metadata.cast_time = read_use_cast_time(store, effects)?;
        Ok(metadata)
    }

    pub fn proc_default_value(&self) -> Result<f64, String> {
        for driver in &self.drivers {
            if !SIMC_UNSET_PROC_DEFAULT_DRIVERS.contains(driver) {
                return Err(format!(
                    "NATIVE_ITEM_PROC_DEFAULT_VALUE_UNSOURCED: native driver{driver} lacks a qualified pinned buff.default_value binding"
                ));
            }
        }
        Ok(0.0)
    }
}

// SimC76f8118 unique_gear.cpp3878–3907: unhasted action time, channel
// duration rather than launch time, and0 when no item use action exists.
fn read_use_cast_time(store: &SpellStore, effects: &[ItemEffect]) -> Result<f64, String> {
    let Some(effect) = effects.iter().find(|effect| effect.trigger == ON_USE) else {
        return Ok(0.0);
    };
    let spell = read_spell(store, effect.spell_id)?;
    let misc = spell
        .misc
        .as_ref()
        .ok_or_else(|| format!("native item spell{} lacks timing metadata", spell.id))?;
    // SpellAttr1 CHANNELED_1/2, same native flags as spell_cast::is_channeled.
    const CHANNELED: u32 = 0x4 | 0x40;
    let milliseconds = if misc.attributes[1] & CHANNELED != 0 {
        misc.duration
            .ok_or_else(|| format!("native item channel{} lacks duration", spell.id))?
            .duration_ms
    } else {
        misc.cast_time.map_or(0, |cast| cast.base_ms)
    };
    if milliseconds < 0 {
        return Err(format!(
            "native item spell{} has nonfinite action duration",
            spell.id
        ));
    }
    Ok(f64::from(milliseconds) / 1000.0)
}

fn read_buffs(
    store: &SpellStore,
    id: u32,
    active: &mut BTreeSet<u32>,
    completed: &mut BTreeMap<u32, Buffs>,
) -> Result<Buffs, String> {
    if let Some(buffs) = completed.get(&id) {
        return Ok(buffs.clone());
    }
    if !active.insert(id) {
        return Err(format!("native item trigger cycle at spell{id}"));
    }
    let spell = read_spell(store, id)?;
    let mut buffs = direct_buffs(spell);
    // DUMMY's TriggerSpell is script input, not a generic cast edge (e.g.
    // Ara-Kara452146 effect1 refers back to its443541 proc driver).
    for effect in &spell.effects {
        let trigger_edge =
            effect.effect != 0 && effect.effect != DUMMY_EFFECT && effect.trigger_spell != 0;
        if trigger_edge {
            buffs.merge(&read_buffs(store, effect.trigger_spell, active, completed)?);
        }
    }
    active.remove(&id);
    completed.insert(id, buffs.clone());
    Ok(buffs)
}

fn direct_buffs(spell: &SpellInfo) -> Buffs {
    let stats: BTreeSet<_> = spell
        .effects
        .iter()
        .filter(|effect| crate::auras::is_aura_effect(effect))
        .flat_map(stat_names)
        .collect();
    let duration = if stats.is_empty() {
        0.0
    } else {
        spell
            .misc
            .as_ref()
            .and_then(|misc| misc.duration)
            .map_or(0.0, |row| f64::from(row.duration_ms.max(0)) / 1000.0)
    };
    Buffs {
        stats,
        duration,
        damage: spell.effects.iter().any(is_damage_effect),
    }
}

// SimC76f8118 special_effect.cpp563–653: offensive/attack auto-parsing.
// Native effect types: SCHOOL_DAMAGE2, HEALTH_LEECH9, WEAPON_PERCENT_DAMAGE31,
// WEAPON_DAMAGE58; periodic DAMAGE3/LEECH8 count only on an actual aura effect.
fn is_damage_effect(effect: &SpellEffect) -> bool {
    matches!(effect.effect, 2 | 9 | 31 | 58)
        || (crate::auras::is_aura_effect(effect) && matches!(effect.aura, 3 | 8))
}

fn stat_names(effect: &SpellEffect) -> Vec<&'static str> {
    match effect.aura {
        handlers::MOD_STAT | handlers::MOD_PERCENT_STAT => PRIMARY_STATS
            .iter()
            .filter(|(index, _)| effect.misc_values[0] < 0 || effect.misc_values[0] == *index)
            .map(|(_, name)| *name)
            .collect(),
        handlers::MOD_TOTAL_STAT_PERCENTAGE => PRIMARY_STATS
            .iter()
            .filter(|(index, _)| {
                effect.misc_values[1] == 0 || effect.misc_values[1] & (1 << *index) != 0
            })
            .map(|(_, name)| *name)
            .collect(),
        handlers::MOD_RATING => rating_names(effect.misc_values[0] as u32),
        _ => Vec::new(),
    }
}

fn rating_names(mask: u32) -> Vec<&'static str> {
    let mut names: Vec<_> = [
        (CRIT, "crit"),
        (HASTE, "haste"),
        (MASTERY, "mastery"),
        (VERSATILITY, "versatility"),
    ]
    .into_iter()
    .filter(|(rating, _)| mask & rating != 0)
    .map(|(_, name)| name)
    .collect();
    if mask & !UTILITY_RATINGS != 0 {
        names.push("other");
    }
    names
}

fn read_cooldown(store: &SpellStore, effect: &ItemEffect) -> Result<Cooldown, String> {
    let mut spell = read_spell(store, effect.spell_id)?.clone();
    let proc = effect.trigger == ON_EQUIP;
    if !proc {
        apply_item_recovery(&mut spell, effect);
    }
    let duration_ms = cooldown_duration_ms(&spell, proc);
    Ok(Cooldown {
        spell,
        proc,
        duration: f64::from(duration_ms) / 1000.0,
    })
}

fn apply_item_recovery(spell: &mut SpellInfo, effect: &ItemEffect) {
    if effect.cooldown_ms < 0 && effect.category_cooldown_ms < 0 {
        return;
    }
    // Zero and mixed signs are authoritative, not a per-field fallback.
    let cooldowns = spell.cooldowns.get_or_insert_with(Default::default);
    cooldowns.recovery_ms = effect.cooldown_ms.max(0) as u32;
    cooldowns.category_recovery_ms = effect.category_cooldown_ms.max(0) as u32;
    spell
        .categories
        .get_or_insert_with(Default::default)
        .category = effect.category;
}

fn cooldown_duration_ms(spell: &SpellInfo, proc: bool) -> u32 {
    if proc {
        spell
            .aura_options
            .as_ref()
            .map_or(0, |row| row.proc_category_recovery_ms.max(0) as u32)
    } else {
        let category_present = spell
            .categories
            .as_ref()
            .is_some_and(|row| row.category != 0);
        spell.cooldowns.as_ref().map_or(0, |row| {
            let category_ms = if category_present {
                row.category_recovery_ms
            } else {
                0
            };
            row.recovery_ms.max(category_ms)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "native ItemEffect source-only cooldown override matrix"]
    fn literal_equipment_native_item_cooldown_zero_and_mixed_signs() {
        let world = super::super::tests::load_world();
        let effects = &world
            .resource::<crate::stats::NativeItemInstances>()
            .effects
            .effects;
        let store = world.resource::<SpellStore>();
        // Exact native rows: zero/positive, negative/positive, zero/negative,
        // zero/zero, negative/zero, both-negative, positive/negative.
        let expected = [
            (45, 60.0),
            (553, 60.0),
            (15477, 0.0),
            (15541, 0.0),
            (31108, 0.0),
            (170101, 120.0),
            (4894, 180.0),
        ];
        let actual: Vec<_> = expected
            .iter()
            .map(|&(id, _)| (id, read_cooldown(store, &effects[&id]).unwrap().duration))
            .collect();
        assert_eq!(actual, expected);
    }
}

fn read_spell(store: &SpellStore, id: u32) -> Result<&SpellInfo, String> {
    store
        .get(id)
        .ok_or_else(|| format!("native item spell{id} absent"))
}
