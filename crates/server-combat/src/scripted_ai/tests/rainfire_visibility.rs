//! Actual Stockade boss cast, area damage and ordinary interest visibility.
use super::*;
use crate::interest::{InterestPlugin, NetworkVisibility};
use crate::spell_cast::area_aura::DynamicObject;

fn rainfire_harness() -> (Harness, Entity, Entity) {
    let data = data().expect("Stockade world data");
    let mut app = stockade_app_with(data, |app| {
        app.add_plugins(InterestPlugin);
    });
    let boss = spawn_boss(&mut app, data, NPC_LORD_OVERHEAT, COPY);
    let spawn = data
        .spawns
        .iter()
        .find(|row| row.template_entry == NPC_LORD_OVERHEAT)
        .unwrap();
    let near = [
        spawn.position_x + 3.0,
        spawn.position_y,
        spawn.position_z,
        0.0,
    ];
    let players = [COPY, OTHER_COPY]
        .into_iter()
        .enumerate()
        .map(|(index, map)| {
            let (player, connection) = spawn_player(&mut app, map, near, index as u64 + 1);
            app.world_mut()
                .entity_mut(connection)
                .insert(ReplicationSender);
            (player, connection, Inbox::default())
        })
        .collect();
    let mut harness = Harness { app, players };
    harness.advance(0.05);
    harness.engage(boss);
    for _ in 0..800 {
        harness.advance(0.05);
        let object = harness
            .app
            .world_mut()
            .query::<(Entity, &DynamicObject)>()
            .iter(harness.app.world())
            .find(|(_, area)| area.spell_id == 86636)
            .map(|(entity, _)| entity);
        if let Some(object) = object {
            harness.advance(0.1);
            return (harness, boss, object);
        }
    }
    panic!("Native Lord Overheat never created Rain of Fire in40s");
}

#[test]
fn rainfire_actual_area_reaches_same_copy_client_not_other_copy() {
    let (mut h, boss, object) = rainfire_harness();
    let area = h.app.world().get::<DynamicObject>(object).unwrap();
    assert_eq!(area.caster, boss);
    assert_eq!(area.radius, 5.0, "active70448 SpellRadius8");
    assert_eq!(area.expires_at - area.started_at, 9.0, "SpellDuration105");
    let center = *h.app.world().get::<Position>(object).unwrap();
    assert_eq!(center, *h.app.world().get::<Position>(h.player()).unwrap());
    let received = h
        .app
        .world()
        .get::<shared::components::DynamicObjectInfo>(object)
        .unwrap();
    assert_eq!(received.caster, boss.to_bits());
    assert_eq!(received.spell_id, 86636);
    assert_eq!(received.radius, area.radius);
    assert!(
        h.app.world().get::<Replicate>(object).is_some(),
        "actual Rain of Fire must be replicated"
    );
    let visibility = h
        .app
        .world()
        .get::<NetworkVisibility>(object)
        .expect("ordinary area interest visibility");
    assert!(
        visibility.is_visible(h.players[0].1),
        "same-copy client sees area"
    );
    assert!(
        !visibility.is_visible(h.players[1].1),
        "other copy cannot see area"
    );
    h.advance(3.1);
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(86636)
                && event.target == Some(h.player().to_bits()))
    );
    assert!(
        !h.inbox(1)
            .log
            .iter()
            .any(|event| event.spell_id == Some(86636))
    );
    // Fixture movement tests the real area leave/damage boundary, not native input delivery.
    let player = h.player();
    h.app.world_mut().get_mut::<Position>(player).unwrap().x += 12.0;
    h.advance(0.1);
    let previous = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(86636))
        .count();
    h.advance(3.1);
    assert_eq!(
        h.inbox(0)
            .log
            .iter()
            .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(86636))
            .count(),
        previous,
        "leaving removes periodic damage"
    );
    h.advance(3.1);
    assert!(
        h.app.world().get_entity(object).is_err(),
        "actual9s expiry despawns replicated area"
    );
}

#[test]
fn rainfire_actual_area_removed_when_caster_despawns() {
    let (mut h, boss, object) = rainfire_harness();
    h.app.world_mut().entity_mut(boss).despawn();
    h.advance(0.1);
    assert!(
        h.app.world().get_entity(object).is_err(),
        "no orphan area after caster removal"
    );
}
