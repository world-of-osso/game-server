use super::nalorakk_offerings::update_offerings;
use super::spellclick::{cast_spellclick, load_spellclick_catalog};
use crate::maps::InMap;
use crate::spell_cast::test_harness::harness;
use shared::components::{Health, Npc, Position, UnitFlags};

#[test]
fn nalorakk_six_avatar_deliveries_unlock_only_their_instance() {
    let mut h = harness(Vec::new(), &[1239001, 1239428, 1249737], 2).unwrap();
    h.app
        .add_systems(bevy::prelude::FixedUpdate, update_offerings);
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_spellclick_catalog(&conn).unwrap());
    let player = h.caster;
    let bush = h.npc;
    let map = InMap {
        map_id: 2825,
        instance_id: 47,
    };
    let other_map = InMap {
        map_id: 2825,
        instance_id: 48,
    };
    h.app.world_mut().entity_mut(player).insert(map);
    h.app.world_mut().entity_mut(bush).insert((
        map,
        Npc {
            template_id: 244941,
            name: "Berry Bush".into(),
        },
    ));
    let immunity = UnitFlags::IMMUNE_TO_PC | UnitFlags::IMMUNE_TO_NPC;
    let mut avatars = Vec::new();
    for index in 0..6 {
        let avatar = h
            .app
            .world_mut()
            .spawn((
                Npc {
                    template_id: 241805,
                    name: "Avatar of Starvation".into(),
                },
                Position {
                    x: 20.0 + index as f32 * 10.0,
                    y: 0.0,
                    z: 0.0,
                },
                map,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        avatars.push(avatar);
    }
    let boss = h
        .app
        .world_mut()
        .spawn((
            Npc {
                template_id: 241812,
                name: "Hoardmonger".into(),
            },
            map,
            UnitFlags(immunity),
        ))
        .id();
    let other = h
        .app
        .world_mut()
        .spawn((
            Npc {
                template_id: 241812,
                name: "Hoardmonger".into(),
            },
            other_map,
            UnitFlags(immunity),
        ))
        .id();
    update_offerings(h.app.world_mut());
    for avatar in &avatars {
        assert!(crate::auras::has_aura(h.app.world(), *avatar, 1249737));
    }
    for (index, avatar) in avatars.iter().enumerate() {
        let x = 20.0 + index as f32 * 10.0;
        h.app.world_mut().entity_mut(player).insert(Position {
            x: x - 8.0,
            y: 0.0,
            z: 0.0,
        });
        h.app.world_mut().entity_mut(bush).insert(Position {
            x: x - 6.0,
            y: 0.0,
            z: 0.0,
        });
        cast_spellclick(h.app.world_mut(), player, bush, 244941);
        h.advance(80);
        assert!(crate::auras::has_aura(h.app.world(), player, 1239428));
        h.app
            .world_mut()
            .entity_mut(player)
            .insert(Position { x, y: 0.0, z: 0.0 });
        h.advance(3);
        assert!(!crate::auras::has_aura(h.app.world(), player, 1239428));
        assert!(!crate::auras::has_aura(h.app.world(), *avatar, 1249737));
        let flags = h.app.world().get::<UnitFlags>(boss).unwrap().0;
        assert_eq!(flags & immunity, if index == 5 { 0 } else { immunity });
        assert_eq!(
            h.app.world().get::<UnitFlags>(other).unwrap().0 & immunity,
            immunity
        );
    }
}

#[test]
fn interaction_objective_unlocks_hoardmonger_through_six_native_deliveries() {
    unlock_hoardmonger_through_objective(244941, "Berry Bush", 1239001);
}

/// Fishing is a channel: its native cast success, and therefore the carry, lands at channel start.
#[test]
fn interaction_objective_accepts_channelled_fishing_pickup() {
    unlock_hoardmonger_through_objective(245698, "Fishing Spot", 1239227);
}

fn unlock_hoardmonger_through_objective(source: u32, name: &str, pickup: u32) {
    use crate::test_support::objectives::{self, InteractionStep};
    let mut h = harness(Vec::new(), &[pickup, 1239428, 1249737], 2).unwrap();
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    objectives::install(&mut h.app, &conn).unwrap();
    let map = InMap {
        map_id: 2825,
        instance_id: 47,
    };
    let owner = h.caster;
    h.app.world_mut().entity_mut(owner).insert(map);
    h.app.world_mut().entity_mut(h.npc).insert((
        map,
        Npc {
            template_id: source,
            name: name.into(),
        },
    ));
    for index in 0..6 {
        h.app.world_mut().spawn((
            map,
            Npc {
                template_id: 241805,
                name: "Avatar".into(),
            },
            Position {
                x: 20.0 + index as f32 * 10.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
        ));
    }
    let immunity = UnitFlags::IMMUNE_TO_PC | UnitFlags::IMMUNE_TO_NPC;
    let boss = h
        .app
        .world_mut()
        .spawn((
            map,
            Npc {
                template_id: 241812,
                name: "Hoardmonger".into(),
            },
            UnitFlags(immunity),
        ))
        .id();
    // Fishing is SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL; the route driver offers it out of combat.
    crate::combat::combat_stop(h.app.world_mut(), owner);
    // Teleported approaches never interrupt the 22s fishing channel, so each offering can wait it out.
    for _ in 0..3000 {
        match objectives::next(h.app.world_mut(), owner, boss)
            .unwrap()
            .unwrap()
        {
            InteractionStep::Approach { target } => {
                let position = *h.app.world().get::<Position>(target).unwrap();
                h.app.world_mut().entity_mut(owner).insert(position);
            }
            InteractionStep::Interact { target } => {
                objectives::interact(h.app.world_mut(), owner, target).unwrap()
            }
            InteractionStep::Wait => {}
        }
        h.advance(1);
        if h.app.world().get::<UnitFlags>(boss).unwrap().0 & immunity == 0 {
            assert!(
                objectives::next(h.app.world_mut(), owner, boss)
                    .unwrap()
                    .is_none()
            );
            return;
        }
    }
    panic!("native six-offering interaction objective did not unlock Hoardmonger");
}

#[test]
fn nalorakk_pickup_requires_a_completed_cast() {
    let mut h = harness(Vec::new(), &[1239001, 1239428], 2).unwrap();
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_spellclick_catalog(&conn).unwrap());
    let player = h.caster;
    let bush = h.npc;
    let map = InMap {
        map_id: 2825,
        instance_id: 47,
    };
    h.app.world_mut().entity_mut(player).insert(map);
    h.app.world_mut().entity_mut(bush).insert((
        map,
        Npc {
            template_id: 244941,
            name: "Berry Bush".into(),
        },
    ));
    cast_spellclick(h.app.world_mut(), player, bush, 244941);
    crate::spell_cast::cancel_cast(h.app.world_mut(), player);
    update_offerings(h.app.world_mut());
    assert!(!crate::auras::has_aura(h.app.world(), player, 1239428));
    cast_spellclick(h.app.world_mut(), player, bush, 244941);
    cast_spellclick(h.app.world_mut(), player, bush, 244941);
    h.advance(80);
    assert!(crate::auras::has_aura(h.app.world(), player, 1239428));
}
