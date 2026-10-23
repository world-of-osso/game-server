use super::healer_intake::{Capture, Frame, Hit, HitKind, Intake};
use bevy::prelude::*;
use shared::components::Health;

#[test]
#[ignore = "RED: raw pre-mitigation spell receipt hook absent"]
fn healer_benchmark_raw_spell_receipt_precedes_native_absorb() {
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[133, 17], 5).unwrap();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 17,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let bonus_caster = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 5.0);
    h.app.insert_resource(Capture {
        members: vec![h.caster],
        sources: [(h.npc, (6, 6)), (bonus_caster, (7, 6))].into(),
        ..Default::default()
    });
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let before = h.health(h.caster);
    crate::spell_cast::effects::deal_damage_shield(
        h.app.world_mut(),
        &crate::spell_cast::effects::EffectCtx {
            caster: h.npc,
            spell: &spell,
            depth: 1,
            dest: None,
            chain_multiplier: 1.0,
        },
        bonus_caster,
        h.caster,
        200.0,
    );
    assert_eq!(
        h.app.world().get::<Health>(h.caster).unwrap().current,
        before,
        "native shield absorbs the hit"
    );
    let hits = &h.app.world().resource::<Capture>().hits;
    assert_eq!(
        hits.len(),
        1,
        "raw intake receipt must survive full absorption"
    );
    assert_eq!(hits[0].raw, 200.0);
    assert_eq!(hits[0].level_scaling_factor, 1.0);
    assert_eq!(hits[0].school, 4);
    assert_eq!(hits[0].target_slot, 0);
    assert_eq!(hits[0].source_guid, 6);
    let flags = serde_json::to_value(&hits[0].kind).unwrap();
    assert_eq!(
        flags["Spell"]["depth"], 1,
        "triggered incoming hits retain native proc eligibility"
    );
    assert_eq!(
        flags["Spell"]["bonus_source_guid"], 7,
        "shield owner and aura caster retain separate native identities"
    );
}

pub(super) fn small_intake() -> Intake {
    let _rng = crate::random::seed_scope(1);
    let graph = super::graph::read_graph().unwrap();
    let mut app = super::runtime::build_app();
    let copy = super::runtime::create_copy(&mut app);
    let party = crate::followers::simulator::spawn_party_with_healer(
        app.world_mut(),
        copy,
        shared::components::Position {
            x: graph.entrance[0],
            y: graph.entrance_height,
            z: graph.entrance[1],
        },
        32,
        105,
    )
    .unwrap();
    let actors = super::runtime::spawn_population(&mut app, copy);
    let sources = super::healer_intake::snapshot_sources(app.world(), &actors).unwrap();
    let pack = graph
        .packs
        .iter()
        .find(|pack| pack.id == graph.route[0].pack)
        .unwrap();
    let attacker = crate::level_scaling::UnitScaling::of(app.world(), actors[&pack.spawns[0].guid]);
    let target = crate::level_scaling::UnitScaling::of(app.world(), party.tank);
    let mapped = crate::level_scaling::damage_dealt(&attacker, &target, 1_000_000.0);
    assert!(
        mapped > 0.0,
        "fixture needs a positive native damage mapping"
    );
    // Fixture severity, not a game coefficient: 80% of native HP before armor/DR.
    let raw = app.world().get::<Health>(party.tank).unwrap().max
        * target.health_multiplier(&attacker)
        * 0.8
        * 1_000_000.0
        / mapped;
    let level_difference =
        i32::from(target.level_against(&attacker)) - i32::from(attacker.level_against(&target));
    let hit = Hit {
        tick: 2,
        source_guid: pack.spawns[0].guid,
        pack: pack.id,
        target_slot: 0,
        school: 1,
        raw,
        level_scaling_factor: crate::level_scaling::damage_to_hp_scale(
            app.world(),
            actors[&pack.spawns[0].guid],
            party.tank,
        ),
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference,
            critical_multiplier: 1.0,
            weapon_speed: crate::combat::AUTO_ATTACK_INTERVAL,
        },
    };
    Intake {
        schema: 1,
        seed: 1,
        map_id: 34,
        difficulty_id: 1,
        level: 32,
        end_tick: 120,
        source_status: "concrete_fixture".into(),
        hits: vec![hit],
        sources,
        frames: vec![Frame {
            tick: 0,
            positions: vec![Some([graph.entrance[0], graph.entrance_height, graph.entrance[1]]); 5],
            in_combat: true,
        }],
        pulls: serde_json::json!([{ "pack":pack.id,"start_tick":1,"end_tick":120 }]),
        source_deaths: vec![],
        gaps: vec![],
    }
}

#[test]
#[ignore = "RED: native raw-hit replay absent"]
fn healer_benchmark_same_intake_replays_identical_native_numbers() {
    let intake = small_intake();
    let first = super::healer_replay::replay(&intake, 105).unwrap();
    assert!(first["metrics"]["effective_hps"].is_number());
    assert!(
        first["metrics"]["effective_healing"].as_u64().unwrap() > 0,
        "determinism must exercise real native healing, not identical zero traces"
    );
    assert_eq!(first, super::healer_replay::replay(&intake, 105).unwrap());
}

#[test]
#[ignore = "RED: recorded native source scaling not restored"]
fn healer_benchmark_restores_recorded_source_scaling_before_native_damage() {
    let _rng = crate::random::seed_scope(1);
    let mut app = super::runtime::build_app();
    let graph = app.world().resource::<super::Space>().0.clone();
    let copy = super::runtime::create_copy(&mut app);
    let party = crate::followers::simulator::spawn_party_with_healer(
        app.world_mut(),
        copy,
        shared::components::Position {
            x: graph.entrance[0],
            y: graph.entrance_height,
            z: graph.entrance[1],
        },
        32,
        105,
    )
    .unwrap();
    let actors = super::runtime::spawn_population(&mut app, copy);
    let source = *actors
        .values()
        .find(|&&unit| {
            app.world()
                .get::<shared::components::Npc>(unit)
                .unwrap()
                .template_id
                == 46248
        })
        .expect("native Riverpaw Basher");
    let snapshots = super::healer_intake::snapshot_sources(app.world(), &actors).unwrap();
    let native_damage = |world: &World| {
        let attacker = crate::level_scaling::UnitScaling::of(world, source);
        let target = crate::level_scaling::UnitScaling::of(world, party.tank);
        (
            attacker.level_against(&target),
            crate::level_scaling::damage_dealt(&attacker, &target, 1_000_000.0),
        )
    };
    let before = native_damage(app.world());
    // Native46248 template and Normal difficulty rows both source delta0..1.
    // The alternate legal spawn roll reproduces a different RNG prefix.
    let mut scaling = app
        .world_mut()
        .get_mut::<shared::level_scaling::LevelScaling>(source)
        .unwrap();
    assert!(matches!(scaling.delta, 0 | 1));
    scaling.delta = 1 - scaling.delta;
    assert_ne!(
        native_damage(app.world()),
        before,
        "fixture must exercise actual source-level damage mapping"
    );
    super::healer_intake::apply_source_snapshots(app.world_mut(), &snapshots, &actors).unwrap();
    assert_eq!(
        native_damage(app.world()),
        before,
        "identical recorded raw damage must retain identical native source-level inputs for every healer"
    );
}

#[test]
#[ignore = "RED: recorded combat denominator missing"]
fn healer_benchmark_reports_frozen_combat_denominator() {
    let mut intake = small_intake();
    intake.end_tick = 20;
    let mut out_of_combat = intake.frames[0].clone();
    out_of_combat.tick = 11;
    out_of_combat.in_combat = false;
    intake.frames.push(out_of_combat);
    let report = super::healer_replay::replay(&intake, 105).unwrap();
    let metrics = &report["metrics"];
    assert_eq!(metrics["recorded_combat_seconds"], 0.5);
    assert!(metrics["native_combat_seconds"].is_number());
    assert_eq!(
        metrics["effective_hps"].as_f64().unwrap(),
        metrics["effective_healing"].as_f64().unwrap() / 0.5
    );
}

#[test]
#[ignore = "RED: actual recorded Stockade physical spell block unsupported"]
fn healer_benchmark_replays_recorded_stockade_spell_block() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/healer-benchmark/stockade-seed1.intake.json");
    let intake: Intake = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let blocked = intake
        .hits
        .iter()
        .find(|hit| {
            matches!(
                hit.kind,
                HitKind::Spell {
                    spell_id: 86727,
                    blocked: true,
                    ..
                }
            )
        })
        .expect("actual captured Strike block")
        .clone();
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[86727], 1).unwrap();
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    let mut unblocked = blocked.clone();
    if let HitKind::Spell { blocked, .. } = &mut unblocked.kind {
        *blocked = false;
    }
    let before = h.health(h.npc);
    crate::spell_cast::effects::replay_healer_hit(h.app.world_mut(), h.caster, h.npc, &unblocked)
        .unwrap();
    let full = before - h.health(h.npc);
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = before;
    crate::spell_cast::effects::replay_healer_hit(h.app.world_mut(), h.caster, h.npc, &blocked)
        .unwrap();
    let loss = before - h.health(h.npc);
    let native_block = (full * crate::melee::CREATURE_BLOCKED_PCT / 100.0).trunc();
    assert!(native_block > 0.0);
    assert_eq!(
        loss,
        full - native_block,
        "recorded block uses native creature block amount exactly once"
    );
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(events.iter().any(
        |event| event.kind == shared::protocol::CombatLogKind::Damage
            && event.blocked == native_block as i32
    ));
}

#[test]
#[ignore = "explicit native recipient mitigation replay control"]
fn healer_benchmark_raw_spell_replay_applies_recipient_reduction_and_absorb_once() {
    for (aura, expected_loss) in [(None, 200.0), (Some(102342), 160.0), (Some(17), 0.0)] {
        let mut h =
            crate::spell_cast::test_harness::harness(Vec::new(), &[133, 17, 102342], 5).unwrap();
        if let Some(spell_id) = aura {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: h.caster,
                    target: h.caster,
                    spell_id,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        let hit = Hit {
            tick: 1,
            source_guid: 6,
            pack: 6,
            target_slot: 0,
            school: 4,
            raw: 200.0,
            level_scaling_factor: crate::level_scaling::damage_to_hp_scale(
                h.app.world(),
                h.npc,
                h.caster,
            ),
            kind: HitKind::Spell {
                spell_id: 133,
                depth: 0,
                bonus_source_guid: None,
                effect_index: None,
                periodic: false,
                crit: false,
                resolved_source: false,
                blocked: false,
            },
        };
        let before = h.health(h.caster);
        crate::spell_cast::effects::replay_healer_hit(h.app.world_mut(), h.npc, h.caster, &hit)
            .unwrap();
        assert_eq!(
            before - h.health(h.caster),
            expected_loss,
            "aura{aura:?}: fixed raw200 must use replay recipient mitigation"
        );
    }
}

#[test]
#[ignore = "explicit native melee armor and shield replay control"]
fn healer_benchmark_raw_melee_replay_applies_native_armor_and_shield() {
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[17], 5).unwrap();
    h.app
        .insert_resource(crate::test_data::world_data().armor_constants.clone());
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .armor = 5000.0;
    let hit = Hit {
        tick: 1,
        source_guid: 6,
        pack: 6,
        target_slot: 0,
        school: 1,
        raw: 200.0,
        level_scaling_factor: crate::level_scaling::damage_to_hp_scale(
            h.app.world(),
            h.npc,
            h.caster,
        ),
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    let before = h.health(h.caster);
    crate::combat::replay_healer_hit(h.app.world_mut(), h.npc, h.caster, &hit).unwrap();
    let loss = before - h.health(h.caster);
    assert!(
        loss > 0.0 && loss < 200.0,
        "native armor must reduce raw200, observed{loss}"
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 17,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let shielded = h.health(h.caster);
    crate::combat::replay_healer_hit(h.app.world_mut(), h.npc, h.caster, &hit).unwrap();
    assert_eq!(
        h.health(h.caster),
        shielded,
        "same raw200 must reach native shield after armor"
    );
}
