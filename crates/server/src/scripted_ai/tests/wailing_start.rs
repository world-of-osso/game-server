//! Controlled source callbacks: real flags/menu/spells remain unmodified.
use super::*;
use shared::protocol::{
    InteractNpc, InteractionChannel, InteractionError, InteractionKind, NpcFlags,
};

fn start_fixture() -> (Harness, Entity, Entity, InMap) {
    let (mut h, prerequisite, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app
        .insert_resource(crate::npc_interaction::load_gossip_catalog(&conn).unwrap());
    // Real prerequisite death, not a passive hostile retaining an old chase.
    h.hit_pct(prerequisite, 1_000.0);
    h.advance(0.1);
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (other_disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, other_map, 1);
    for (index, actor) in [(0, disciple), (1, other_disciple)] {
        let position = *h.app.world().get::<Position>(actor).unwrap();
        h.app
            .world_mut()
            .entity_mut(h.players[index].0)
            .insert(position);
    }
    h.advance(0.1);
    assert_eq!(h.app.world().get::<NpcFlags>(disciple).unwrap().0, 0);
    assert!(h.app.world().resource::<SpellStore>().get(5232).is_none());
    (h, disciple, other_disciple, map)
}

fn finish_druid(h: &mut Harness, map: InMap, encounter: u32) {
    let slot = instance_script::boss_id_for_encounter(h.app.world_mut(), map, encounter).unwrap();
    assert!(instance_script::set_boss_state(
        h.app.world_mut(),
        map,
        slot,
        EncounterState::Done
    ));
}

#[test]
fn encounter_coverage_wailing_controlled_gossip_hello_uses_four_done_and_real_empty_menu() {
    let (mut h, disciple, other_disciple, map) = start_fixture();
    let player = h.player();
    let other_player = h.players[1].0;
    let connection = h.players[0].1;
    h.app
        .world_mut()
        .get_mut::<MessageSender<InteractNpc>>(connection)
        .unwrap()
        .send::<InteractionChannel>(InteractNpc {
            npc: disciple.to_bits(),
        });
    h.advance(0.1);
    assert!(
        h.inbox(0).opened.is_empty(),
        "raw npcflag0 blocks natural interaction"
    );
    assert!(
        h.inbox(0)
            .failed
            .iter()
            .any(|failure| failure.error == InteractionError::InvalidTarget)
    );
    assert!(!creature_gossip_hello(h.app.world_mut(), player, disciple));
    finish_druid(&mut h, map, 588);
    finish_druid(&mut h, map, 585);
    assert!(
        !creature_gossip_hello(h.app.world_mut(), player, disciple),
        "three druids insufficient"
    );
    finish_druid(&mut h, map, 590);
    assert!(
        !creature_gossip_hello(h.app.world_mut(), other_player, other_disciple),
        "other copy stays unready"
    );
    // Direct source callback only: no synthetic GOSSIP flag or option row.
    assert!(creature_gossip_hello(h.app.world_mut(), player, disciple));
    h.advance(0.1);
    let opened = h
        .inbox(0)
        .opened
        .last()
        .expect("source prepared gossip menu");
    assert_eq!(opened.npc, disciple.to_bits());
    let InteractionKind::Gossip(menu) = &opened.kind else {
        panic!("expected source gossip")
    };
    assert_eq!(menu.menu_id, 202);
    assert!(
        menu.options.is_empty(),
        "current normalized source options are absent, not fabricated"
    );
    assert!(h.inbox(1).opened.is_empty());
    assert!(
        !h.has_aura(player, 5232),
        "missing source spell cannot become a fabricated buff"
    );
    assert_eq!(h.app.world().get::<NpcFlags>(disciple).unwrap().0, 0);
}

#[test]
fn encounter_coverage_wailing_controlled_select_closes_then_starts_authored_first_path() {
    let (mut h, disciple, _, map) = start_fixture();
    let root = std::path::PathBuf::from(
        std::env::var_os("WAILING_NAV_ROOT").expect("private source map43 navigation"),
    );
    h.app
        .insert_resource(crate::navigation::Navigation::new(&root));
    for encounter in [588, 585, 590] {
        finish_druid(&mut h, map, encounter);
    }
    let player = h.player();
    assert!(creature_gossip_hello(h.app.world_mut(), player, disciple));
    h.advance(0.1);
    assert!(
        h.app
            .world()
            .get::<crate::npc_interaction::ActiveInteraction>(h.players[0].1)
            .is_some()
    );
    let origin = *h.app.world().get::<Position>(disciple).unwrap();
    let speech = h.app.world().resource::<CreatureTexts>().groups[&3678][&1][0]
        .male_text
        .clone();
    assert!(creature_gossip_select(h.app.world_mut(), player, disciple));
    h.advance(0.1);
    assert_eq!(h.inbox(0).closed.last().unwrap().npc, disciple.to_bits());
    assert!(h.inbox(1).closed.is_empty());
    assert!(
        h.app
            .world()
            .get::<crate::npc_interaction::ActiveInteraction>(h.players[0].1)
            .is_none()
    );
    assert_eq!(
        h.app
            .world()
            .get::<UnitFactionTemplate>(disciple)
            .unwrap()
            .0,
        250
    );
    assert_eq!(h.app.world().get::<NpcFlags>(disciple).unwrap().0, 0);
    dispatch(h.app.world_mut(), disciple, Hook::Reset);
    h.advance(9.7);
    assert!(
        !h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content == speech)
    );
    // Controlled combat callback clock, not an ordinary-fight claim.
    h.app
        .world_mut()
        .entity_mut(disciple)
        .insert(CombatStatus(true));
    dispatch(h.app.world_mut(), disciple, Hook::UpdateAi(30_000));
    h.app
        .world_mut()
        .entity_mut(disciple)
        .insert(CombatStatus(false));
    h.advance(0.1);
    assert!(
        !h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content == speech)
    );
    h.advance(0.3);
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content == speech)
    );
    assert!(
        !h.inbox(1)
            .chat
            .iter()
            .any(|message| message.content == speech)
    );
    // Source +3.5s MovePath, followed by TC's existing 1s initialization wait.
    h.advance(4.1);
    let waiting = h.app.world().get::<Position>(disciple).unwrap();
    assert!((waiting.x - origin.x).hypot(waiting.z - origin.z) < 0.001);
    h.advance(2.0);
    let moving = *h.app.world().get::<Position>(disciple).unwrap();
    assert!(
        (moving.x - origin.x).hypot(moving.z - origin.z) > 1.0,
        "actual first source-path movement, no arrival injection"
    );
    assert!(!h.has_aura(player, 5232));
}
