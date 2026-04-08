use super::*;

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
    assert_eq!(
        ticks,
        vec![PeriodicTick::Damage(150.0), PeriodicTick::Damage(150.0)]
    );
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
    auras
        .active
        .push(make_dot_with_interval(100, 1, 100.0, 2.0));
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
            Some(AuraEffect::PeriodicDamage {
                damage: 200.0,
                interval: 3.0,
            }),
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
            Some(AuraEffect::PeriodicDamage {
                damage: 100.0,
                interval: 3.0,
            }),
            None,
            None,
        ],
        tick_interval: haste.tick_interval,
        ..Default::default()
    };
    assert!((aura.tick_interval - 2.0).abs() < 0.001);
    assert_eq!(haste.num_ticks, 6);
}
