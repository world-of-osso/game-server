use super::*;
use shared::components::GuildMembership;
use shared::reputation::{CharacterReputation, default_faction_registry};

#[test]
fn guild_mobile_banking_chest_user_needs_membership_and_friendly_reputation() {
    let mut h = harness(HUMAN_TEMPLATE).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(pair::<UseGameObject>());
    let (entry, go_type, display_id, name, scale) = h
        .conn
        .query_row(
            "SELECT entry,type,displayId,name,size FROM tdb_gameobject_template WHERE entry=206602",
            [],
            |r| {
                Ok((
                    r.get::<_, u32>(0)?,
                    r.get::<_, u8>(1)?,
                    r.get::<_, u32>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, f32>(4)?,
                ))
            },
        )
        .unwrap();
    let bounds = h.conn.query_row(
        "SELECT GeoBox_0,GeoBox_1,GeoBox_2,GeoBox_3,GeoBox_4,GeoBox_5 FROM game_object_display_info WHERE ID=?1", [display_id],
        |r| Ok(GameObjectBounds {
            min: Vec3::new(r.get(0)?, r.get(1)?, r.get(2)?),
            max: Vec3::new(r.get(3)?, r.get(4)?, r.get(5)?), scale, orientation: 0.0,
        }),
    ).unwrap();
    let object = h
        .app
        .world_mut()
        .spawn((
            GameObjectInfo {
                entry,
                go_type,
                display_id,
                name,
                scale,
            },
            bounds,
            Position {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
            crate::maps::InMap::continent(0),
            crate::game_objects::summoned::SummonedObject {
                owner: Some(h.player),
                spell_id: 88304,
                expires_at: None,
            },
        ))
        .id();
    for (member, reputation, allowed) in [
        (false, 42000, false),
        (true, 0, false),
        (true, 2999, false),
        (true, 3000, true),
    ] {
        let mut rep = CharacterReputation::new_for_race(&default_faction_registry(), 1);
        rep.set_rep(1168, reputation);
        h.app.world_mut().entity_mut(h.player).insert(rep);
        if member {
            h.app
                .world_mut()
                .entity_mut(h.player)
                .insert(GuildMembership {
                    guild_id: 1,
                    guild_name: "Mobile Guild".into(),
                });
        } else {
            h.app
                .world_mut()
                .entity_mut(h.player)
                .remove::<GuildMembership>();
        }
        h.send::<_, InteractionChannel>(UseGameObject {
            object: object.to_bits(),
        });
        let opened = h.take::<InteractionOpened>();
        let failed = h.take::<InteractionFailed>();
        if allowed {
            assert_eq!(
                opened,
                vec![InteractionOpened {
                    npc: object.to_bits(),
                    kind: InteractionKind::Role(NpcRole::GuildBanker)
                }]
            );
            assert!(failed.is_empty());
        } else {
            assert!(
                opened.is_empty(),
                "member={member} rep={reputation} must not open chest"
            );
            assert_eq!(failed.len(), 1);
            assert!(h.session().is_none());
        }
    }
    h.app
        .world_mut()
        .entity_mut(h.player)
        .remove::<GuildMembership>();
    h.advance();
    assert!(
        h.session().is_none(),
        "membership revocation closes mobile vault"
    );
    assert!(!h.take::<InteractionClosed>().is_empty());
}
