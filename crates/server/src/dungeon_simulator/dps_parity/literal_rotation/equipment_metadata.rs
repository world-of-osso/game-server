//! Reachable native item auras and TC SpellHistory.cpp1119–1144 cooldown sources.
use crate::auras::handlers;
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};
use server::item_instance::effects::ItemEffect;
use std::collections::{BTreeMap, BTreeSet};

const ON_USE: u8 = 0;
const ON_EQUIP: u8 = 1;
const DUMMY_EFFECT: u32 = 3;
const PRIMARY_STATS: &[(i32, &str)] = &[(0, "strength"), (1, "other"), (2, "other"), (3, "other")];
// TC Unit.h combat-rating masks; utility-only ratings do not qualify as any_dps.
const CRIT: u32 = 0x700;
const HASTE: u32 = 0xe0000;
const MASTERY: u32 = 1 << 25;
const VERSATILITY: u32 = 0x7000_0000;
const UTILITY_RATINGS: u32 = (1 << 13) | (1 << 16) | (1 << 20);

#[derive(Clone, Default)]
struct Buffs {
    stats: BTreeSet<&'static str>,
    duration: f64,
}

impl Buffs {
    fn merge(&mut self, other: &Self) {
        self.stats.extend(&other.stats);
        self.duration = self.duration.max(other.duration);
    }
}

pub(super) struct Cooldown {
    pub spell: SpellInfo,
    pub proc: bool,
    pub duration: f64,
}

#[derive(Default)]
pub(super) struct Metadata {
    pub stats: BTreeSet<&'static str>,
    pub duration: f64,
    pub use_buff: bool,
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
            let buffs = read_buffs(store, effect.spell_id, &mut active, &mut completed)?;
            metadata.stats.extend(&buffs.stats);
            metadata.duration = metadata.duration.max(buffs.duration);
            metadata.use_buff |= effect.trigger == ON_USE && !buffs.stats.is_empty();
            let cooldown = read_cooldown(store, effect)?;
            if cooldown.duration > 0.0 {
                metadata.cooldowns.push(cooldown);
            }
        }
        Ok(metadata)
    }
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
            .duration
            .map_or(0.0, |row| f64::from(row.duration_ms.max(0)) / 1000.0)
    };
    Buffs { stats, duration }
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
                effect.misc_values[1] == 0 || effect.misc_values[1] & (1 << index) != 0
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
    if !proc && (effect.cooldown_ms >= 0 || effect.category_cooldown_ms >= 0) {
        // Native spell values apply only when BOTH item fields are negative.
        // Zero and mixed signs are authoritative, not a per-field fallback.
        let cooldowns = spell.cooldowns.get_or_insert_with(Default::default);
        cooldowns.recovery_ms = effect.cooldown_ms.max(0) as u32;
        cooldowns.category_recovery_ms = effect.category_cooldown_ms.max(0) as u32;
        spell
            .categories
            .get_or_insert_with(Default::default)
            .category = effect.category;
    }
    let duration_ms = if proc {
        spell
            .aura_options
            .as_ref()
            .map_or(0, |row| row.proc_category_recovery_ms.max(0) as u32)
    } else {
        spell.cooldowns.as_ref().map_or(0, |row| {
            row.recovery_ms.max(
                if spell
                    .categories
                    .as_ref()
                    .is_some_and(|row| row.category != 0)
                {
                    row.category_recovery_ms
                } else {
                    0
                },
            )
        })
    };
    Ok(Cooldown {
        spell,
        proc,
        duration: f64::from(duration_ms) / 1000.0,
    })
}

fn read_spell(store: &SpellStore, id: u32) -> Result<&SpellInfo, String> {
    store
        .get(id)
        .ok_or_else(|| format!("native item spell{id} absent"))
}
