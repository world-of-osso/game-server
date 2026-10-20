//! TC a352b1fa KillRewarder.cpp159, Player.cpp14964–14967, SpellAuraDefines.h435.
use super::{ActiveAuras, handlers};

fn amounts(auras: Option<&ActiveAuras>, kind: u32) -> impl Iterator<Item = f32> {
    auras
        .into_iter()
        .flat_map(ActiveAuras::iter)
        .flat_map(move |aura| {
            aura.effects
                .iter()
                .filter(move |e| e.aura_type == kind)
                .map(|e| e.amount * aura.stack_multiplier())
        })
}

pub(crate) fn kill_xp(base: u32, auras: Option<&ActiveAuras>) -> u32 {
    let multiplier: f32 = amounts(auras, handlers::MOD_XP_PCT)
        .map(|amount| 1.0 + amount / 100.0)
        .product();
    (base as f32 * multiplier) as u32
}

pub(crate) fn quest_xp(base: u32, auras: Option<&ActiveAuras>) -> u32 {
    amounts(auras, handlers::MOD_XP_QUEST_PCT).fold(base, |xp, amount| {
        (xp as f32 * (1.0 + amount / 100.0)) as u32
    })
}

/// MiscValue0 selects the source; bonuses add, unlike the XP multiplier product.
pub(crate) fn money(base: u64, auras: Option<&ActiveAuras>, source: i32) -> u64 {
    let points: f64 = auras
        .into_iter()
        .flat_map(ActiveAuras::iter)
        .map(|aura| {
            aura.effects
                .iter()
                .filter(|e| e.aura_type == handlers::MOD_MONEY_GAIN && e.misc_values[0] == source)
                .map(|e| f64::from(e.amount * aura.stack_multiplier()))
                .sum::<f64>()
        })
        .sum();
    (base as f64 * (1.0 + points / 100.0)) as u64
}
