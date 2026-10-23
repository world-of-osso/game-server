//! Concrete observation fixtures; RED is deliberately opt-in until implementation lands.
use bevy::prelude::*;
use shared::components::{CombatStatus, Mana, Position};
use shared::protocol::{CombatLogEvent, CombatLogKind};

fn receipt(
    source: Entity,
    target: Entity,
    kind: CombatLogKind,
    amount: i32,
    overflow: i32,
) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(source.to_bits()),
        target: Some(target.to_bits()),
        kind,
        amount,
        overflow,
        spell_id: Some(2061),
        school_mask: 2,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
    }
}

fn small_fight() -> serde_json::Value {
    let mut app = super::runtime::build_app();
    let copy = super::runtime::create_copy(&mut app);
    let party = crate::followers::simulator::spawn_party(
        app.world_mut(),
        copy,
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    )
    .unwrap();
    let graph = super::graph::Graph {
        map_id: 34,
        entrance: [0.0; 2],
        entrance_height: 0.0,
        entrance_pack: 0,
        packs: vec![],
        edges: vec![],
        teleports: vec![],
        unreachable: vec![],
        route: vec![],
    };
    let healer = *party
        .members
        .iter()
        .find(|&&unit| {
            app.world()
                .get::<crate::followers::Follower>(unit)
                .unwrap()
                .profile
                .role
                == shared::group::GroupRole::Healer
        })
        .unwrap();
    let mut recorder =
        super::healer_metrics::start(app.world_mut(), &graph, &party, &Default::default());
    app.world_mut()
        .entity_mut(party.tank)
        .insert(CombatStatus(true));
    app.world_mut().get_mut::<Mana>(healer).unwrap().current = 123.0;
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .advance_by(std::time::Duration::from_secs(3));
    // 200 raw healing, 50 overhealing; unrelated source must not enter healer totals.
    app.world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .extend([
            receipt(healer, party.tank, CombatLogKind::Heal, 200, 50),
            receipt(party.owner, party.tank, CombatLogKind::Heal, 999, 0),
            receipt(party.owner, party.members[2], CombatLogKind::Death, 0, 0),
        ]);
    recorder.observe_frame(app.world());
    recorder.report(app.world())["dungeon_healing"].clone()
}

#[test]
#[ignore = "RED: dungeon healing extraction not implemented"]
fn healer_benchmark_extracts_known_small_fight() {
    let metrics = small_fight();
    assert_eq!(metrics["party_deaths"], 1);
    assert_eq!(metrics["raw_healing"], 200);
    assert_eq!(metrics["effective_healing"], 150);
    assert_eq!(metrics["overheal_pct"], 25.0);
    assert_eq!(metrics["effective_hps"], 50.0);
    assert_eq!(metrics["mana_at_end"], 123.0);
    assert_eq!(metrics["combat_seconds"], 3.0);
}

#[test]
#[ignore = "RED: deterministic dungeon healing extraction not implemented"]
fn healer_benchmark_same_fight_has_identical_numbers() {
    let first = small_fight();
    assert!(
        first["effective_hps"].is_number(),
        "missing benchmark is not a deterministic result"
    );
    assert_eq!(first, small_fight());
}
