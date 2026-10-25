use super::*;
use crate::creature_ai::CreatureState;
use crate::spell_cast::test_harness::{
    Harness, drain, harness_with_plugins, spawn_connection, spawn_player,
};
use lightyear::prelude::MessageReceiver;

fn receive(h: &mut Harness, connection: Entity, frames: usize) -> Vec<ThreatUpdate> {
    let mut received = Vec::new();
    for _ in 0..frames {
        h.advance(1);
        received.extend(drain::<ThreatUpdate>(&mut h.app, connection));
    }
    received
}

fn connect(h: &mut Harness, player: Entity) -> Entity {
    let connection = spawn_connection(&mut h.app, player);
    h.app.world_mut().entity_mut(connection).insert((
        MessageSender::<ThreatUpdate>::default(),
        MessageReceiver::<ThreatUpdate>::default(),
    ));
    connection
}

#[test]
fn threat_updates_reach_engaged_players_not_spectators_and_clear_after_reset() {
    let mut h = harness_with_plugins(Vec::new(), &[], 1, false, |app| {
        app.add_plugins(ThreatUpdatesPlugin);
    })
    .expect("world.db fixture");
    let tank = h.caster;
    let healer = spawn_player(&mut h.app, 5, 20.0);
    let spectator = spawn_player(&mut h.app, 8, 25.0);
    let tank_connection = connect(&mut h, tank);
    let healer_connection = connect(&mut h, healer);
    let spectator_connection = connect(&mut h, spectator);
    let creature = h.npc;
    h.app
        .world_mut()
        .entity_mut(creature)
        .insert((ThreatTable::default(), CreatureState::Idle));
    crate::creature_aggro::add_threat(h.app.world_mut(), creature, tank, 200.0);
    crate::creature_aggro::add_healing_threat(h.app.world_mut(), healer, tank, 100.0, 2);
    // The real damage entry is 1:1; the healed unit's enemy gains 0.5:1 heal threat.
    let table = h.app.world().get::<ThreatTable>(creature).unwrap();
    assert_eq!(table.threat_for(tank.to_bits()), 200.0);
    assert_eq!(table.threat_for(healer.to_bits()), 50.0);
    let mut tanks = Vec::new();
    let mut healers = Vec::new();
    for _ in 0..10 {
        h.advance(1);
        tanks.extend(drain::<ThreatUpdate>(&mut h.app, tank_connection));
        healers.extend(drain::<ThreatUpdate>(&mut h.app, healer_connection));
        assert!(drain::<ThreatUpdate>(&mut h.app, spectator_connection).is_empty());
    }
    assert_eq!(tanks.len(), 1);
    assert_eq!(healers, tanks);
    let update = &tanks[0];
    assert_eq!(update.creature, creature.to_bits());
    assert_eq!(update.victim, Some(tank.to_bits()));
    assert_eq!(update.entries[0].raw_threat, 200.0);
    assert_eq!(update.entries[1].raw_percent, 25.0);
    assert_eq!(update.entries[1].scaled_percent, 25.0 / 1.3);
    assert_eq!((update.entries[0].status, update.entries[1].status), (3, 0));
    // No periodic spam of identical tables.
    assert!(receive(&mut h, tank_connection, 10).is_empty());
    h.app
        .world_mut()
        .get_mut::<ThreatTable>(creature)
        .unwrap()
        .reset();
    let cleared = receive(&mut h, tank_connection, 10);
    assert_eq!(
        cleared,
        vec![ThreatUpdate {
            creature: creature.to_bits(),
            victim: None,
            entries: vec![]
        }]
    );
}

#[test]
fn threat_righteous_fury_uses_loaded_aura_multiplier() {
    let mut h =
        harness_with_plugins(Vec::new(), &[25780], 2, false, |_| {}).expect("world.db fixture");
    let tank = h.caster;
    let creature = h.npc;
    h.app
        .world_mut()
        .entity_mut(creature)
        .insert((ThreatTable::default(), CreatureState::Idle));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: tank,
            target: tank,
            spell_id: 25780,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    // Pinned retail world.db: RF 25780 MOD_THREAT=950%, mask=127 (all schools).
    crate::creature_aggro::add_school_threat(h.app.world_mut(), creature, tank, 100.0, 2);
    assert_eq!(
        h.app
            .world()
            .get::<ThreatTable>(creature)
            .unwrap()
            .threat_for(tank.to_bits()),
        1050.0
    );
}

#[test]
fn threat_healing_splits_across_healed_units_enemies_not_unrelated_mobs() {
    let mut h = harness_with_plugins(Vec::new(), &[], 1, false, |_| {}).expect("world.db fixture");
    let tank = h.caster;
    let healer = spawn_player(&mut h.app, 5, 20.0);
    let other = spawn_player(&mut h.app, 1, 30.0);
    let spawn = |h: &mut Harness, victim: Entity| {
        let mut table = ThreatTable::default();
        table.add_damage_threat(victim.to_bits(), 200.0, 1.0);
        h.app
            .world_mut()
            .spawn((
                table,
                CreatureState::Pursuing {
                    target: victim.to_bits(),
                },
                shared::components::Health {
                    current: 1000.0,
                    max: 1000.0,
                },
            ))
            .id()
    };
    let a = spawn(&mut h, tank);
    let b = spawn(&mut h, tank);
    let unrelated = spawn(&mut h, other);
    crate::creature_aggro::add_healing_threat(h.app.world_mut(), healer, tank, 100.0, 2);
    for creature in [a, b] {
        assert_eq!(
            h.app
                .world()
                .get::<ThreatTable>(creature)
                .unwrap()
                .threat_for(healer.to_bits()),
            25.0
        );
    }
    assert_eq!(
        h.app
            .world()
            .get::<ThreatTable>(unrelated)
            .unwrap()
            .threat_for(healer.to_bits()),
        0.0
    );
}
