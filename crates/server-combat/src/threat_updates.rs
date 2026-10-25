//! Amounts missing from UnitThreatList: TC a352b1fa ThreatManager.cpp:829-875.
//! Coalesce changes at 4 Hz; empty tables clear previously engaged clients.

use std::collections::HashMap;

use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::components::{Npc, Player, Position, ThreatTable};
use shared::protocol::{CombatChannel, ThreatUnit, ThreatUpdate};

use crate::auth::Authenticated;
use crate::networking::{CombatTarget, PlayerLink};

#[derive(Resource, Default)]
struct ThreatDelivery {
    next_send: f64,
    sent: HashMap<Entity, HashMap<u64, ThreatUpdate>>,
}

pub struct ThreatUpdatesPlugin;

impl Plugin for ThreatUpdatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ThreatDelivery>()
            .add_systems(PostUpdate, send_threat_updates);
    }
}

fn percent(raw: f32, victim: f32) -> f32 {
    if victim > 0.0 {
        raw / victim * 100.0
    } else {
        0.0
    }
}

fn snapshot(world: &World, creature: Entity, table: &ThreatTable) -> ThreatUpdate {
    let victim = world
        .get::<CombatTarget>(creature)
        .and_then(|target| target.0)
        .map(Entity::to_bits);
    let victim_threat = victim.map_or(0.0, |id| table.threat_for(id));
    let highest = table.top_threat().map_or(0.0, |entry| entry.threat);
    let creature_pos = world.get::<Position>(creature);
    let mut entries: Vec<ThreatUnit> = table
        .entries()
        .iter()
        .map(|entry| {
            let unit = Entity::from_bits(entry.entity);
            let player = world.get::<Player>(unit);
            let npc = world.get::<Npc>(unit);
            let tanking = victim == Some(entry.entity);
            let status = match (tanking, entry.threat > victim_threat) {
                (true, _) => {
                    if highest > entry.threat {
                        2
                    } else {
                        3
                    }
                }
                (false, true) => 1,
                (false, false) => 0,
            };
            let in_melee = creature_pos
                .zip(world.get::<Position>(unit))
                .is_some_and(|(a, b)| {
                    let (x, y, z) = (a.x - b.x, a.y - b.y, a.z - b.z);
                    x * x + y * y + z * z <= 25.0
                });
            let raw_percent = if tanking {
                100.0
            } else {
                percent(entry.threat, victim_threat)
            };
            let scaled_percent = if tanking {
                100.0
            } else {
                (raw_percent / if in_melee { 1.1 } else { 1.3 }).min(100.0)
            };
            ThreatUnit {
                unit: entry.entity,
                name: player
                    .map(|player| player.name.clone())
                    .or_else(|| npc.map(|npc| npc.name.clone()))
                    .unwrap_or_else(|| entry.entity.to_string()),
                class_id: player.map_or(0, |player| player.class),
                raw_threat: entry.threat,
                status,
                raw_percent,
                scaled_percent,
            }
        })
        .collect();
    entries.sort_by(|a, b| {
        b.raw_threat
            .total_cmp(&a.raw_threat)
            .then(a.unit.cmp(&b.unit))
    });
    ThreatUpdate {
        creature: creature.to_bits(),
        victim,
        entries,
    }
}

fn send_threat_updates(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    if now < world.resource::<ThreatDelivery>().next_send {
        return;
    }
    let tables: Vec<(Entity, ThreatTable)> = world
        .query::<(Entity, &ThreatTable)>()
        .iter(world)
        .filter(|(_, table)| !table.is_empty())
        .map(|(unit, table)| (unit, table.clone()))
        .collect();
    let mut by_player: HashMap<u64, HashMap<u64, ThreatUpdate>> = HashMap::new();
    for (creature, table) in tables {
        let update = snapshot(world, creature, &table);
        for entry in &update.entries {
            if world.get::<Player>(Entity::from_bits(entry.unit)).is_some() {
                by_player
                    .entry(entry.unit)
                    .or_default()
                    .insert(update.creature, update.clone());
            }
        }
    }
    let connections: HashMap<Entity, Entity> = world
        .query_filtered::<(Entity, &PlayerLink), (With<Authenticated>, With<MessageSender<ThreatUpdate>>)>()
        .iter(world)
        .map(|(connection, link)| (connection, link.0))
        .collect();
    let mut delivery = world
        .remove_resource::<ThreatDelivery>()
        .expect("ThreatDelivery initialized");
    delivery.next_send = now + 0.25;
    delivery
        .sent
        .retain(|connection, _| connections.contains_key(connection));
    for (connection, player) in connections {
        let current = by_player
            .get(&player.to_bits())
            .cloned()
            .unwrap_or_default();
        let prior = delivery.sent.entry(connection).or_default();
        let mut sender = world
            .get_mut::<MessageSender<ThreatUpdate>>(connection)
            .expect("queried sender");
        for (creature, update) in &current {
            if prior.get(creature) != Some(update) {
                sender.send::<CombatChannel>(update.clone());
            }
        }
        for creature in prior
            .keys()
            .filter(|creature| !current.contains_key(creature))
        {
            sender.send::<CombatChannel>(ThreatUpdate {
                creature: *creature,
                victim: None,
                entries: vec![],
            });
        }
        *prior = current;
    }
    world.insert_resource(delivery);
}

#[cfg(test)]
#[path = "threat_updates_tests.rs"]
mod tests;
