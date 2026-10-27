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
        let ticks = events.iter().filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(55095)
                && event.kind == CombatLogKind::Damage
                && event.periodic
        }).count();
        let procs = events.iter().filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(195617)
                && event.kind == CombatLogKind::Energize
        }).count();
        assert!(ticks >= 100, "actual Frost Fever needs enough ticks: {ticks}");
        let rate = procs as f64 / ticks as f64;
        assert!((minimum..maximum).contains(&rate),
            "actual {targets}T Frost Fever proc rate {rate:.3} ({procs}/{ticks}) violates pinned chance");
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
#[ignore = "diagnostic Frost native 300s factors and resource changes"]
fn diag_frost_factors_and_resource_spending() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    use crate::spell_cast::test_harness::ObservedPowerChanges;
    use shared::components::PowerType;

    let output = std::path::PathBuf::from(std::env::var("DPS_PARITY_OUTPUT").unwrap());
    let mut totals = Vec::new();
    for seed in [70448, 1, 2, 3, 4, 5, 6, 7, 8, 9] {
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
        let (seconds, _, _) = super::super::advance_combat(&mut run).unwrap();
        assert!((seconds - 300.0).abs() < 0.001);
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
        assert!(runes > 0.0 && runic_power_raw > 0.0);
        totals.push(serde_json::json!({
            "seed": seed,
            "runes": runes,
            "runic_power_raw": runic_power_raw,
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
