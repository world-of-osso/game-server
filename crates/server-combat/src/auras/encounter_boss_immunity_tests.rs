//! Encounter bosses reject retail-immune mechanics and still take snares.
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::*;

const HALLS_OF_INFUSION: u32 = 2527;
const KHAJIN: u32 = 189727;
/// Protection Warrior Shield Charge child: damage effects 0/1, stun effect 2 (mechanic 12).
const SHIELD_CHARGE: u32 = 385954;
/// Hamstring: snare effect 0 (aura 33, mechanic 11).
const HAMSTRING: u32 = 1715;
/// Melee spell hits can miss, dodge or block; ten attempts make a total miss negligible.
const ATTEMPTS: usize = 10;
const FULL_HEALTH: f32 = 1_000_000.0;

fn has_aura(h: &Harness, spell: u32) -> bool {
    h.app
        .world()
        .get::<ActiveAuras>(h.npc)
        .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == spell))
}

/// A warrior and a target of creature `entry`, both inside Halls of Infusion.
fn warrior_against(entry: u32) -> Harness {
    let mut h =
        harness(Vec::new(), &[SHIELD_CHARGE, HAMSTRING], 1).expect("real world.db required");
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::maps::store::load_map_store(&conn).unwrap());
    let map = crate::maps::InMap {
        map_id: HALLS_OF_INFUSION,
        instance_id: 1,
    };
    h.app.world_mut().entity_mut(h.caster).insert(map);
    h.app.world_mut().entity_mut(h.npc).insert((
        map,
        shared::components::Npc {
            template_id: entry,
            name: String::new(),
        },
        shared::components::Health {
            current: FULL_HEALTH,
            max: FULL_HEALTH,
        },
    ));
    h
}

/// Casts `spell` at the target until `landed` holds; returns whether it did.
fn cast_until(h: &mut Harness, spell: u32, landed: impl Fn(&Harness) -> bool) -> bool {
    let (caster, npc) = (h.caster, h.npc);
    (0..ATTEMPTS).any(|_| {
        crate::spell_cast::cast_triggered(h.app.world_mut(), caster, spell, Some(npc), 1);
        h.advance(2);
        landed(h)
    })
}

#[test]
fn encounter_boss_takes_shield_charge_damage_without_its_stun() {
    let mut h = warrior_against(KHAJIN);
    assert!(
        h.app
            .world()
            .resource::<crate::maps::MapStore>()
            .raid_creature_encounters
            .contains_key(&(HALLS_OF_INFUSION, KHAJIN)),
        "Khajin is bound to his DungeonEncounter"
    );
    assert!(cast_until(&mut h, SHIELD_CHARGE, |h| h.health(h.npc) < FULL_HEALTH));
    assert!(!has_aura(&h, SHIELD_CHARGE), "boss rejects the stun");
    assert!(
        cast_until(&mut h, HAMSTRING, |h| has_aura(h, HAMSTRING)),
        "boss still takes snares"
    );
}

#[test]
fn unbound_creature_in_the_same_dungeon_is_stunned_by_shield_charge() {
    let mut h = warrior_against(6);
    assert!(cast_until(&mut h, SHIELD_CHARGE, |h| has_aura(
        h,
        SHIELD_CHARGE
    )));
}
