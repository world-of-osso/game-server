//! Real producer hits/ticks qualify the pinned PRD/RPPM streams.
use super::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};

fn is_owned_damage(event: &CombatLogEvent, caster: Entity, spell: u32) -> bool {
    let owned_spell = event.source == Some(caster.to_bits()) && event.spell_id == Some(spell);
    let damage = event.kind == CombatLogKind::Damage && event.amount > 0;
    owned_spell && damage
}

fn owned_damage_hits(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| is_owned_damage(event, run.actor, spell))
        .count()
}

fn remove_owned_aura(run: &mut Run, spell: u32) {
    let actor = run.actor;
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.caster == actor && aura.spell_id == spell,
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_procs_real_bolts_and_grasp_ticks_earn_pinned_cunning_volley() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, 453172));
    assert!(
        !run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(caster)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == 198590)
    );
    for _ in 0..40 {
        crate::spell_cast::creature_cast(run.app.world_mut(), caster, 686, Some(target), false)
            .unwrap();
        for _ in 0..70 {
            run.app.update();
        }
    }
    assert_eq!(owned_damage_hits(&run, 686), 40);
    let bolts = owned_damage_hits(&run, 453176);
    // SimC76f8118 init1232–1235/hpp1066–1067: SB-selected50% PRD,
    // not talent effect5 (which is the native Volley enemy cap).
    assert!(
        (12..=28).contains(&bolts),
        "40 actual Bolts must earn the pinned50% stream: {bolts}"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 205180, Some(caster), 0);
    for _ in 0..5 {
        crate::spell_cast::creature_cast(run.app.world_mut(), caster, 1261153, Some(target), false)
            .unwrap();
        for _ in 0..80 {
            run.app.update();
        }
    }
    let ticks = owned_damage_hits(&run, 1261153);
    let channel_volleys = owned_damage_hits(&run, 453176) - bolts;
    assert!(
        ticks >= 16,
        "actual native Grasp damage ticks are required: {ticks}"
    );
    assert!(
        channel_volleys as f64 >= ticks as f64 * 0.2
            && channel_volleys as f64 <= ticks as f64 * 0.8,
        "actual Grasp ticks share the selected stream: ticks={ticks}, volleys={channel_volleys}"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_procs_actual_critical_ua_ticks_earn_selected_ravenous_nightfall() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, 459440));
    remove_owned_aura(&mut run, 108558);
    remove_owned_aura(&mut run, 264571);
    let mut earned = 0;
    for _ in 0..30 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 1259790, Some(target), 0);
        for _ in 0..140 {
            let before = run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            run.app.update();
            if crate::auras::has_aura(run.app.world(), caster, 264571) {
                let events = &run
                    .app
                    .world()
                    .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                    .0;
                assert!(
                    events[before..]
                        .iter()
                        .any(|event| is_owned_damage(event, caster, 1259790)
                            && event.periodic
                            && event.crit),
                    "a new Ravenous Nightfall must come from an actual critical eligible tick"
                );
                earned += 1;
                remove_owned_aura(&mut run, 264571);
            }
        }
    }
    // SimC76f8118 actions459–467/init1227: selected Ravenous RPPM,
    // with base Corruption/Wither Nightfall disabled rather than a fake chance.
    assert!(
        earned > 0,
        "real critical native UA ticks must earn Ravenous Nightfall"
    );
    remove_owned_aura(&mut run, 459440);
    for _ in 0..6 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 1259790, Some(target), 0);
        for _ in 0..140 {
            run.app.update();
            assert!(
                !crate::auras::has_aura(run.app.world(), caster, 264571),
                "unselected Ravenous cannot earn Nightfall from continuing real ticks"
            );
        }
    }
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_procs_grasp_uses_actual_corruption_periodic_child_without_wither() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    remove_owned_aura(&mut run, 445465);
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 172, Some(target), 0);
    for _ in 0..20 {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), target, 146739),
        "real Corruption172 must attach native periodic146739"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, 205180, Some(caster), 0);
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, 1261153, Some(target), false)
        .unwrap();
    for _ in 0..80 {
        run.app.update();
    }
    assert!(owned_damage_hits(&run, 1261153) > 0);
    assert!(
        owned_damage_hits(&run, 1261158) > 0,
        "real Grasp ticks must copy actual owned Corruption146739 into native1261158"
    );
    assert_eq!(
        owned_damage_hits(&run, 1279686),
        0,
        "no Wither hero means no Wither child"
    );
}
