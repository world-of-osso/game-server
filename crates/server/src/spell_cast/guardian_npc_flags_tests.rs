//! Ordinary NPC ownership remains TC-literal under the class-follower minion policy.
use super::test_harness::*;
use super::*;

#[test]
fn guardians_ordinary_npc_owner_keeps_uncontrolled_imp_and_hostile_casts() {
    let mut h = harness(Vec::new(), &[104317, 104318], 9).unwrap();
    let data = crate::world_data::load_world_data(&world_db().unwrap().powers_db);
    let owner = h.npc;
    let entry = h
        .app
        .world()
        .get::<shared::components::Npc>(owner)
        .unwrap()
        .template_id;
    let native_level = data.templates[&entry].min_level;
    h.app.insert_resource(data);
    h.app.init_resource::<ObservedCombatEvents>();
    h.app.world_mut().entity_mut(owner).insert((
        shared::components::UnitLevel(native_level),
        crate::networking::CombatTarget(Some(h.caster)),
    ));
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        owner,
        104317,
        Some(owner),
        0
    ));
    let world = h.app.world_mut();
    let imps: Vec<_> = world
        .query::<(Entity, &super::summon::SummonedCreature)>()
        .iter(world)
        .filter(|(_, summon)| summon.owner == owner && summon.spell_id == 104317)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(imps.len(), 1, "real native104317 must spawn one Wild Imp");
    let imp = imps[0];
    let flags = world
        .get::<shared::components::UnitFlags>(imp)
        .map_or(0, |flags| flags.0);
    assert_eq!(flags & crate::faction::UNIT_FLAG_PLAYER_CONTROLLED, 0);
    assert!(super::targets::relation_matches(
        world,
        imp,
        h.caster,
        super::targets::Relation::Enemy
    ));
    h.advance(120);
    assert!(
        h.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(imp.to_bits())
                && event.spell_id == Some(104318)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess),
        "uncontrolled NPC-owned Imp must still cast against its ordinary hostile target"
    );
}
