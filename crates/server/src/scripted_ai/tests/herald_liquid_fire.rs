//! Ground-only Liquid Fire native inputs. No flying-family coverage claim.
use super::*;

const CHAINS: &[(u32, u32, i32, u32)] = &[
    (31706, 181890, 15_000, 30927),
    (30928, 182533, 30_000, 32492),
    (23971, 180125, 20_000, 23972),
];

#[test]
fn encounter_coverage_herald_liquid_fire_source_qualification() {
    let (_directory, path) = prepare_encounter_database("herald");
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let store = &crate::spell_cast::test_harness::world_db().unwrap().store;
    for &(child, object, duration, pulse) in CHAINS {
        let spell = store.get(child).expect("native hit-unit child");
        assert_eq!(spell.effects.len(), 1);
        let effect = &spell.effects[0];
        assert_eq!((effect.index, effect.effect), (0, 76));
        assert_eq!(effect.misc_values[0], object as i32);
        assert_eq!(effect.implicit_targets, [18, 0]);
        assert_eq!(
            spell.misc.as_ref().unwrap().duration.unwrap().duration_ms,
            duration
        );
        let source: (u32, u32, u32, String, String) = conn.query_row(
            "SELECT type,displayId,Data3,AIName,ScriptName FROM tdb_gameobject_template WHERE entry=?1",
            [object], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
        ).unwrap();
        assert_eq!(
            source,
            (
                6,
                4251,
                pulse,
                String::new(),
                "go_vazruden_liquid_fire".into()
            )
        );
        let damage = store.get(pulse).expect("native ordinary trigger damage");
        assert_eq!(damage.effects.len(), 1);
        assert_eq!(damage.effects[0].effect, 2);
        assert_eq!(damage.effects[0].implicit_targets, [22, 15]);
        println!(
            "native child{} object{} duration{} damage{}: {}",
            child,
            object,
            duration,
            pulse,
            serde_json::to_string(damage).unwrap()
        );
    }
    let data = crate::world_data::load_world_data(std::path::Path::new(conn.path().unwrap()));
    for difficulty in [1, 2] {
        let template = data
            .template_for(21252, difficulty)
            .expect("source trigger difficulty");
        assert_eq!(template.entry, 21252);
        println!(
            "native21252 difficulty{difficulty}: name={} faction={} type={}",
            template.name, template.faction, template.creature_type
        );
    }
}
