//! Native70448 Spiritfont and Courage of the White Tiger on the selected 270 snapshot party.
use super::healer_simc_replay::{ClassParty, hold, snapshot_party};
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use bevy::prelude::Entity;
use shared::components::Health;
use shared::protocol::{CombatLogEvent, CombatLogKind};

fn advance_until(run: &mut ClassParty, done: impl Fn(&ClassParty) -> bool) {
    for _ in 0..600 {
        if done(run) {
            return;
        }
        run.app.update();
    }
    panic!("condition not reached within 600 updates");
}

fn own_aura(run: &ClassParty, unit: Entity, spell: u32) -> Option<crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<ActiveAuras>(unit)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.healer)
        .cloned()
}

fn set_health_fraction(run: &mut ClassParty, unit: Entity, fraction: f32) {
    let mut health = run.app.world_mut().get_mut::<Health>(unit).unwrap();
    health.current = health.max * fraction;
}

fn events_since(run: &ClassParty, start: usize) -> Vec<CombatLogEvent> {
    run.app.world().resource::<ObservedCombatEvents>().0[start..].to_vec()
}

fn raw_heal(event: &CombatLogEvent) -> f32 {
    (event.amount + event.overflow) as f32
}

fn settle_held_healer() -> ClassParty {
    let mut run = snapshot_party(270, 90, 1);
    let healer = run.healer;
    hold(&mut run, healer);
    for _ in 0..3 {
        run.app.update();
    }
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .crit_pct = 0.0;
    run
}

/// Cast once the previous cast's global cooldown or cast bar has ended.
fn cast(run: &mut ClassParty, spell: u32, target: Entity) {
    use crate::spell_cast::CastFailure;
    for _ in 0..200 {
        let healer = run.healer;
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            healer,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => return,
            Err(CastFailure::OnGlobalCooldown | CastFailure::InProgress) => run.app.update(),
            Err(failure) => panic!("cast {spell} failed: {failure:?}"),
        }
    }
    panic!("cast {spell} never left the global cooldown");
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn mistweaver_spiritfont_pools_mists_moves_them_and_cocoons_targets() {
    let mut run = settle_held_healer();
    let healer = run.healer;
    let [tank, _, ally, other, leader] = run.slots;
    for talent in [1260511, 1260680] {
        assert!(
            own_aura(&run, healer, talent).is_some(),
            "selected {talent}"
        );
    }
    // 1260680 names no class mask; only Thunder Focus Tea grants Spiritfont.
    cast(&mut run, 115151, tank);
    for _ in 0..5 {
        run.app.update();
    }
    assert!(own_aura(&run, healer, 1260565).is_none());
    cast(&mut run, 116680, healer);
    advance_until(&mut run, |run| own_aura(run, healer, 1260565).is_some());

    for (unit, fraction) in [(tank, 0.1), (ally, 0.5), (other, 0.5)] {
        set_health_fraction(&mut run, unit, fraction);
    }
    cast(&mut run, 124682, tank);
    advance_until(&mut run, |run| own_aura(run, healer, 1260670).is_some());
    assert!(
        own_aura(&run, healer, 1260565).is_none(),
        "Enveloping Mist spends it"
    );
    let misted: Vec<_> = run
        .slots
        .into_iter()
        .filter(|&unit| own_aura(&run, unit, 1260617).is_some())
        .collect();
    assert_eq!(misted, vec![tank, ally, other], "only the injured allies");

    // DescriptionVariables 813 `$newshield` at 1260680 s1 (30%) effectiveness.
    let world = run.app.world();
    let max = world.get::<Health>(healer).unwrap().max;
    let versatility = world
        .get::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .versatility_pct;
    let percent = crate::spell_cast::scripts::script_effect_value(world, healer, 1260681, 1);
    let expected = max * percent / 100.0 * (1.0 + versatility / 100.0) * 0.3;
    for unit in [tank, ally, other] {
        let cocoon = own_aura(&run, unit, 1260681).expect("Spiritfont cocoon");
        let amount = cocoon.effects.iter().find(|e| e.index == 0).unwrap().amount;
        assert!(
            (amount - expected).abs() <= 1.0,
            "{amount} versus {expected}"
        );
    }

    let heal_on = |events: &[CombatLogEvent], unit: Entity| {
        events
            .iter()
            .find(|e| {
                e.kind == CombatLogKind::Heal
                    && e.spell_id == Some(1260617)
                    && e.target == Some(unit.to_bits())
            })
            .map(raw_heal)
    };
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    advance_until(&mut run, |run| {
        heal_on(&events_since(run, start), ally).is_some()
    });
    let three_mists = heal_on(&events_since(&run, start), ally).unwrap();

    // A full target's mist ends at its next tick; with no injured candidate the pool splits.
    set_health_fraction(&mut run, other, 1.0);
    advance_until(&mut run, |run| own_aura(run, other, 1260617).is_none());
    assert!(own_aura(&run, ally, 1260617).is_some());
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    advance_until(&mut run, |run| {
        heal_on(&events_since(run, start), ally).is_some()
    });
    let two_mists = heal_on(&events_since(&run, start), ally).unwrap();
    assert!(
        (two_mists / three_mists - 1.5).abs() < 0.02,
        "5/2 versus 5/3 of the tick: {two_mists} after {three_mists}"
    );

    // With an injured candidate the mist moves and cocoons its new target.
    set_health_fraction(&mut run, leader, 0.5);
    set_health_fraction(&mut run, tank, 1.0);
    advance_until(&mut run, |run| own_aura(run, tank, 1260617).is_none());
    let moved = own_aura(&run, leader, 1260617).expect("mist moves to the injured leader");
    assert!(own_aura(&run, leader, 1260681).is_some());
    let driver = own_aura(&run, healer, 1260670).unwrap();
    assert!((moved.expires_at.unwrap() - driver.expires_at.unwrap()).abs() < 0.06);
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn mistweaver_courage_claw_heals_an_ally_for_its_damage_after_two_seconds() {
    let mut run = settle_held_healer();
    let healer = run.healer;
    let enemy = run.sources[0];
    assert!(own_aura(&run, healer, 443087).is_some());
    // 443087 s2 100 + Mistweaver Monk 1258138 effect 22's 100% through label 3429.
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), healer, 443087, 1);
    assert_eq!(percent, 200.0);
    for unit in run.slots {
        set_health_fraction(&mut run, unit, 0.2);
    }
    cast(&mut run, 325197, healer);
    advance_until(&mut run, |run| own_aura(run, healer, 460127).is_some());

    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    cast(&mut run, 100780, enemy);
    let cast_at = crate::spell_cast::now(run.app.world());
    run.app.update();
    assert!(
        own_aura(&run, healer, 460127).is_none(),
        "Tiger Palm spends it"
    );
    let claw = |events: &[CombatLogEvent]| {
        events
            .iter()
            .find(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(457917))
            .cloned()
    };
    advance_until(&mut run, |run| claw(&events_since(run, start)).is_some());
    let travel = crate::spell_cast::now(run.app.world()) - cast_at;
    assert!(
        (1.95..=2.2).contains(&travel),
        "claw lands {travel} s later"
    );
    for _ in 0..3 {
        run.app.update();
    }
    let events = events_since(&run, start);
    let claw = claw(&events).unwrap();
    assert_eq!(claw.target, Some(enemy.to_bits()));
    let heals: Vec<_> = events
        .iter()
        .filter(|e| e.kind == CombatLogKind::Heal && e.spell_id == Some(443106))
        .collect();
    assert_eq!(heals.len(), 1, "one ally: {heals:?}");
    assert!(
        run.slots
            .iter()
            .any(|unit| Some(unit.to_bits()) == heals[0].target)
    );
    let ratio = raw_heal(heals[0]) / claw.amount as f32;
    assert!(
        (ratio - 2.0).abs() < 0.01,
        "s2 200% of post-mitigation damage, no healing modifiers: {ratio}"
    );
}
