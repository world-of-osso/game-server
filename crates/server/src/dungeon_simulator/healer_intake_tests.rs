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
    h.app.insert_resource(Capture {
        members: vec![h.caster],
        sources: [(h.npc, (6, 6))].into(),
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
    crate::spell_cast::effects::deal_damage(
        h.app.world_mut(),
        &crate::spell_cast::effects::EffectCtx {
            caster: h.npc,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        },
        h.caster,
        200.0,
        false,
        false,
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
    assert_eq!(hits[0].school, 4);
    assert_eq!(hits[0].target_slot, 0);
    assert_eq!(hits[0].source_guid, 6);
}

pub(super) fn small_intake() -> Intake {
    let graph = super::graph::read_graph().unwrap();
    let pack = graph
        .packs
        .iter()
        .find(|pack| pack.id == graph.route[0].pack)
        .unwrap();
    let hit = Hit {
        tick: 2,
        source_guid: pack.spawns[0].guid,
        pack: pack.id,
        target_slot: 0,
        school: 1,
        raw: 80.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    Intake {
        schema: 1,
        seed: 1,
        map_id: 34,
        difficulty_id: 1,
        level: 32,
        end_tick: 40,
        source_status: "concrete_fixture".into(),
        hits: vec![hit.clone(), Hit { tick: 5, ..hit }],
        frames: vec![Frame {
            tick: 0,
            positions: vec![Some([graph.entrance[0], graph.entrance_height, graph.entrance[1]]); 5],
            in_combat: true,
        }],
        pulls: serde_json::json!([{ "pack":pack.id,"start_tick":1,"end_tick":40 }]),
        gaps: vec![],
    }
}

#[test]
#[ignore = "RED: native raw-hit replay absent"]
fn healer_benchmark_same_intake_replays_identical_native_numbers() {
    let intake = small_intake();
    let first = super::healer_replay::replay(&intake, 105).unwrap();
    assert!(first["metrics"]["effective_hps"].is_number());
    assert_eq!(first, super::healer_replay::replay(&intake, 105).unwrap());
}
