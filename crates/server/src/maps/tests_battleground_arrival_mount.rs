use super::*;
use shared::components::{Mounted, PlayerMotion};
use shared::profession::ProfessionSkillLine;

#[path = "tests_battleground_exit_mount.rs"]
mod exit_mount;

const GOLDEN_GRYPHON: u32 = 32235;
const STANDARD_GROUND_SPEED: u32 = 86457;
const UNRELATED_AURA: u32 = DUNGEON_DESERTER_SPELL_ID;

#[test]
fn bg_arrival_mount_concrete_dismounts_only_after_world_port_ack() {
    assert_arrival_dismounts(1018);
}

#[test]
fn bg_arrival_mount_random_dismounts_only_after_world_port_ack() {
    assert_arrival_dismounts(32);
}

fn mount_source_rider(h: &mut Harness, who: &Traveler) {
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    let spells = [
        DESERTER_SPELL_ID,
        UNRELATED_AURA,
        GOLDEN_GRYPHON,
        STANDARD_GROUND_SPEED,
    ]
    .into_iter()
    .map(|id| data.store.get(id).unwrap().clone())
    .collect();
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(spells));
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::auras::mount::load_mount_store(&db).unwrap());
    h.app.world_mut().entity_mut(who.player).insert((
        PlayerMotion::default(),
        crate::profession::PlayerProfessions {
            lines: vec![ProfessionSkillLine {
                skill_line: crate::auras::mount::SKILL_RIDING,
                step: 1,
                rank: 75,
                max_rank: 75,
            }],
            spells: Vec::new(),
        },
    ));
    apply_sourced_aura(h, who, UNRELATED_AURA);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(GOLDEN_GRYPHON)
        .unwrap();
    let mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: who.player,
            target: who.player,
            spell_id: GOLDEN_GRYPHON,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
    assert_source_mounted(h, who);
}

fn assert_source_mounted(h: &Harness, who: &Traveler) {
    let auras = h.app.world().get::<ActiveAuras>(who.player).unwrap();
    assert!(auras.has_spell(GOLDEN_GRYPHON));
    assert!(auras.has_type(crate::auras::handlers::MOUNTED));
    assert!(
        auras.has_spell(STANDARD_GROUND_SPEED),
        "authored riding75 capability speed aura"
    );
    assert!(auras.has_spell(UNRELATED_AURA));
    let mounted = h
        .app
        .world()
        .get::<Mounted>(who.player)
        .expect("real mount callback creates display projection");
    assert_eq!(mounted.mount_id, 129);
    assert_eq!(mounted.mount_display_id, 17697);
}

fn assert_arrival_dismounts(queue_id: u32) {
    let (mut h, players, _database) = pending_battleground(queue_id);
    let who = &players[0];
    mount_source_rider(&mut h, who);
    let invitation = battleground(&h, who).clone();
    let unrelated = h
        .app
        .world()
        .get::<ActiveAuras>(who.player)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == UNRELATED_AURA)
        .unwrap()
        .instance_id;
    let updates = request_port(&mut h, who, invitation.match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "successful port initiation: {updates:?}"
    );
    let destinations = h.new_worlds(who);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].map_id, invitation.map_id);
    assert!(h.map_of(who).is_none());
    assert_source_mounted(&h, who);
    h.ack(who);
    assert_eq!(h.map_of(who).unwrap().map_id, invitation.map_id);
    assert!(battleground(&h, who).accepted);
    let auras = h.app.world().get::<ActiveAuras>(who.player).unwrap();
    assert!(
        !auras.has_type(crate::auras::handlers::MOUNTED),
        "actual ordinary BG arrival must remove mount78 through normal cleanup"
    );
    assert!(!auras.has_spell(GOLDEN_GRYPHON));
    assert!(!auras.has_spell(STANDARD_GROUND_SPEED));
    assert!(h.app.world().get::<Mounted>(who.player).is_none());
    assert_eq!(
        auras
            .iter()
            .find(|aura| aura.spell_id == UNRELATED_AURA)
            .unwrap()
            .instance_id,
        unrelated
    );
}

#[test]
fn bg_arrival_mount_queue_without_match_retains_mount() {
    let (mut h, players, _database) = deserter_harness(1);
    let who = &players[0];
    mount_source_rider(&mut h, who);
    let updates = request_queue(&mut h, who, 1018);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "queue: {updates:?}"
    );
    assert_queued(&h, &players, 1018);
    assert!(h.new_worlds(who).is_empty());
    assert_source_mounted(&h, who);
}

#[test]
fn bg_arrival_mount_declined_invitation_retains_mount() {
    let (mut h, players, _database) = pending_battleground(1018);
    let who = &players[0];
    mount_source_rider(&mut h, who);
    let invitation = battleground(&h, who).clone();
    h.app
        .world_mut()
        .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(RespondToBattlegroundInvite {
            match_id: invitation.match_id,
            accepted: false,
        });
    let updates = h.receive_pvp_updates(5);
    assert!(
        updates
            .values()
            .flatten()
            .all(|update| update.error.is_none()),
        "decline: {updates:?}"
    );
    assert!(h.new_worlds(who).is_empty());
    assert_not_queued(&h, std::slice::from_ref(who));
    assert_source_mounted(&h, who);
}

#[test]
fn bg_arrival_mount_refused_entry_retains_mount() {
    let (mut h, players, _database) = pending_battleground(1018);
    let who = &players[0];
    mount_source_rider(&mut h, who);
    let invitation = battleground(&h, who).clone();
    h.app
        .world_mut()
        .remove_resource::<crate::game_objects::GameObjectSpawns>();
    let updates = request_port(&mut h, who, invitation.match_id);
    assert!(
        updates
            .iter()
            .any(|update| update.error.as_deref() == Some("gameobject catalog unavailable")),
        "refused native entry: {updates:?}"
    );
    assert!(h.new_worlds(who).is_empty());
    assert_eq!(h.map_of(who), Some(STORMWIND));
    assert!(!battleground(&h, who).accepted);
    assert_source_mounted(&h, who);
}

#[test]
fn bg_arrival_mount_ordinary_dungeon_transfer_retains_mount() {
    let (mut h, players, _database) = deserter_harness(1);
    let who = &players[0];
    mount_source_rider(&mut h, who);
    let destination = h.enter_stockade(who);
    assert_eq!(destination.map_id, 34);
    assert_eq!(h.map_of(who).unwrap().map_id, 34);
    assert_source_mounted(&h, who);
}
