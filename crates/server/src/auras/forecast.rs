//! Read-only lifetime of the native stats used by a current melee projection.
use super::*;

pub(crate) fn first_melee_stat_expiry(unit: Entity, auras: &ActiveAuras) -> Option<f64> {
    let mut expiries: Vec<_> = auras.iter().filter_map(|aura| aura.expires_at).collect();
    expiries.sort_by(f64::total_cmp);
    expiries.dedup();
    expiries.into_iter().find(|&at| {
        let future = build_unexpired_snapshot(auras, at);
        !same_melee_stats(unit, auras, &future)
    })
}

fn build_unexpired_snapshot(auras: &ActiveAuras, at: f64) -> ActiveAuras {
    ActiveAuras {
        auras: auras
            .iter()
            .filter(|aura| aura.expires_at.is_none_or(|expiry| expiry > at))
            .cloned()
            .collect(),
        ..Default::default()
    }
}

fn same_melee_stats(unit: Entity, current: &ActiveAuras, future: &ActiveAuras) -> bool {
    // Flight branch selection cannot change a melee projection.
    if stat_mods::compute(current, false) != stat_mods::compute(future, false) {
        return false;
    }
    if scripted_stat_signature(unit, current) != scripted_stat_signature(unit, future) {
        return false;
    }
    direct_melee_signature(current) == direct_melee_signature(future)
}

fn scripted_stat_signature(unit: Entity, auras: &ActiveAuras) -> (f32, f32, f32) {
    (
        scripts::fount_max_health_pct(unit, auras),
        scripts::protective_growth_multiplier(unit, auras),
        ebon_might::primary_bonus(auras),
    )
}

fn direct_melee_signature(auras: &ActiveAuras) -> ([f32; 2], [bool; 3]) {
    (
        [
            auras.modifier(handlers::MOD_ATTACKER_MELEE_HIT_CHANCE),
            auras.modifier(handlers::MOD_ATTACKER_MELEE_CRIT_CHANCE),
        ],
        [auras.is_stunned(), auras.is_confused(), auras.is_feared()],
    )
}
