//! TC a352b1fa instance_azure_vault.cpp / boss_leymor.cpp intro lifetime boundary.
use super::*;

const LEYMOR: u32 = 186644;
const ARCANE_TENDER: u32 = 191164;
const AZURE_MAP: u32 = 2515;
const FOLLOWER_DIFFICULTY: u32 = 205;
const STASIS: u32 = 375729;
const ARCANE_ERUPTION: u32 = 375749;
// TC a352b1fa UnitDefines.h176; production has no named NPC immunity bit yet.
const IMMUNE_TO_NPC: u32 = 0x0000_0200;

#[test]
fn azure_leymor_sourced_npc_immunity_rejects_npc_attack_admission() {
    let flags = shared::components::UnitFlags(IMMUNE_TO_NPC);
    assert!(
        !flags.can_be_attacked_by(false),
        "TC NPC immunity rejects NPC attackers"
    );
    assert!(
        flags.can_be_attacked_by(true),
        "NPC-only immunity does not imply PC immunity"
    );
    let all =
        shared::components::UnitFlags(IMMUNE_TO_NPC | shared::components::UnitFlags::IMMUNE_TO_PC);
    assert!(!all.can_be_attacked_by(false));
    assert!(!all.can_be_attacked_by(true));
}

fn eruption_count(h: &mut Harness, leymor: Entity) -> usize {
    h.app
        .world_mut()
        .query::<&crate::spell_cast::area_trigger::SpellAreaTrigger>()
        .iter(h.app.world())
        .filter(|field| field.caster == leymor && field.spell_id == ARCANE_ERUPTION)
        .count()
}

fn source_azure_copy() -> (Harness, Entity, InMap) {
    let (mut h, _, map) = encounter_fixture(
        LEYMOR,
        AZURE_MAP,
        0,
        FOLLOWER_DIFFICULTY,
        None,
        EncounterState::NotStarted,
        true,
    );
    h.advance(0.1);
    let leymor = h
        .creatures_of(LEYMOR)
        .into_iter()
        .find(|e| h.app.world().get::<InMap>(*e) == Some(&map))
        .unwrap();
    (h, leymor, map)
}

fn spawn_second_copy(h: &mut Harness) -> (Entity, InMap) {
    let maps = h.app.world().resource::<MapStore>().clone();
    let map = h
        .app
        .world_mut()
        .resource_mut::<crate::maps::InstanceManager>()
        .create_map(
            &maps,
            AZURE_MAP,
            crate::maps::instances::InstanceEntrant {
                character_id: 2002,
                leader: 2002,
                difficulty_id: FOLLOWER_DIFFICULTY,
                ..Default::default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 0,
            },
        )
        .unwrap()
        .map;
    let other = h.players[1].0;
    h.app.world_mut().entity_mut(other).insert(map);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    crate::zones::spawn_map_creatures(
        &mut commands,
        h.app.world().resource::<WorldData>(),
        crate::zones::MapContent {
            map,
            difficulty_id: FOLLOWER_DIFFICULTY,
            dead_bosses: Vec::new(),
        },
        false,
    );
    queue.apply(h.app.world_mut());
    h.advance(0.1);
    let leymor = h
        .creatures_of(LEYMOR)
        .into_iter()
        .find(|e| h.app.world().get::<InMap>(*e) == Some(&map))
        .unwrap();
    (leymor, map)
}

#[test]
fn azure_leymor_tender_depletion_does_not_wake_another_live_copy() {
    let (mut h, leymor, map) = source_azure_copy();
    let (other_leymor, other_map) = spawn_second_copy(&mut h);
    assert_ne!(map, other_map);
    assert!(h.has_aura(other_leymor, STASIS));
    let tenders: Vec<_> = h
        .creatures_of(ARCANE_TENDER)
        .into_iter()
        .filter(|e| h.app.world().get::<InMap>(*e) == Some(&map))
        .collect();
    assert_eq!(tenders.len(), 3);
    let player = h.player();
    for tender in tenders {
        crate::combat::kill_unit(h.app.world_mut(), player, tender);
    }
    h.advance(1.2);
    assert!(!h.has_aura(leymor, STASIS));
    assert!(
        h.has_aura(other_leymor, STASIS),
        "other copy's native group remains alive"
    );
    assert_eq!(eruption_count(&mut h, other_leymor), 0);
}

#[test]
fn azure_leymor_appears_in_native_stasis_until_tender_group_depletion() {
    let (h, leymor, _) = source_azure_copy();
    assert!(
        h.has_aura(leymor, STASIS),
        "authored intro starts in stasis"
    );
    let flags = h
        .app
        .world()
        .get::<shared::components::UnitFlags>(leymor)
        .unwrap();
    assert_ne!(flags.0 & shared::components::UnitFlags::IMMUNE_TO_PC, 0);
    assert_ne!(flags.0 & IMMUNE_TO_NPC, 0);
}

#[test]
fn azure_leymor_last_native_tender_death_wakes_after_one_second_and_stays_awake() {
    let (mut h, leymor, map) = source_azure_copy();
    let tenders: Vec<_> = h
        .creatures_of(ARCANE_TENDER)
        .into_iter()
        .filter(|e| h.app.world().get::<InMap>(*e) == Some(&map))
        .collect();
    assert_eq!(tenders.len(), 3, "three authenticated group members");
    assert!(h.has_aura(leymor, STASIS));
    let player = h.player();
    for tender in &tenders[..2] {
        crate::combat::kill_unit(h.app.world_mut(), player, *tender);
    }
    h.advance(1.2);
    assert!(
        h.has_aura(leymor, STASIS),
        "third member keeps intro locked"
    );
    crate::combat::kill_unit(h.app.world_mut(), player, tenders[2]);
    h.advance(0.8);
    assert!(
        h.has_aura(leymor, STASIS),
        "source wake delay has not elapsed"
    );
    h.advance(0.3);
    assert!(!h.has_aura(leymor, STASIS));
    let flags = h
        .app
        .world()
        .get::<shared::components::UnitFlags>(leymor)
        .unwrap();
    assert_eq!(
        flags.0 & (shared::components::UnitFlags::IMMUNE_TO_PC | IMMUNE_TO_NPC),
        0
    );
    assert_eq!(eruption_count(&mut h, leymor), 1);
    crate::scripted_ai::reinitialize(h.app.world_mut(), leymor);
    h.advance(0.1);
    assert!(
        !h.has_aura(leymor, STASIS),
        "copy intro flag survives actor reinitialization"
    );
    assert_eq!(eruption_count(&mut h, leymor), 1, "wake happens once");
}
