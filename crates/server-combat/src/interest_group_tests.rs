use super::*;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::persistence::ClientId;

#[test]
fn toyfx28_private_group_membership_changes_visibility_without_movement() {
    let mut app = setup_interest_app();
    app.init_resource::<GroupRegistry>();
    let connections: Vec<_> = (1..=4)
        .map(|id| {
            let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
            let player = app.world().get::<PlayerLink>(connection).unwrap().0;
            app.world_mut().entity_mut(player).insert(ClientId(id));
            (player, connection)
        })
        .collect();
    let online =
        OnlineView::from_iter(connections.iter().enumerate().map(|(index, (player, _))| {
            let id = index as u64 + 1;
            (
                id,
                OnlineMember {
                    entity: player.to_bits(),
                    name: format!("Member{id}"),
                    class: 8,
                    level: 80,
                    portrait: default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: default(),
                },
            )
        }));
    let group = {
        let mut registry = app.world_mut().resource_mut::<GroupRegistry>();
        for member in [2, 3] {
            registry.invite(1, &format!("Member{member}"), &online);
            registry.respond_invite(member, true);
        }
        registry.group_id(1).unwrap()
    };
    let target = spawn_interest_target(&mut app, pos(2.0, 0.0, 0.0));
    app.world_mut()
        .entity_mut(target)
        .insert(PrivateObjectGroup(group));
    run_fixed_ticks(&mut app, 1);
    for (_, connection) in &connections[..3] {
        assert_visible(
            app.world().get::<NetworkVisibility>(target).unwrap(),
            *connection,
            "group member",
        );
    }
    assert_not_visible(
        app.world().get::<NetworkVisibility>(target).unwrap(),
        connections[3].1,
        "stranger",
    );

    app.world_mut().resource_mut::<GroupRegistry>().leave(2);
    run_fixed_ticks(&mut app, 1);
    assert_not_visible(
        app.world().get::<NetworkVisibility>(target).unwrap(),
        connections[1].1,
        "departed member without movement",
    );
    {
        let mut registry = app.world_mut().resource_mut::<GroupRegistry>();
        registry.invite(1, "Member4", &online);
        registry.respond_invite(4, true);
    }
    run_fixed_ticks(&mut app, 1);
    assert_visible(
        app.world().get::<NetworkVisibility>(target).unwrap(),
        connections[3].1,
        "new group member without movement",
    );

    // The creator changing groups never changes this object's captured group GUID.
    app.world_mut().resource_mut::<GroupRegistry>().leave(1);
    {
        let mut registry = app.world_mut().resource_mut::<GroupRegistry>();
        registry.invite(1, "Member2", &online);
        registry.respond_invite(2, true);
        assert_ne!(registry.group_id(1), Some(group));
    }
    run_fixed_ticks(&mut app, 1);
    for (_, connection) in &connections[..2] {
        assert_not_visible(
            app.world().get::<NetworkVisibility>(target).unwrap(),
            *connection,
            "creator's new group cannot see old-group summon",
        );
    }
    for (_, connection) in &connections[2..] {
        assert_visible(
            app.world().get::<NetworkVisibility>(target).unwrap(),
            *connection,
            "old group keeps its summon",
        );
    }
    // Group membership is not permission to cross map copies or the ordinary interest radius.
    app.world_mut()
        .entity_mut(connections[2].0)
        .insert(pos(103.0, 0.0, 0.0));
    app.world()
        .resource::<InterestDirtyQueue>()
        .mark(connections[2].0);
    run_fixed_ticks(&mut app, 1);
    assert_not_visible(
        app.world().get::<NetworkVisibility>(target).unwrap(),
        connections[2].1,
        "beyond source interest radius",
    );
}
