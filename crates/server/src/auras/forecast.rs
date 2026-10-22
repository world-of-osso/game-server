//! Read-only lifetime of the native stats used by a current melee projection.
use super::*;

/// Live native tick phase and depleted shield pools, without rolling or applying effects.
pub(crate) fn healing_coverage(auras: &ActiveAuras, now: f64, horizon: f64) -> (f32, f32) {
    let mut periodic = 0.0;
    let mut absorb = 0.0;
    for aura in auras
        .iter()
        .filter(|aura| !aura.harmful && aura.expires_at.is_none_or(|expiry| expiry > now))
    {
        let through = aura.expires_at.unwrap_or(now + horizon).min(now + horizon);
        for effect in &aura.effects {
            if effect.aura_type == handlers::SCHOOL_ABSORB {
                absorb += effect.absorb_remaining.max(0.0);
            } else if effect.aura_type == handlers::PERIODIC_HEAL && effect.period_ms > 0 {
                let next = effect.next_tick_at();
                if next <= through {
                    let period = f64::from(effect.period_ms) / 1000.0;
                    let ticks = ((through - next.max(now)) / period).floor() + 1.0;
                    periodic += effect.amount.max(0.0) * aura.stack_multiplier() * ticks as f32;
                }
            }
        }
    }
    (periodic, absorb)
}

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
    if stat_mods::compute(current) != stat_mods::compute(future) {
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
