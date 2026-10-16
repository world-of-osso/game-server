//! TC Create initializes area before SelectLevel; TempSummon clamps every unflagged summon.
use super::*;

#[test]
fn toyfx55_wild_real_use_clamps_podium_to_region() {
    assert_wild_source_levels(239018, 1227063, 240220, &[(20, 20), (80, 30)]);
}

#[test]
fn toyfx55_wild_use_creature_level_keeps_region_selected_shrine_level() {
    assert_wild_source_levels(245942, 1240082, 246037, &[(20, 30), (80, 30)]);
}

fn assert_wild_source_levels(item: u32, spell: u32, entry: u32, cases: &[(u8, u8)]) {
    use crate::level_scaling::CreatureArmor;
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Health, Npc, UnitLevel};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let mut observed = Vec::new();
    for &(owner_level, expected) in cases {
        let (mut h, character) = setup(
            &directory
                .path()
                .join(format!("wild-{item}-{owner_level}.redb")),
        );
        place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert((Level(owner_level), UnitLevel(owner_level)));
        h.app.insert_resource(toy_catalog::load(&conn).unwrap());
        h.app.insert_resource(shared_item_templates(&path));
        h.app.insert_resource(shared_world_data(&path));
        h.app
            .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
        h.app
            .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
        learn_effect_toy(&mut h, character, item);
        assert_eq!(h.app.world().get::<Level>(h.caster).unwrap().0, owner_level);
        assert_eq!(
            h.app.world().get::<UnitLevel>(h.caster).unwrap().0,
            owner_level
        );
        let origin = *h.app.world().get::<Position>(h.caster).unwrap();
        let destination = (item == 245942).then_some([origin.x + 3.0, origin.y, origin.z]);
        assert_eq!(activate_at(&mut h, item, destination)[0].error, None);
        h.advance(40);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let world = h.app.world_mut();
        let levels: Vec<_> = world
            .query::<(&Npc, &SummonedCreature, &UnitLevel, &Health, &CreatureArmor)>()
            .iter(world)
            .filter(|(npc, summon, _, _, _)| npc.template_id == entry && summon.spell_id == spell)
            .map(|(_, _, level, health, armor)| (level.0, health.max, armor.0))
            .collect();
        assert_eq!(
            levels.len(),
            1,
            "one source actor from actual learned UseToy"
        );
        // Installed ExpectedStat IDs20/30 (ExpansionID -2), class mod4=1,
        // no ContentTuning73 mods; Podium HealthModifier20, Shrine1.
        let expected_health = match (item, expected) {
            (239018, 20) => 48066.0,
            (239018, 30) => 87576.0,
            (245942, 30) => 4379.0,
            _ => panic!("missing concrete source stat expectation"),
        };
        let expected_armor = if expected == 20 { 135.0 } else { 155.0 };
        observed.push((
            owner_level,
            levels[0],
            (expected, expected_health, expected_armor),
        ));
    }
    assert!(
        observed
            .iter()
            .all(|(_, level, expected)| level == expected),
        "item{item}: (owner, actual(level,HP,armor), source expected) {observed:?}"
    );
}
