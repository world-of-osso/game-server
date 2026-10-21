//! TC core Coyote removal uses source-owned target conditions, not any nearby summon.
use super::*;
use crate::spell_cast::summon::SummonedCreature;

fn install_coyote_sources(h: &mut Harness) {
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    h.app
        .insert_resource(crate::conditions::load_spell_target_conditions(&path).unwrap());
}

fn summon_source_coyote(h: &mut Harness, owner: Entity) -> Entity {
    crate::spell_cast::cast_triggered(h.app.world_mut(), owner, 51149, None, 0);
    h.advance(2);
    let world = h.app.world_mut();
    world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == 51149)
        .map(|(entity, _)| entity)
        .expect("actual source51149 Coyote")
}

#[test]
fn toy_coyote_owned_condition_selects_only_source_owner_not_other_summoner() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("coyote-conditions.redb"));
    install_coyote_sources(&mut h);
    let owner = h.caster;
    let other = h.npc;
    let own = summon_source_coyote(&mut h, owner);
    let other_owned = summon_source_coyote(&mut h, other);
    // Ordinary TempSummon's summoner does not establish source GetOwnerGUID.
    let unowned = crate::zones::summon_creature(h.app.world_mut(), owner, 28267, [0.0; 4]).unwrap();
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(52173)
        .unwrap()
        .clone();
    let selected = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        owner,
        None,
        &spell,
        &spell.effects[0],
        0,
        false,
    )
    .unwrap();
    assert_eq!(selected, vec![own]);
    assert!(!selected.contains(&other_owned));
    assert!(!selected.contains(&unowned));
    // Same entry and owner in another world copy remains outside the source search.
    h.app
        .world_mut()
        .entity_mut(own)
        .insert(crate::maps::InMap {
            map_id: 0,
            instance_id: 42,
        });
    let selected = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        owner,
        None,
        &spell,
        &spell.effects[0],
        0,
        false,
    )
    .unwrap();
    assert!(selected.is_empty());
}
