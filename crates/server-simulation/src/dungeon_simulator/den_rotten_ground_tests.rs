use super::*;
use crate::spell_cast::area_trigger::SpellAreaTrigger;

#[test]
#[ignore = "requires Den graph, world data and native ground assets"]
fn den_keystone_rotten_supplies_creates_and_expires_ground() {
    let options = super::super::keystone::Options {
        map_id: 2825,
        level: 2,
        specs: [66, 105, 262, 63, 253],
    };
    let mut run = spawn_run_mode(1, &mut None, Some(options)).unwrap();
    let guid = run
        .graph
        .packs
        .iter()
        .flat_map(|pack| &pack.spawns)
        .find(|spawn| spawn.entry == 241813)
        .unwrap()
        .guid;
    let gatherer = run.actors[&guid];
    let target = run.party.owner;
    let position = *run.app.world().get::<Position>(gatherer).unwrap();
    run.app.world_mut().entity_mut(target).insert(position);
    crate::spell_cast::cast_triggered(run.app.world_mut(), gatherer, 1297699, Some(target), 0);
    for _ in 0..30 {
        run.app.update();
    }
    let world = run.app.world_mut();
    let fields: Vec<_> = world
        .query::<(Entity, &SpellAreaTrigger)>()
        .iter(world)
        .filter(|(_, field)| field.spell_id == 1297700)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(fields.len(), 1, "Rotten Supplies impact creates a field");
    assert!(
        crate::auras::has_aura(world, target, 1297701),
        "impact target enters the field"
    );
    for _ in 0..420 {
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(fields[0]).is_err(),
        "field expires"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), target, 1297701),
        "field aura expires"
    );
}
