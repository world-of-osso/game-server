//! TC a352b1fa Blood Furnace Keli'dan the Breaker17377 (boss_kelidan_the_breaker.cpp).
use super::*;

const KELIDAN: u32 = 17377;
const CHANNELER: u32 = 17653;
const BLOOD_FURNACE: u32 = 542;
/// TC DATA_KELIDAN: installed DB2 encounter1923, completion bit1.
const KELIDAN_BIT: u32 = 1;
const CHANNELER_GROUPS: [u32; 5] = [360, 361, 362, 363, 364];
const EVOCATION: u32 = 30935;
const SHADOW_BOLT_VOLLEY: u32 = 17228;
const SHADOW_BOLT_VOLLEY_H: u32 = 40070;
const CORRUPTION: u32 = 30938;
const BURNING_NOVA: u32 = 30940;
const STAR_BEAM: u32 = 30888;
const SHADOW_BOLT: u32 = 12739;
const SHADOW_BOLT_H: u32 = 15472;
const MARK_OF_SHADOW: u32 = 30937;
const VORTEX: u32 = 37370;
const FIRE_NOVA: u32 = 33775;
const FIRE_NOVA_H: u32 = 37371;

/// A Blood Furnace copy filled by production copy fill from the real map542 rows,
/// with authentic ground and eye tables.
fn kelidan_copy(difficulty: u32) -> (Harness, InMap) {
    let (mut h, _, map) = encounter_fixture(
        KELIDAN,
        BLOOD_FURNACE,
        KELIDAN_BIT,
        difficulty,
        None,
        EncounterState::NotStarted,
        true,
    );
    let directory = crate::ground::ground_data_dir();
    let wdt = h.app.world().resource::<MapStore>().maps[&BLOOD_FURNACE].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &directory,
        std::collections::HashMap::from([(BLOOD_FURNACE, wdt)]),
    ));
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&directory.join("los"))
            .expect("authentic eye tables"),
    );
    (h, map)
}

/// The first player sees `actors` (chat, SpellGo and combat log delivery).
fn watch(h: &mut Harness, actors: &[Entity]) {
    let connection = h.players[0].1;
    for &actor in actors {
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(connection);
        h.app.world_mut().entity_mut(actor).insert(visibility);
    }
}

fn spawn_string_id(h: &Harness, actor: Entity) -> Option<String> {
    h.app
        .world()
        .get::<crate::zones::CreatureStringIds>(actor)
        .and_then(|ids| ids.spawn.clone())
}

/// The copy's channeler carrying spawn StringId `KelidansShadowmoonChannelerN`.
fn channeler(h: &mut Harness, n: u32) -> Entity {
    let wanted = format!("KelidansShadowmoonChanneler{n}");
    let channelers = h.creatures_of(CHANNELER);
    channelers
        .into_iter()
        .find(|&actor| spawn_string_id(h, actor).as_deref() == Some(wanted.as_str()))
        .unwrap_or_else(|| panic!("copy has {wanted}"))
}

#[test]
fn encounter_coverage_kelidan_native_inputs_qualify() {
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let groups =
        crate::world_data::load_script_spawn_groups(&conn, &CHANNELER_GROUPS, true).unwrap();
    let strings = crate::world_data::load_spawn_string_ids(&conn).unwrap();
    for (n, group) in CHANNELER_GROUPS.iter().enumerate() {
        let guids = &groups[group];
        assert_eq!(guids.len(), 1, "manual group{group} holds one channeler");
        assert_eq!(
            strings.get(&guids[0]).map(String::as_str),
            Some(format!("KelidansShadowmoonChanneler{}", n + 1).as_str()),
            "group{group} member StringId"
        );
    }
    for difficulty in [1, 2] {
        let (mut h, map) = kelidan_copy(difficulty);
        assert_eq!(
            h.app
                .world()
                .resource::<MapStore>()
                .instance_scripts
                .get(&BLOOD_FURNACE)
                .map(String::as_str),
            Some("instance_blood_furnace")
        );
        let bosses = h.creatures_of(KELIDAN);
        assert_eq!(
            bosses.len(),
            1,
            "TDB spawnDifficulties1,2 in difficulty{difficulty}"
        );
        let boss = bosses[0];
        let channelers = h.creatures_of(CHANNELER);
        eprintln!(
            "KELIDAN difficulty{difficulty} copy-fill channelers={} ids={:?}",
            channelers.len(),
            channelers
                .iter()
                .map(|&actor| spawn_string_id(&h, actor))
                .collect::<Vec<_>>()
        );
        let texts = &h.app.world().resource::<CreatureTexts>().groups;
        let counts: Vec<_> = (0..=3).map(|group| texts[&KELIDAN][&group].len()).collect();
        assert_eq!(counts, [1, 1, 2, 1], "native wake/nova/slay/death groups");
        assert!(
            texts[&KELIDAN]
                .values()
                .flatten()
                .all(|text| text.chat_type == 14),
            "Keli'dan MonsterYell"
        );
        assert_eq!(texts[&CHANNELER][&0].len(), 3, "channeler aggro yells");
        assert_eq!(texts[&CHANNELER][&1].len(), 7, "channeler aggro says");
        let spells = h.app.world().resource::<SpellStore>();
        assert!(
            spells.get(SHADOW_BOLT_VOLLEY_H).is_none(),
            "retail70448 has no Heroic volley40070"
        );
        for spell in [
            EVOCATION,
            SHADOW_BOLT_VOLLEY,
            CORRUPTION,
            BURNING_NOVA,
            STAR_BEAM,
            SHADOW_BOLT,
            SHADOW_BOLT_H,
            MARK_OF_SHADOW,
            VORTEX,
            FIRE_NOVA,
            FIRE_NOVA_H,
        ] {
            assert!(spells.get(spell).is_some(), "spell{spell}");
        }
        // HARNESS SETUP: direct threat, not the source channeler release.
        h.engage(boss);
        h.advance(0.5);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, KELIDAN_BIT),
            Some(EncounterState::InProgress)
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 1923 && start.difficulty_id == difficulty),
            "bit1 -> DungeonEncounter1923 in difficulty{difficulty}"
        );
        assert!(h.inbox(1).starts.is_empty(), "other copy isolated");
    }
}

#[test]
fn encounter_coverage_kelidan_native_spell_admissions_qualify() {
    for difficulty in [1, 2] {
        let (mut h, _) = kelidan_copy(difficulty);
        let boss = h.creatures_of(KELIDAN)[0];
        let first = channeler(&mut h, 1);
        let partner = channeler(&mut h, 4);
        watch(&mut h, &[boss, first, partner]);
        let world = h.app.world_mut();
        crate::spell_cast::creature_cast(world, boss, EVOCATION, None, false)
            .expect("ordinary self Evocation");
        crate::spell_cast::creature_cast(world, first, STAR_BEAM, Some(partner), false)
            .expect("ordinary channeler1 -> channeler4 Star Beam");
        h.advance(0.5);
        assert!(h.has_aura(boss, EVOCATION));
        assert!(h.has_aura(partner, STAR_BEAM));

        crate::spell_cast::creature_cast(h.app.world_mut(), boss, BURNING_NOVA, None, false)
            .expect("ordinary self Burning Nova");
        h.advance(0.5);
        assert!(h.has_aura(boss, BURNING_NOVA), "5s self stun aura");
        h.advance(5.0);
        assert!(!h.has_aura(boss, BURNING_NOVA), "stun expires");

        let nova = if difficulty == 2 {
            FIRE_NOVA_H
        } else {
            FIRE_NOVA
        };
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, nova, None, false)
            .expect("ordinary Fire Nova");
        if difficulty == 2 {
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, VORTEX, None, true)
                .expect("triggered Heroic Vortex");
        }
        h.advance(5.5);
        eprintln!(
            "KELIDAN difficulty{difficulty} nova casts={} vortex casts={}",
            h.casts(0, boss, nova),
            h.casts(0, boss, VORTEX)
        );
        assert_eq!(h.casts(0, boss, nova), 1, "Fire Nova completes");
    }
}
