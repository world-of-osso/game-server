//! Earned Frost proc relationships through ordinary native casts and weapon swings.
use super::*;
use crate::auras::{self, ActiveAuras, RemoveReason};

fn clear_proc(run: &mut Run, spell: u32) {
    auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        RemoveReason::Consumed,
        |aura| aura.spell_id == spell,
    );
}

fn cast(run: &mut Run, spell: u32) {
    let target = run.targets[0];
    use crate::spell_cast::CastFailure;
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => {
                super::super::preparation::finish_pending_setup(run).unwrap();
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(reason) => panic!("native Frost cast{spell}: {reason:?}"),
        }
    }
    panic!("native Frost cast{spell} never became ready");
}

#[test]
#[ignore = "native251 profile, real Pillar and ordinary Frost Strike"]
fn frost_obliteration_frost_strike_earns_killing_machine() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let rotation = Rotation::load(run.app.world_mut(), run.actor, &run.profile, encounter).unwrap();
    super::super::engage(&mut run);
    cast(&mut run, 49020);
    cast(&mut run, 49020);
    let actor = run.actor;
    super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
    assert!(auras::has_aura(run.app.world(), actor, 281238));
    assert!(auras::has_aura(run.app.world(), actor, 51271));
    clear_proc(&mut run, 51124);
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49143, Some(target), false)
        .unwrap();
    assert!(
        auras::has_aura(run.app.world(), actor, 51124),
        "successful Frost Strike during actual Pillar must earn native Killing Machine"
    );
    let facts = NativeFacts::new(run.app.world(), actor, &rotation.bindings, encounter, 1);
    assert_eq!(
        facts.value("buff.killing_machine.react").unwrap(),
        1.0,
        "a predictable Obliteration proc reacts at acquisition without advancing time"
    );
}

#[test]
#[ignore = "real Frost Strikes earn native Hyperpyrexia residual ticks"]
fn frost_hyperpyrexia_from_real_frost_strikes() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 456238));
    for _ in 0..12 {
        cast(&mut run, 49020);
        cast(&mut run, 49020);
        cast(&mut run, 49143);
    }
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(458169)
            && event.kind == CombatLogKind::Damage
            && event.periodic
            && event.amount > 0),
        "actual Frost Strikes must earn Hyperpyrexia ticks"
    );
}

#[test]
#[ignore = "native251 profile, repeated native-resource-funded Frost Strike"]
fn frost_frost_strikes_roll_native_rime_chance() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 59057));
    let mut procs = 0;
    for _ in 0..24 {
        cast(&mut run, 49020);
        cast(&mut run, 49020);
        clear_proc(&mut run, 59052);
        cast(&mut run, 49143);
        if auras::has_aura(run.app.world(), run.actor, 59052) {
            procs += 1;
        }
    }
    assert!(
        (1..24).contains(&procs),
        "native45% Rime is neither absent nor an unconditional grant: {procs}/24"
    );
}

#[test]
#[ignore = "native251 profile, actual critical white swings"]
fn frost_critical_white_swings_earn_killing_machine() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    cast(&mut run, 49020);
    clear_proc(&mut run, 51124);
    let earned = (0..2000).any(|_| {
        run.app.update();
        run.app
            .world()
            .get::<ActiveAuras>(run.actor)
            .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == 51124))
    });
    assert!(
        earned,
        "real critical white swings must roll Killing Machine PRD"
    );
}

#[test]
#[ignore = "native251 selected Icy Death Torrent follows real critical white swings"]
fn frost_icy_death_torrent_follows_critical_white_swings() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;

    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 3).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 435010));
    cast(&mut run, 49020);
    for _ in 0..2000 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.crit),
        "fixture must produce a real critical white swing"
    );
    for target in &run.targets {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(439539)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "actual critical white swings must earn native Icy Death Torrent on {target:?}"
        );
    }
}

fn icebreaker_rime_damage(selected: bool) -> i32 {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;

    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 392950));
    if !selected {
        clear_proc(&mut run, 392950);
    }
    earn_rime(&mut run);
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    cast(&mut run, 49184);
    run.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .find(|event| {
            event.spell_id == Some(49184)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(run.targets[0].to_bits())
        })
        .expect("earned Rime Howling Blast must hit its actual primary")
        .amount
}

#[test]
#[ignore = "native251 Icebreaker applies its native bonus to earned Rime's primary"]
fn frost_rime_icebreaker_amplifies_primary_howling_blast() {
    let without = icebreaker_rime_damage(false);
    let with = icebreaker_rime_damage(true);
    assert!(
        with as f32 > without as f32 * 1.29 && (with as f32) < without as f32 * 1.31,
        "selected native Icebreaker must add 30% to the earned Rime primary: {without} -> {with}"
    );
}

fn bind_in_darkness_rime_damage(selected: bool) -> i32 {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 440031));
    earn_rime(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 443532));
    if !selected {
        clear_proc(&mut run, 443532);
    }
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    cast(&mut run, 49184);
    run.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .find(|event| {
            event.spell_id == Some(49184)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(run.targets[0].to_bits())
        })
        .expect("earned Rime must hit its primary")
        .amount
}

#[test]
#[ignore = "native251 Bind in Darkness amplifies the earned Rime primary"]
fn frost_rime_bind_in_darkness_amplifies_primary() {
    let without = bind_in_darkness_rime_damage(false);
    let with = bind_in_darkness_rime_damage(true);
    assert!(
        with as f32 > without as f32 * 1.59 && (with as f32) < without as f32 * 1.61,
        "Bind in Darkness must apply its native 60% Rime bonus: {without} -> {with}"
    );
}

#[test]
#[ignore = "native251 actual Pillar and Empower Rune Weapon fund one free rune spender"]
fn frost_erw_pillar_funds_one_free_rune_spender() {
    use crate::spell_cast::runes::Runes;
    for spender in [207230, 49020] {
        let _seed = crate::random::seed_scope(251);
        let mut run = super::super::prepare_literal(251, 3).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
        cast(&mut run, 47568);
        for _ in 0..30 {
            run.app.update();
        }
        assert!(
            auras::has_aura(run.app.world(), actor, 1230959),
            "actual ERW grants the native effect"
        );
        let before = run.app.world().get::<Runes>(actor).unwrap().ready();
        assert!(before >= 2);
        let target = run.targets[0];
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, spender, Some(target), false)
            .expect("ordinary rune spender after real Pillar and ERW");
        let after = run.app.world().get::<Runes>(actor).unwrap().ready();
        assert_eq!(
            after, before,
            "earned ERW must fund spender {spender} without consuming runes"
        );
        assert!(
            !auras::has_aura(run.app.world(), actor, 1230959),
            "the real spender consumes the earned ERW effect"
        );
    }
}

#[test]
#[ignore = "native251 Reaper's Mark earns an Echoing Fury Exterminate use"]
fn frost_reapers_mark_earns_echoing_fury_rune_discount() {
    use crate::spell_cast::runes::Runes;
    for spender in [207230, 49020] {
        let _seed = crate::random::seed_scope(251);
        let mut run = super::super::prepare_literal(251, 3).unwrap();
        super::super::engage(&mut run);
        assert!(auras::has_aura(run.app.world(), run.actor, 1265855));
        cast(&mut run, 439843);
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let relevant = events
            .iter()
            .filter(|event| matches!(event.spell_id, Some(439843 | 441416)))
            .collect::<Vec<_>>();
        assert!(
            auras::has_aura(run.app.world(), run.actor, 441416),
            "real Reaper's Mark must earn Exterminate; observed {relevant:?}"
        );
        for _ in 0..30 {
            run.app.update();
        }
        let before = run.app.world().get::<Runes>(run.actor).unwrap().ready();
        assert!(before >= 2);
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, spender, Some(target), false)
            .expect("ordinary spender after actual Reaper's Mark");
        let after = run.app.world().get::<Runes>(actor).unwrap().ready();
        assert_eq!(
            before - after,
            1,
            "earned Exterminate must halve native rune cost"
        );
        assert!(
            !auras::has_aura(run.app.world(), actor, 441416),
            "one actual spender consumes the earned use"
        );
    }
}

#[test]
#[ignore = "real Exterminate second hit applies selected Wither Away Frost Fever"]
fn frost_wither_away_real_exterminate_applies_fever() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 441894));
    cast(&mut run, 439843);
    let target = run.targets[0];
    auras::remove_auras(
        run.app.world_mut(),
        target,
        RemoveReason::Consumed,
        |aura| aura.spell_id == 55095,
    );
    cast(&mut run, 49020);
    for _ in 0..120 {
        run.app.update();
    }
    assert!(
        auras::has_aura(run.app.world(), target, 55095),
        "earned Exterminate must apply Frost Fever"
    );
}

#[test]
#[ignore = "native251 earned Exterminate releases its two damage hits"]
fn frost_exterminate_spender_deals_both_native_hits() {
    use shared::protocol::CombatLogKind;
    for spender in [207230, 49020] {
        let _seed = crate::random::seed_scope(251);
        let mut run = super::super::prepare_literal(251, 3).unwrap();
        super::super::engage(&mut run);
        cast(&mut run, 439843);
        assert!(auras::has_aura(run.app.world(), run.actor, 441416));
        cast(&mut run, spender);
        for _ in 0..120 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for spell in [441424, 441426] {
            assert!(
                events.iter().any(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(run.targets[0].to_bits())
                        && event.spell_id == Some(spell)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                }),
                "earned spender {spender} must deal native Exterminate hit {spell}"
            );
        }
    }
}

#[test]
#[ignore = "actual Mark explosion earns two independent Exterminate uses"]
fn frost_mark_explosion_funds_two_discounted_spenders() {
    use crate::spell_cast::runes::Runes;
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    for _ in 0..6000 {
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
        let exploded = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(436304)
                    && event.kind == CombatLogKind::Damage
            });
        let stacks = run
            .app
            .world()
            .get::<ActiveAuras>(run.actor)
            .and_then(|active| {
                active
                    .iter()
                    .find(|aura| aura.spell_id == 441416 && aura.caster == run.actor)
                    .map(|aura| aura.stacks)
            });
        if exploded && stacks == Some(2) {
            break;
        }
    }
    let actor = run.actor;
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 441416 && aura.stacks == 2)
    );
    for remaining in [1, 0] {
        for _ in 0..400 {
            run.app.update();
            if run.app.world().get::<Runes>(actor).unwrap().ready() >= 2
                && !auras::has_aura(run.app.world(), actor, 1230959)
            {
                break;
            }
        }
        for _ in 0..30 {
            run.app.update();
        }
        let before = run.app.world().get::<Runes>(actor).unwrap().ready();
        assert!(before >= 2);
        let target = run.targets[0];
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49020, Some(target), false)
            .expect("ordinary rune spender after real Mark explosion");
        let after = run.app.world().get::<Runes>(actor).unwrap().ready();
        assert_eq!(
            before - after,
            1,
            "each earned use halves the native rune cost"
        );
        let stacks = run
            .app
            .world()
            .get::<ActiveAuras>(actor)
            .and_then(|active| {
                active
                    .iter()
                    .find(|aura| aura.spell_id == 441416 && aura.caster == actor)
                    .map(|aura| aura.stacks)
            })
            .unwrap_or(0);
        assert_eq!(
            stacks, remaining,
            "each real spender consumes exactly one earned use"
        );
    }
}

#[test]
#[ignore = "actual Reaper's Mark hit earns Grim Reaper Killing Machine"]
fn frost_mark_hit_earns_killing_machine() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 434905));
    assert!(!auras::has_aura(run.app.world(), run.actor, 51124));
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 439843, Some(target), false)
        .expect("real Reaper's Mark is available");
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(439843)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert!(
        auras::has_aura(run.app.world(), actor, 51124),
        "actual Mark hit must grant Killing Machine with Grim Reaper"
    );
}

#[test]
#[ignore = "native Frost Reaper's Mark uses only its Frost direct hit"]
fn frost_reapers_mark_deals_one_native_direct_hit() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    cast(&mut run, 439843);
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(439843)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        hits, 1,
        "a real Frost Mark must use only its Frost damage effect"
    );
}

#[test]
#[ignore = "actual Obliterate impacts earn Frostreaper damage"]
fn frost_frostreaper_follows_real_obliterate_and_frost_strike() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 1230301));
    super::super::advance_combat(&mut run).unwrap();
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let mark = events
        .iter()
        .position(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1233351)
                && event.kind == CombatLogKind::AuraApplied
        })
        .expect("actual Obliterate must earn a Frostreaper mark");
    assert!(
        events[mark..].iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1233619)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "real earned Frostreaper marks must produce damage"
    );
}

#[test]
#[ignore = "actual Killing Machine consumption rolls native Murderous Efficiency"]
fn frost_murderous_efficiency_tracks_consumed_killing_machine_stacks() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let mut consumed = 0;
    let mut procs = 0;
    for seed in [70448, 1, 2] {
        let _seed = crate::random::seed_scope(seed);
        let mut run = super::super::prepare_measurement(251, 1).unwrap();
        super::super::engage(&mut run);
        assert!(auras::has_aura(run.app.world(), run.actor, 207061));
        let actor = run.actor;
        crate::auras::observations::start_tracking(run.app.world_mut(), actor, &[(51124, 51128)]);
        super::super::advance_combat(&mut run).unwrap();
        let usage = crate::auras::observations::snapshot(run.app.world());
        consumed += usage["51124"]["consumed"].as_u64().unwrap();
        procs += run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(207062)
                    && event.kind == CombatLogKind::Energize
            })
            .count() as u64;
    }
    assert!(
        consumed >= 200,
        "need actual earned KM consumption, got {consumed}"
    );
    let rate = procs as f64 / consumed as f64;
    assert!(
        (0.10..0.30).contains(&rate),
        "actual ME rolls must follow native 20% per consumed KM stack: {procs}/{consumed}"
    );
}

#[test]
#[ignore = "actual Empower Rune Weapon grants Killing Machine"]
fn frost_erw_earns_killing_machine() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(!auras::has_aura(run.app.world(), run.actor, 51124));
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 47568, Some(target), false)
        .expect("real Empower Rune Weapon is available");
    assert!(
        auras::has_aura(run.app.world(), actor, 51124),
        "actual Empower Rune Weapon must grant Killing Machine"
    );
}

#[test]
#[ignore = "real Mark produces two Waves of Souls and a critical return"]
fn frost_mark_earns_two_waves_of_souls() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 3).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 439851));
    cast(&mut run, 439843);
    for _ in 0..100 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    for target in &run.targets {
        let hits: Vec<_> = events
            .iter()
            .filter(|event| {
                event.spell_id == Some(435802)
                    && event.kind == CombatLogKind::Damage
                    && event.target == Some(target.to_bits())
                    && event.amount > 0
            })
            .collect();
        assert_eq!(hits.len(), 2, "Wave impacts per target");
        assert!(hits[1].crit, "return wave must critically strike");
        assert!(auras::has_aura(run.app.world(), *target, 443404));
    }
}

#[test]
#[ignore = "real Mark explosion releases Soul Rupture only on secondary targets"]
fn frost_soul_rupture_from_real_mark_explosion() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 3).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 437161));
    cast(&mut run, 439843);
    for _ in 0..500 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.spell_id == Some(439594)
        && event.kind == CombatLogKind::Damage
        && event.target != Some(run.targets[0].to_bits())
        && event.amount > 0));
    assert!(!events.iter().any(|event| event.spell_id == Some(439594)
        && event.kind == CombatLogKind::Damage
        && event.target == Some(run.targets[0].to_bits())));
}

#[test]
#[ignore = "real Mark explosion grants Swift and Painful with no secondary target"]
fn frost_soul_rupture_without_secondary_earns_swift_and_painful() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 437161));
    assert!(auras::has_aura(run.app.world(), run.actor, 443560));
    cast(&mut run, 439843);
    for _ in 0..500 {
        run.app.update();
        if auras::has_aura(run.app.world(), run.actor, 469169) {
            return;
        }
    }
    panic!("single-target Mark explosion must earn Swift and Painful");
}

fn grim_mark_explosion(health_fraction: f32) -> i32 {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 434905));
    cast(&mut run, 439843);
    let mut health = run
        .app
        .world_mut()
        .get_mut::<shared::components::Health>(run.targets[0])
        .unwrap();
    health.current = health.max * health_fraction;
    for _ in 0..500 {
        run.app.update();
    }
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(436304)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .expect("real Mark expiration must explode")
        .amount
}

#[test]
#[ignore = "actual Mark explosion scales with Grim Reaper target missing health"]
fn frost_grim_reaper_scales_real_mark_explosion() {
    let full = grim_mark_explosion(1.0);
    let half = grim_mark_explosion(0.5);
    assert!(
        (half as f32 / full as f32 - 1.15).abs() < 0.02,
        "full={full} half={half}"
    );
}

#[test]
#[ignore = "native Reaper's Mark explodes and earns Exterminate from actual damage"]
fn frost_mark_explosion_earns_exterminate() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 441378));
    cast(&mut run, 439843);
    assert!(auras::has_aura(run.app.world(), run.targets[0], 434765));
    for _ in 0..500 {
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let explosion = events
        .iter()
        .position(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(436304)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .expect("actual Mark must explode after real damaging hits or expiry");
    assert!(
        events[explosion..].iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(434765)
                && event.kind == CombatLogKind::AuraRemoved
                && event.amount > 1
        }),
        "real damage must build the Mark before it explodes"
    );
    assert!(
        events[explosion..].iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(441416)
                && matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )
                && event.amount == 2
        }),
        "actual explosion must earn both native Exterminate uses"
    );
}

#[test]
#[ignore = "native Breath ending restores its authored runes"]
fn frost_breath_end_restores_runes() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    super::super::advance_combat(&mut run).unwrap();
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1249658)
                && event.kind == CombatLogKind::AuraRemoved),
        "real Breath must end during the fight"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1234304)
                && event.kind == CombatLogKind::Energize
                && event.amount > event.overflow),
        "real Breath expiration must replenish spent runes from native 1234304"
    );
}

#[test]
#[ignore = "real rune spenders extend Frozen Dominion through Gathering Storm"]
fn frost_gathering_storm_extends_winter_for_paid_and_free_spenders() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    assert!(auras::has_aura(run.app.world(), actor, 194912));
    super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
    let expiration = |run: &Run| {
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1233152 && aura.caster == actor)
            .unwrap()
            .expires_at
            .unwrap()
    };
    let base_extension =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 194912, 0);
    let before = expiration(&run);
    cast(&mut run, 49020);
    let paid = expiration(&run);
    assert!(
        (paid - before - f64::from(base_extension) * 0.2).abs() < 0.001,
        "paid two-rune Obliterate extends Winter: {before} -> {paid}"
    );
    cast(&mut run, 47568);
    let before_free = expiration(&run);
    assert!(auras::has_aura(run.app.world(), actor, 1230959));
    for _ in 0..30 {
        run.app.update();
    }
    let runes_before = run
        .app
        .world()
        .get::<crate::spell_cast::runes::Runes>(actor)
        .unwrap()
        .ready();
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49020, Some(target), false)
        .unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<crate::spell_cast::runes::Runes>(actor)
            .unwrap()
            .ready(),
        runes_before
    );
    let free = expiration(&run);
    assert!(!auras::has_aura(run.app.world(), actor, 1230959));
    assert!(
        (free - before_free - f64::from(base_extension) * 0.2).abs() < 0.001,
        "free Obliterate still extends Winter by its base rune cost: {before_free} -> {free}"
    );
    let stacks = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 211805 && aura.caster == actor)
        .unwrap()
        .stacks;
    assert_eq!(
        stacks, 4,
        "both real Obliterates earn stacks per base rune cost"
    );
    for _ in 0..600 {
        run.app.update();
        if !auras::has_aura(run.app.world(), actor, 1233152) {
            break;
        }
    }
    assert!(!auras::has_aura(run.app.world(), actor, 1233152));
    assert!(
        !auras::has_aura(run.app.world(), actor, 211805),
        "Winter ending clears Gathering Storm"
    );
}

#[test]
#[ignore = "real Frostwyrm's Fury earns and uses Chosen recall"]
fn frost_fury_earns_and_uses_recall() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    cast(&mut run, 279302);
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        auras::overrides::resolve_spell(run.app.world(), run.actor, 279302),
        1265384
    );
    cast(&mut run, 1265384);
    for _ in 0..10 {
        run.app.update();
    }
    assert!(!auras::has_aura(run.app.world(), run.actor, 1265639));
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(279303)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(hits, 2, "Fury and its recall must each deal damage");
}

#[test]
#[ignore = "real Frostwyrm's Fury grants Chosen of Frostbrood haste"]
fn frost_fury_earns_chosen_haste() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 1265632));
    let before = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    cast(&mut run, 279302);
    assert!(auras::has_aura(run.app.world(), run.actor, 1265630));
    let after = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    assert!(
        after > before,
        "Chosen haste must increase actual melee haste: {before} -> {after}"
    );
}

#[test]
#[ignore = "native Frostwyrm's Fury extends an active Pillar with Chosen of Frostbrood"]
fn frost_fury_extends_real_pillar() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    assert!(auras::has_aura(run.app.world(), actor, 1265633));
    super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
    let expiration = |run: &Run| {
        run.app
            .world()
            .get::<ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 51271 && aura.caster == run.actor)
            .unwrap()
            .expires_at
            .unwrap()
    };
    let before = expiration(&run);
    let extension_ms =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 1265633, 0);
    cast(&mut run, 279302);
    let after = expiration(&run);
    assert!(
        (after - before - f64::from(extension_ms) / 1000.0).abs() < 0.001,
        "real Fury must extend active Pillar by native Chosen duration: {before} -> {after}"
    );
}

#[test]
#[ignore = "native Frost Fever ticks use the pinned Runic Power proc rate"]
fn frost_fever_actual_ticks_roll_runic_power() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    for (targets, minimum, maximum) in [(1, 0.15, 0.45), (3, 0.07, 0.32)] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_measurement(251, targets).unwrap();
        super::super::engage(&mut run);
        super::super::advance_combat(&mut run).unwrap();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        let ticks = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(55095)
                    && event.kind == CombatLogKind::Damage
                    && event.periodic
            })
            .count();
        let procs = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(195617)
                    && event.kind == CombatLogKind::Energize
            })
            .count();
        assert!(
            ticks >= 100,
            "actual Frost Fever needs enough ticks: {ticks}"
        );
        let rate = procs as f64 / ticks as f64;
        assert!(
            (minimum..maximum).contains(&rate),
            "actual {targets}T Frost Fever proc rate {rate:.3} ({procs}/{ticks}) violates pinned chance"
        );
    }
}

#[test]
#[ignore = "native Frost earned rune refill preserves in-progress regeneration"]
fn frost_earned_rune_refill_preserves_regeneration() {
    use crate::spell_cast::test_harness::ObservedRuneRefills;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    run.app.insert_resource(ObservedRuneRefills::default());
    super::super::engage(&mut run);
    for _ in 0..6000 {
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
        let observations = &run.app.world().resource::<ObservedRuneRefills>().0;
        if let Some((_, before, after)) = observations.iter().find(|(actor, before, after)| {
            *actor == run.actor && before.order.len() >= 4 && after.ready() == before.ready() + 1
        }) {
            for rune in before.order.iter().take(3) {
                let index = usize::from(*rune);
                assert_eq!(
                    before.cooldowns[index], after.cooldowns[index],
                    "earned rune refill must preserve active regeneration while a depleted rune waits"
                );
            }
            return;
        }
    }
    panic!("real rotation did not earn a rune refill while a depleted rune waited");
}

#[test]
#[ignore = "diagnostic native Frost first Mark and Exterminate lifecycle"]
fn diag_frost_mark_exterminate_timeline() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    let mut cursor = 0;
    let mut trace = Vec::new();
    super::super::advance_combat_observed(&mut run, |run| {
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        let time = run.app.world().resource::<Time>().elapsed_secs_f64();
        if time < 30.0 {
            for event in &events[cursor..] {
                if matches!(
                    event.spell_id,
                    Some(439843 | 434765 | 436304 | 441416 | 441424 | 49020 | 51124)
                ) {
                    trace.push(serde_json::json!({"time": time, "spell": event.spell_id,
                        "kind": format!("{:?}", event.kind), "amount": event.amount}));
                }
            }
        }
        cursor = events.len();
    })
    .unwrap();
    assert!(!trace.is_empty());
    let output = std::path::PathBuf::from(std::env::var("DPS_PARITY_OUTPUT").unwrap());
    std::fs::write(
        output.join("251.mark-timeline.json"),
        serde_json::to_vec_pretty(&trace).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "real haste increases preserve rune recharge progress"]
fn frost_active_rune_recharge_tracks_real_haste_increase() {
    use crate::spell_cast::runes::Runes;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    for _ in 0..6000 {
        let actor = run.actor;
        let before_haste = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        let before = run.app.world().get::<Runes>(actor).unwrap().clone();
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
        let after_haste = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        let after = run.app.world().get::<Runes>(actor).unwrap();
        if after_haste <= before_haste + 5.0 || before.order != after.order {
            continue;
        }
        // Aura stat recomputation can occur after rune regeneration in the same fixed tick.
        run.app.update();
        let settled_haste = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        let after = run.app.world().get::<Runes>(actor).unwrap();
        if (settled_haste - after_haste).abs() > 0.001 || before.order != after.order {
            continue;
        }
        for rune in before.order.iter().take(3) {
            let index = usize::from(*rune);
            let old = before.cooldowns[index];
            let new = after.cooldowns[index];
            let adjusted = old * (1.0 + before_haste / 100.0) / (1.0 + after_haste / 100.0);
            if old > 3.0 && old - adjusted > 0.15 {
                assert!(
                    new <= adjusted + 0.06,
                    "real haste increase {before_haste:.2}% -> {after_haste:.2}% must rescale active rune {old:.3}s -> {adjusted:.3}s; native {new:.3}s"
                );
                return;
            }
        }
    }
    panic!("real rotation never increased haste with an active rune recharge");
}

#[test]
#[ignore = "diagnostic fixed-haste native rune regeneration against pinned SimC"]
fn diag_frost_fixed_haste_idle_rune_regeneration() {
    use crate::spell_cast::runes::Runes;
    use shared::components::PowerType;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    let haste = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .haste_pct;
    let expected = 10.0 / (1.0 + haste / 100.0);
    assert_eq!(
        crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Runes, -6.0),
        Some(-6.0)
    );
    let start = run.app.world().resource::<Time>().elapsed_secs_f64();
    let mut first_three = None;
    let mut all_six = None;
    for _ in 0..1200 {
        run.app.update();
        let elapsed = run.app.world().resource::<Time>().elapsed_secs_f64() - start;
        let ready = run.app.world().get::<Runes>(actor).unwrap().ready();
        if ready >= 3 && first_three.is_none() {
            first_three = Some(elapsed);
        }
        if ready == 6 && all_six.is_none() {
            all_six = Some(elapsed);
        }
    }
    let first = first_three.expect("first three depleted runes regenerate");
    let all = all_six.expect("remaining three depleted runes regenerate");
    println!(
        "fixed haste {haste}%: first three {first:.3}s, all six {all:.3}s; SimC {expected:.3}s and {:.3}s",
        expected * 2.0
    );
    assert!((first - f64::from(expected)).abs() <= 0.10);
    assert!((all - f64::from(expected * 2.0)).abs() <= 0.20);
    assert_eq!(run.app.world().get::<Runes>(actor).unwrap().ready(), 6);
}

#[test]
#[ignore = "diagnostic Frost oracle-aligned actual spell factors"]
fn diag_frost_oracle_aligned_factors() {
    use super::super::single_spell::{SpellProbe, capture_cast};
    let output = std::path::PathBuf::from(std::env::var("DPS_PARITY_OUTPUT").unwrap());
    for (root, focus, label, pillar) in [
        (49020, 1264082, "obliterate_frost", true),
        (49184, 49184, "howling_blast", false),
    ] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_literal(251, 1).unwrap();
        super::super::engage(&mut run);
        if pillar {
            let actor = run.actor;
            super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
        }
        let receipt = capture_cast(
            &mut run,
            &SpellProbe {
                spec: 251,
                spell: root,
                damage_spell: focus,
                action: if root == 49020 {
                    "obliterate"
                } else {
                    "howling_blast"
                },
                ticks: 1,
            },
        )
        .unwrap();
        assert_eq!(receipt["captured_hits"], 1);
        std::fs::write(
            output.join(format!("251.oracle-aligned.{label}.json")),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
    }
}

fn real_frost_debuff_damage_ratio(debuff: u32, source: u32) -> (Vec<f32>, u8) {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..6000 {
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
        if run
            .app
            .world()
            .get::<ActiveAuras>(target)
            .is_some_and(|active| {
                active
                    .iter()
                    .any(|aura| aura.spell_id == debuff && aura.caster == actor)
            })
        {
            break;
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(source)
                && event.kind == CombatLogKind::Damage),
        "real source {source} must hit target"
    );
    let stacks = run
        .app
        .world()
        .get::<ActiveAuras>(target)
        .and_then(|active| {
            active
                .iter()
                .find(|aura| aura.spell_id == debuff && aura.caster == actor)
                .map(|aura| aura.stacks)
        })
        .unwrap_or_else(|| panic!("real source {source} must earn target debuff {debuff}"));
    let spells: Vec<_> = [196771, 1264082, 222026]
        .into_iter()
        .map(|id| {
            run.app
                .world()
                .resource::<crate::spell_info::SpellStore>()
                .get(id)
                .unwrap()
                .clone()
        })
        .collect();
    let with: Vec<_> = spells
        .iter()
        .map(|spell| auras::damage_taken_multiplier(run.app.world(), target, actor, spell))
        .collect();
    auras::remove_auras(
        run.app.world_mut(),
        target,
        RemoveReason::Consumed,
        |aura| aura.spell_id == debuff && aura.caster == actor,
    );
    let ratios = spells
        .iter()
        .zip(with)
        .map(|(spell, value)| {
            value / auras::damage_taken_multiplier(run.app.world(), target, actor, spell)
        })
        .collect();
    (ratios, stacks)
}

#[test]
#[ignore = "real Frost Fever ticks earn Brittle and amplify Winter"]
fn frost_fever_earns_brittle_target_damage() {
    let (ratios, stacks) = real_frost_debuff_damage_ratio(374557, 55095);
    assert_eq!(stacks, 1);
    for ratio in ratios {
        assert!(
            (ratio - 1.06).abs() < 0.001,
            "real Brittle must apply its native 6%: {ratio}"
        );
    }
}

#[test]
#[ignore = "real Arctic Assault earns Razorice and amplifies Winter"]
fn frost_arctic_assault_earns_razorice_target_damage() {
    let (ratios, stacks) = real_frost_debuff_damage_ratio(51714, 195975);
    // Native 3% amplified by the selected Unholy Bond label modifier (20%).
    for ratio in ratios {
        assert!(
            (ratio - (1.0 + 0.036 * f32::from(stacks))).abs() < 0.001,
            "real Razorice must apply its native per-stack value: {ratio}, stacks={stacks}"
        );
    }
}

#[test]
#[ignore = "diagnostic Frost target debuff production and damage-taken factors"]
fn diag_frost_target_debuffs() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    run.app
        .insert_resource(FactorCapture::new(actor, target, 196771, "winter", 30));
    super::super::engage(&mut run);
    let mut states = Vec::new();
    let mut previous = String::new();
    for step in 0..800 {
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
        let entries: Vec<_> = run.app.world().get::<ActiveAuras>(target).into_iter().flat_map(|active| active.iter())
            .filter(|aura| matches!(aura.spell_id, 51714 | 376974 | 374557))
            .map(|aura| serde_json::json!({"spell": aura.spell_id, "caster": aura.caster.to_bits(), "stacks": aura.stacks,
                "effects": aura.effects.iter().map(|effect| serde_json::json!({"index": effect.index,"type":effect.aura_type,"amount":effect.amount})).collect::<Vec<_>>() })).collect();
        let key = entries
            .iter()
            .map(|entry| entry.to_string())
            .collect::<String>();
        if key != previous {
            states.push(serde_json::json!({"step": step, "auras": entries}));
            previous = key;
        }
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let output = std::path::PathBuf::from(std::env::var("DPS_PARITY_OUTPUT").unwrap());
    std::fs::write(
        output.join("251.target-debuffs.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"actor":actor.to_bits(),"states":states,"capture":capture}),
        )
        .unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "real Winter hits earn Everfrost target stacks and amplify later hits"]
fn frost_winter_hits_earn_everfrost_target_damage() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(auras::has_aura(run.app.world(), actor, 376938));
    run.app
        .insert_resource(FactorCapture::new(actor, target, 196771, "winter", 2));
    super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
    for _ in 0..100 {
        run.app.update();
        if run.app.world().resource::<FactorCapture>().export()["captured_hits"] == 2 {
            break;
        }
    }
    let stacks = run
        .app
        .world()
        .get::<ActiveAuras>(target)
        .and_then(|active| {
            active
                .iter()
                .find(|aura| aura.spell_id == 376974 && aura.caster == actor)
                .map(|aura| aura.stacks)
        })
        .unwrap_or(0);
    assert_eq!(stacks, 2, "each real Winter hit earns a target stack");
    let capture = run.app.world().resource::<FactorCapture>().export();
    let hits: Vec<_> = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["spell_id"] == 196771 && event["event"] == "direct_amount")
        .collect();
    assert_eq!(hits.len(), 2);
    let first = hits[0]["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["factor"] == "target_damage")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    let second = hits[1]["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["factor"] == "target_damage")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    let pct = crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 376938, 0);
    assert!(
        (second / first - (1.0 + f64::from(pct) / 100.0)).abs() < 0.001,
        "the next real Winter hit uses the native earned-stack bonus: {first} -> {second}"
    );
}

#[test]
#[ignore = "real Winter tick is not directly amplified by Pillar"]
fn frost_winter_tick_pillar_only_changes_attack_power() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    fn tick(keep_pillar: bool) -> (f64, f64) {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_literal(251, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
        assert!(auras::has_aura(run.app.world(), actor, 1233152));
        if !keep_pillar {
            clear_proc(&mut run, 51271);
        }
        run.app.insert_resource(FactorCapture::new(
            actor,
            run.targets[0],
            196771,
            "remorseless_winter_damage",
            1,
        ));
        for _ in 0..100 {
            run.app.update();
            if run.app.world().resource::<FactorCapture>().export()["captured_hits"] == 1 {
                break;
            }
        }
        let capture = run.app.world().resource::<FactorCapture>().export();
        let hit = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["spell_id"] == 196771 && event["event"] == "direct_amount")
            .unwrap();
        (
            hit["factors"]["amount"].as_f64().unwrap(),
            hit["factors"]["a_power"].as_f64().unwrap(),
        )
    }
    let (with, with_ap) = tick(true);
    let (without, without_ap) = tick(false);
    assert!(
        (with / with_ap - without / without_ap).abs() < 0.001,
        "Pillar changes Winter through attack power only, not direct damage: {with}/{with_ap} vs {without}/{without_ap}"
    );
}

#[test]
#[ignore = "Freezing Tempest gains one stack per Winter pulse, not other spells"]
fn frost_freezing_tempest_only_counts_real_winter_pulses() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    for targets in [1, 3] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_literal(251, targets).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        assert!(auras::has_aura(run.app.world(), actor, 1296652));
        super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
        let hits = |run: &Run| {
            run.app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.spell_id == Some(196771)
                        && event.kind == CombatLogKind::Damage
                })
                .count()
        };
        let stacks = |run: &Run| {
            run.app
                .world()
                .get::<ActiveAuras>(actor)
                .and_then(|active| {
                    active
                        .iter()
                        .find(|aura| aura.spell_id == 1297365 && aura.caster == actor)
                        .map(|aura| aura.stacks)
                })
                .unwrap_or(0)
        };
        for _ in 0..60 {
            run.app.update();
            if hits(&run) >= targets {
                break;
            }
        }
        assert_eq!(hits(&run), targets, "first real pulse hits each target");
        assert_eq!(
            stacks(&run),
            1,
            "one actual pulse grants one stack, not one per target"
        );
        for _ in 0..30 {
            run.app.update();
        }
        let previous_hits = hits(&run);
        for _ in 0..30 {
            run.app.update();
            if hits(&run) > previous_hits {
                break;
            }
        }
        assert!(hits(&run) > previous_hits);
        let before = stacks(&run);
        let target = run.targets[0];
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49184, Some(target), false)
            .unwrap();
        for _ in 0..5 {
            run.app.update();
        }
        assert_eq!(
            hits(&run),
            previous_hits + targets,
            "no additional Winter pulse during Howling Blast"
        );
        assert_eq!(
            stacks(&run),
            before,
            "a real Howling Blast must not grant Freezing Tempest"
        );
    }
}

#[test]
#[ignore = "starting a real Winter clears existing earned Freezing Tempest"]
fn frost_winter_start_clears_earned_freezing_tempest() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
    for _ in 0..40 {
        run.app.update();
        if auras::has_aura(run.app.world(), actor, 1297365) {
            break;
        }
    }
    assert!(auras::has_aura(run.app.world(), actor, 1297365));
    assert!(
        crate::spell_cast::try_cast_triggered(run.app.world_mut(), actor, 1233152, Some(actor), 1),
        "native Frozen Dominion Winter starts"
    );
    assert!(
        !auras::has_aura(run.app.world(), actor, 1297365),
        "Winter restart clears previously earned stacks"
    );
}

#[test]
#[ignore = "diagnostic actual Freezing Tempest stack events"]
fn diag_frost_freezing_tempest_events() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    let mut old = 0;
    let mut rows = Vec::new();
    for step in 0..1200 {
        let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
        let stacks = run
            .app
            .world()
            .get::<ActiveAuras>(actor)
            .and_then(|active| {
                active
                    .iter()
                    .find(|aura| aura.spell_id == 1297365 && aura.caster == actor)
                    .map(|aura| aura.stacks)
            })
            .unwrap_or(0);
        if stacks != old {
            let events = &run.app.world().resource::<ObservedCombatEvents>().0[start..];
            rows.push(serde_json::json!({"step":step,"before":old,"after":stacks,
                "events":events.iter().map(|event| format!("{:?}:{}",event.kind,event.spell_id.unwrap_or(0))).collect::<Vec<_>>() }));
            old = stacks;
        }
    }
    let output = std::path::PathBuf::from(std::env::var("DPS_PARITY_OUTPUT").unwrap());
    std::fs::write(
        output.join("251.tempest-timeline.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "diagnostic actual Torrent hit factor capture"]
fn diag_frost_torrent_hit_factors() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        439539,
        "icy_death_torrent",
        30,
    ));
    super::super::engage(&mut run);
    for _ in 0..1200 {
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
        if run.app.world().resource::<FactorCapture>().export()["captured_hits"]
            .as_u64()
            .unwrap()
            >= 30
        {
            break;
        }
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    assert!(capture["captured_hits"].as_u64().unwrap() > 0);
    let output = std::path::PathBuf::from(std::env::var("DPS_PARITY_OUTPUT").unwrap());
    std::fs::write(
        output.join("251.torrent-factors.json"),
        serde_json::to_vec_pretty(&capture).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "diagnostic actual Winter tick factor capture"]
fn diag_frost_winter_tick_factors() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        196771,
        "remorseless_winter_damage",
        40,
    ));
    super::super::engage(&mut run);
    for _ in 0..1200 {
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
        if run.app.world().resource::<FactorCapture>().export()["captured_hits"]
            .as_u64()
            .unwrap()
            >= 40
        {
            break;
        }
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    assert!(capture["captured_hits"].as_u64().unwrap() > 0);
    let output = std::path::PathBuf::from(std::env::var("DPS_PARITY_OUTPUT").unwrap());
    std::fs::write(
        output.join("251.winter-factors.json"),
        serde_json::to_vec_pretty(&capture).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "diagnostic actual Obliterate physical hit factors"]
fn diag_frost_obliterate_physical_factors() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        222024,
        "obliterate_damage",
        30,
    ));
    super::super::engage(&mut run);
    for _ in 0..1200 {
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
        if run.app.world().resource::<FactorCapture>().export()["captured_hits"]
            .as_u64()
            .unwrap()
            >= 30
        {
            break;
        }
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    assert!(capture["captured_hits"].as_u64().unwrap() > 0);
    let output = std::path::PathBuf::from(std::env::var("DPS_PARITY_OUTPUT").unwrap());
    std::fs::write(
        output.join("251.obliterate-physical-factors.json"),
        serde_json::to_vec_pretty(&capture).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "diagnostic Frost native 300s factors and resource changes"]
fn diag_frost_factors_and_resource_spending() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    use crate::spell_cast::test_harness::ObservedPowerChanges;
    use shared::components::PowerType;

    let output = std::path::PathBuf::from(std::env::var("DPS_PARITY_OUTPUT").unwrap());
    let mut totals = Vec::new();
    for seed in [70448, 1, 2] {
        let _seed = crate::random::seed_scope(seed);
        let mut run = super::super::prepare_measurement(251, 1).unwrap();
        run.app.insert_resource(ObservedPowerChanges::default());
        if seed == 70448 || seed == 1 {
            let (spell, label) = if seed == 70448 {
                (1264082, "obliterate_frost")
            } else {
                (49184, "howling_blast")
            };
            run.app.insert_resource(FactorCapture::new(
                run.actor,
                run.targets[0],
                spell,
                label,
                20,
            ));
        }
        super::super::engage(&mut run);
        let actor = run.actor;
        crate::auras::observations::start_tracking(run.app.world_mut(), actor, &[(51124, 51128)]);
        let start_runes = run
            .app
            .world()
            .get::<crate::spell_cast::runes::Runes>(run.actor)
            .unwrap()
            .ready();
        let (seconds, _, _) = super::super::advance_combat(&mut run).unwrap();
        assert!((seconds - 300.0).abs() < 0.001);
        let end_runes = run
            .app
            .world()
            .get::<crate::spell_cast::runes::Runes>(run.actor)
            .unwrap()
            .ready();
        let combat = super::super::report::combat(run.app.world(), &run.damage_targets, seconds);
        let observed = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let killing_machine_events: Vec<_> = observed
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits()) && event.spell_id == Some(51124)
            })
            .map(|event| {
                serde_json::json!({
                    "kind": format!("{:?}", event.kind),
                    "amount": event.amount,
                    "timestamp_ms": event.timestamp_unix_ms,
                })
            })
            .collect();
        let changes = &run.app.world().resource::<ObservedPowerChanges>().0;
        let spent = |power| {
            -changes
                .iter()
                .filter(|(actor, kind, amount)| {
                    *actor == run.actor && *kind == power && *amount < 0.0
                })
                .map(|(_, _, amount)| *amount)
                .sum::<f32>()
        };
        let runes = spent(PowerType::Runes);
        let runic_power_raw = spent(PowerType::RunicPower);
        let rune_refills = changes
            .iter()
            .filter(|(actor, kind, amount)| {
                *actor == run.actor && *kind == PowerType::Runes && *amount > 0.0
            })
            .map(|(_, _, amount)| *amount)
            .sum::<f32>();
        assert!(runes > 0.0 && runic_power_raw > 0.0);
        totals.push(serde_json::json!({
            "seed": seed,
            "runes": runes,
            "rune_refills": rune_refills,
            "start_runes": start_runes,
            "end_runes": end_runes,
            "regenerated_runes": runes + end_runes as f32 - start_runes as f32 - rune_refills,
            "runic_power_raw": runic_power_raw,
            "energize": combat["energize"],
            "killing_machine_events": killing_machine_events,
            "aura_usage": crate::auras::observations::snapshot(run.app.world()),
        }));
        if seed == 70448 || seed == 1 {
            let capture = run.app.world().resource::<FactorCapture>().export();
            assert!(capture["captured_hits"].as_u64().unwrap() > 0);
            std::fs::write(
                output.join(format!("251.factors.{seed}.json")),
                serde_json::to_vec_pretty(&capture).unwrap(),
            )
            .unwrap();
        }
    }
    std::fs::write(
        output.join("251.power-changes.json"),
        serde_json::to_vec_pretty(&totals).unwrap(),
    )
    .unwrap();
}

fn earn_rime(run: &mut Run) {
    for _ in 0..24 {
        cast(run, 49020);
        cast(run, 49020);
        cast(run, 49143);
        if auras::has_aura(run.app.world(), run.actor, 59052) {
            return;
        }
    }
    panic!("ordinary native Frost Strikes never earned Rime");
}

#[test]
#[ignore = "actual Rime consumption earns Dark Talons and increases Icy Talons stacks"]
fn frost_dark_talons_from_earned_rime_increases_icy_talons() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 436687));
    for _ in 0..20 {
        earn_rime(&mut run);
        cast(&mut run, 49184);
        if auras::has_aura(run.app.world(), run.actor, 443595) {
            assert!(
                auras::has_aura(run.app.world(), run.actor, 443586),
                "Icy Talons must earn Dark Talons Shadowfrost"
            );
            let aura = run
                .app
                .world()
                .get::<ActiveAuras>(run.actor)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 194879 && aura.caster == run.actor)
                .unwrap();
            assert!(
                aura.stacks > 3 && aura.max_stacks >= 6,
                "Dark Talons must grant three Icy Talons stacks and increase its cap: {aura:?}"
            );
            return;
        }
    }
    panic!("earned Rime casts never produced Dark Talons");
}

fn earn_killing_machine(run: &mut Run) {
    cast(run, 49020);
    cast(run, 49020);
    let actor = run.actor;
    super::super::enhancement_generation_tests::cast(run, 51271, actor);
    clear_proc(run, 51124);
    cast(run, 49143);
    assert!(auras::has_aura(run.app.world(), actor, 51124));
}

#[test]
#[ignore = "earned Killing Machine changes actual Obliterate weapon hits to Frost"]
fn frost_km_converts_obliterate_weapon_children_to_frost() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    earn_killing_machine(&mut run);
    let actor = run.actor;
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49020, Some(target), false)
        .unwrap();
    for _ in 0..3 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[start..];
    for id in [222024, 66198] {
        let event = events
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(id)
                    && event.kind == CombatLogKind::Damage
            })
            .expect("real Obliterate weapon impact");
        assert_eq!(
            event.school_mask, 16,
            "earned Killing Machine converts weapon child {id} to Frost"
        );
    }
}

#[test]
#[ignore = "converted real Obliterate weapon hit applies mastery and earned Razorice"]
fn frost_km_obliterate_weapon_uses_mastery_and_razorice() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..6000 {
        let razorice = run
            .app
            .world()
            .get::<ActiveAuras>(target)
            .and_then(|active| {
                active
                    .iter()
                    .find(|aura| aura.spell_id == 51714 && aura.caster == actor)
            })
            .cloned();
        if auras::has_aura(run.app.world(), actor, 51124)
            && let Some(aura) = razorice
        {
            let mastery = run
                .app
                .world()
                .get::<crate::stats::CharacterStats>(actor)
                .unwrap()
                .mastery_pct
                / 100.0;
            let amount = aura
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .amount;
            let expected = (1.0 + mastery) * (1.0 + amount * f32::from(aura.stacks) / 100.0);
            run.app.insert_resource(FactorCapture::new(
                actor,
                target,
                222024,
                "obliterate_damage",
                1,
            ));
            if crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                49020,
                Some(target),
                false,
            )
            .is_ok()
            {
                for _ in 0..3 {
                    run.app.update();
                }
                let capture = run.app.world().resource::<FactorCapture>().export();
                let hit = capture["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|event| event["spell_id"] == 222024 && event["event"] == "direct_amount")
                    .unwrap();
                let product = hit["applied_multipliers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|factor| factor["factor"] == "ret_damage")
                    .unwrap()["value"]
                    .as_f64()
                    .unwrap();
                assert!(
                    (product - f64::from(expected)).abs() < 0.001,
                    "real converted hit uses mastery and owned Razorice: {product} vs {expected}"
                );
                return;
            }
        }
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
    }
    panic!("real rotation never admitted KM Obliterate against earned Razorice");
}

#[test]
#[ignore = "native251 earned KM, real weapon children before delayed consumption"]
fn frost_km_spenders_consume_after_weapon_impacts() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    for spender in [49020, 207230] {
        let _seed = crate::random::seed_scope(251);
        let mut run = super::super::prepare_literal(251, 1).unwrap();
        super::super::engage(&mut run);
        earn_killing_machine(&mut run);
        run.app
            .world_mut()
            .resource_mut::<ObservedCombatEvents>()
            .0
            .clear();
        let target = run.targets[0];
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spender,
            Some(target),
            false,
        )
        .expect("ordinary rune spender with earned KM");
        assert!(
            auras::has_aura(run.app.world(), run.actor, 51124),
            "KM survives launch for native weapon children"
        );
        for _ in 0..3 {
            run.app.update();
        }
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        let consumed = events
            .iter()
            .position(|event| {
                event.spell_id == Some(51124) && event.kind == CombatLogKind::AuraRemoved
            })
            .expect("actual rune spender must consume earned KM");
        let damage = events
            .iter()
            .rposition(|event| {
                event.kind == CombatLogKind::Damage
                    && match spender {
                        49020 => matches!(
                            event.spell_id,
                            Some(222024 | 66198 | 325461 | 1264082 | 1264083 | 1264084)
                        ),
                        _ => event.spell_id == Some(207230),
                    }
            })
            .expect("native spender weapon damage");
        assert!(
            damage < consumed,
            "KM must remain through all native weapon impacts: spender{spender}"
        );
    }
}

#[test]
#[ignore = "native251 literal casts earn KM callbacks without injected procs or resources"]
fn frost_km_callbacks_restore_runes_and_emit_native_buffs_and_arctic_assault() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_measurement(251, 3).unwrap();
    super::super::engage(&mut run);
    for _ in 0..2400 {
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(207062)
            && event.kind == CombatLogKind::Energize
            && event.amount > event.overflow),
        "Murderous Efficiency must restore an actually spent native rune"
    );
    for spell in [1230916, 377101, 377103] {
        assert!(
            events
                .iter()
                .any(|event| event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::AuraApplied),
            "earned KM must produce native buff{spell}"
        );
    }
    assert!(
        events.iter().any(|event| event.spell_id == Some(195975)
            && event.kind == CombatLogKind::Damage
            && event.amount > 0),
        "earned KM must fire native Arctic Assault damage"
    );
}

#[test]
#[ignore = "native251 actual Breath and earned KM extend the running native aura"]
fn frost_km_consumption_extends_active_breath() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    cast(&mut run, 49020);
    earn_killing_machine(&mut run);
    cast(&mut run, 47568);
    let actor = run.actor;
    super::super::enhancement_generation_tests::cast(&mut run, 1249658, actor);
    let read_aura = |run: &Run, spell| {
        run.app
            .world()
            .get::<ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell)
            .cloned()
    };
    let before = read_aura(&run, 1249658)
        .expect("ordinary Breath is active")
        .expires_at
        .unwrap();
    let streak_before = read_aura(&run, 1230916).map_or(0, |aura| aura.stacks);
    assert!(
        read_aura(&run, 51124).is_some(),
        "setup retains an earned KM"
    );
    let extension_ms = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1249658)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap()
        .base_points;
    // Setup spent all six starting runes. Wait for ordinary rune regeneration;
    // Frozen Dominion's real Winter must not require a lucky Empowerment roll.
    cast(&mut run, 49020);
    for _ in 0..3 {
        run.app.update();
    }
    let streak_after = read_aura(&run, 1230916)
        .expect("consumed KM earns Killing Streak")
        .stacks;
    let spent = streak_after - streak_before;
    assert!(spent > 0);
    let after = read_aura(&run, 1249658)
        .expect("Breath survives consumption")
        .expires_at
        .unwrap();
    let expected = f64::from(extension_ms) * f64::from(spent) / 1000.0;
    assert!(
        (after - before - expected).abs() < 0.002,
        "native Breath expiry {before} -> {after}, expected extension{expected}"
    );
}

#[test]
#[ignore = "native251 paid Runic Power must roll actual Runic Empowerment rune restoration"]
fn frost_paid_runic_power_restores_native_runes() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    for _ in 0..2400 {
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.spell_id == Some(49143) && event.kind == CombatLogKind::CastSuccess),
        "fixture must pay native RP for actual Frost Strike"
    );
    assert!(
        events.iter().any(|event| event.spell_id == Some(193486)
            && event.kind == CombatLogKind::Energize
            && event.amount > event.overflow),
        "paid RP must earn effective Runic Empowerment rune restoration, not a synthetic rotation grant"
    );
}

#[test]
#[ignore = "native251 selected Frozen Dominion starts actual Remorseless Winter from Pillar"]
fn frost_pillar_starts_frozen_dominion_winter() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 3).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 377226));
    let actor = run.actor;
    super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
    assert!(
        auras::has_aura(run.app.world(), actor, 1233152),
        "ordinary Pillar with selected Frozen Dominion starts its native Winter aura"
    );
    for _ in 0..80 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    for target in &run.targets {
        assert!(
            events.iter().any(|event| event.spell_id == Some(196771)
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
            "native Winter must pulse damage onto each in-range enemy{target:?}"
        );
    }
}

#[test]
#[ignore = "native251 earned Rime damage must accumulate Cryogenic Chamber and real Winter releases it"]
fn frost_cryogenic_chamber_accumulates_earned_rime_hits_and_winter_releases() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 3).unwrap();
    super::super::engage(&mut run);
    assert!(auras::has_aura(run.app.world(), run.actor, 456237));
    earn_rime(&mut run);
    cast(&mut run, 49184);
    let chamber = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 456370)
        .expect("each actual earned-Rime Howling Blast hit must build native Cryogenic Chamber");
    assert_eq!(
        chamber.stacks, 3,
        "one stack per real in-range HB recipient"
    );
    let actor = run.actor;
    super::super::enhancement_generation_tests::cast(&mut run, 51271, actor);
    for _ in 0..3 {
        run.app.update();
    }
    assert!(!auras::has_aura(run.app.world(), actor, 456370));
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    for target in &run.targets {
        let explosions = events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(456371)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert_eq!(
            explosions, 1,
            "one stored-damage explosion per actual enemy"
        );
    }
}

fn erw_full_recharge(run: &Run) -> f64 {
    let bindings = super::bindings::build();
    NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
        1,
    )
    .value("cooldown.empower_rune_weapon.full_recharge_time")
    .unwrap()
}

#[test]
#[ignore = "actual earned Rime Howling Blast extends an active Breath"]
fn frost_rime_howling_blast_extends_active_breath() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_measurement(251, 1).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let extension =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 1249658, 2);
    for _ in 0..6000 {
        if auras::has_aura(run.app.world(), actor, 59052)
            && auras::has_aura(run.app.world(), actor, 1249658)
        {
            let expiry = |run: &Run| {
                run.app
                    .world()
                    .get::<ActiveAuras>(actor)
                    .unwrap()
                    .iter()
                    .find(|aura| aura.spell_id == 1249658 && aura.caster == actor)
                    .unwrap()
                    .expires_at
                    .unwrap()
            };
            let before = expiry(&run);
            if crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                49184,
                Some(target),
                false,
            )
            .is_ok()
            {
                assert!(!auras::has_aura(run.app.world(), actor, 59052));
                let after = expiry(&run);
                assert!(
                    (after - before - f64::from(extension) / 1000.0).abs() < 0.002,
                    "actual Rime spender extends active Breath: {before} -> {after}"
                );
                return;
            }
        }
        super::super::drive_player(&mut run).unwrap();
        run.app.update();
    }
    panic!("real rotation never admitted earned Rime during Breath");
}

#[test]
#[ignore = "actual earned Rime Howling Blast grants native Rage RP"]
fn frost_rime_howling_blast_earns_rage_runic_power() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    earn_rime(&mut run);
    let actor = run.actor;
    assert!(auras::has_aura(run.app.world(), actor, 377076));
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49184, Some(target), false)
        .unwrap();
    run.app.update();
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[start..];
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(341725)
                && event.kind == CombatLogKind::Energize),
        "actual Rime consumption must trigger native Rage RP"
    );
}

#[test]
#[ignore = "native251 earned Rime must be spent by one ordinary Howling Blast"]
fn frost_howling_blast_spends_earned_rime() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    earn_rime(&mut run);
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 49184, Some(target), false)
        .expect("earned Rime admits HB without waiting for resources");
    run.app.update();
    assert!(
        !auras::has_aura(run.app.world(), run.actor, 59052),
        "one actual HB must spend its earned Rime, not leave fifteen seconds of free casts"
    );
}

#[test]
#[ignore = "native251 Rime HB applies Frostbound Will's native recharge adjustment"]
fn frost_rime_howling_blast_reduces_empower_recharge() {
    let _seed = crate::random::seed_scope(251);
    let mut run = super::super::prepare_literal(251, 1).unwrap();
    super::super::engage(&mut run);
    earn_rime(&mut run);
    let milliseconds = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1238680)
        .expect("selected native Frostbound Will")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(milliseconds < 0.0);
    cast(&mut run, 47568);
    let before = erw_full_recharge(&run);
    assert!(before > f64::from(-milliseconds) / 1000.0);
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 49184, Some(target), false)
        .unwrap();
    let after = erw_full_recharge(&run);
    let expected = before + f64::from(milliseconds) / 1000.0;
    assert!(
        (after - expected).abs() < 0.002,
        "native Rime HB recharge {before} -> {after}, expected {expected}"
    );
}
