use super::*;

#[test]
fn toyfx28_group_private_use_captures_group_and_child_inherits_it() {
    use crate::group::{GroupRegistry, OnlineMember, OnlineView};
    use crate::interest::{PrivateObjectGroup, PrivateObjectOwner};
    use crate::spell_cast::summon::SummonedCreature;
    let directory = tempfile::tempdir().unwrap();
    let path = import_toyfx8_world(directory.path());
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("inherit.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    h.app
        .insert_resource(crate::world_data::load_world_data(&path));
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(character));
    let online = OnlineView::from_iter(
        [(character, "Caster"), (character + 1, "Friend")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id,
                    OnlineMember {
                        entity: h.caster.to_bits(),
                        name: name.into(),
                        class: 8,
                        level: 80,
                        portrait: default(),
                        dungeon_difficulty: 1,
                        raid_difficulty: default(),
                    },
                )
            }),
    );
    let group = {
        let mut registry = h.app.world_mut().resource_mut::<GroupRegistry>();
        registry.invite(character, "Friend", &online);
        registry.respond_invite(character + 1, true);
        registry.group_id(character).unwrap()
    };
    super::group_private::learn_group_toy(&mut h, character, 112324);
    assert_eq!(activate_at(&mut h, 112324, None)[0].error, None);
    h.advance(40);
    let world = h.app.world_mut();
    let actor = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .find(|(_, summon)| summon.spell_id == 162979)
        .unwrap()
        .0;
    assert_eq!(world.get::<PrivateObjectGroup>(actor).unwrap().0, group);
    assert!(world.get::<PrivateObjectOwner>(actor).is_none());
    // An owner-only source child still inherits an already-private caster's group GUID.
    crate::spell_cast::cast_triggered(world, actor, 304505, Some(actor), 0);
    let child = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .find(|(_, summon)| summon.spell_id == 304505)
        .unwrap()
        .0;
    assert_eq!(world.get::<PrivateObjectGroup>(child).unwrap().0, group);
    assert!(world.get::<PrivateObjectOwner>(child).is_none());
}
